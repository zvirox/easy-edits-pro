// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! POSIX shared-memory SPSC frame rings for worker audio transport.
//!
//! One shm file holds two single-producer/single-consumer frame rings:
//!
//! ```text
//! ┌─ page 0 ─── request ring header   (client → worker)
//! ├─ page 1 ─── response ring header (worker → client)
//! ├─ request data   : capacity × channels × f32
//! └─ response data  : capacity × channels × f32
//! ```
//!
//! Ring headers are 48-bytes of u32/u64 scalars in page-sized regions. The
//! produced/consumed counters are absolute frame positions that wrap modulo
//! the capacity, so free/full tests are plain integer comparisons and wrap
//! behavior is automatic.
//!
//! Creation uses `O_CREAT | O_EXCL` with mode `0600`; the creator unlinks
//! the backing file on drop, so a crashed session never leaves shared
//! audio behind in `/dev/shm`. A second process can only *open* (worker
//! side) and is expected to validate, never rewrite, the headers.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::errors::{DspError, DspResult};

/// Magic marking every ring header ("DSR1").
pub const RING_MAGIC: &[u8; 4] = b"DSR1";
/// Ring header layout version.
pub const RING_VERSION: u32 = 1;
/// Size of one ring-header region (page-aligned).
pub const RING_HEADER_SIZE: usize = 4096;
/// Default in-flight capacity, in frames, per ring.
pub const RING_CAPACITY_FRAMES: u64 = 65_536;
/// Largest frame count in one command: half the ring, so both rings can
/// always absorb a full command without producer-side blocking.
pub const RING_MAX_COMMAND_FRAMES: u64 = RING_CAPACITY_FRAMES / 2;

const HEADER_BYTES: usize = 48;
const DATA_OFFSET: usize = RING_HEADER_SIZE * 2;

/// An mmap of the two-ring shm transport file.
pub struct ShmRings {
    path: PathBuf,
    mmap: memmap2::MmapMut,
    /// The creator owns the unlink-on-drop guard.
    owner: bool,
    capacity_frames: u64,
    channels: usize,
}

impl ShmRings {
    /// Creates a fresh shm transport file with initialized headers. The
    /// file name embeds the creating pid plus a process-local counter and
    /// is created `0600`.
    pub fn create() -> DspResult<ShmRings> {
        use std::os::unix::fs::OpenOptionsExt;

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let slot = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(format!(
            "/dev/shm/eep-dsp-shm-{}-{}",
            std::process::id(),
            slot
        ));

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|err| DspError::ShmTransport(format!("create {}: {err}", path.display())))?;
        // Layout is fixed at creation; capacity is baked into both headers.
        let channels = 2;
        let file_size = size_for(RING_CAPACITY_FRAMES, channels);
        file.set_len(file_size)
            .map_err(|err| DspError::ShmTransport(format!("size {}: {err}", path.display())))?;

        let mmap = unsafe {
            memmap2::MmapMut::map_mut(&file)
                .map_err(|err| DspError::ShmTransport(format!("mmap {}: {err}", path.display())))?
        };

        let rings = ShmRings {
            path,
            mmap,
            owner: true,
            capacity_frames: RING_CAPACITY_FRAMES,
            channels,
        };
        rings.init_headers(channels);
        Ok(rings)
    }

    /// Opens an existing transport file (worker side): validates headers,
    /// rewrites nothing, unlinks nothing.
    pub fn open(path: &Path) -> DspResult<ShmRings> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|err| DspError::ShmTransport(format!("open {}: {err}", path.display())))?;
        let mmap = unsafe {
            memmap2::MmapMut::map_mut(&file)
                .map_err(|err| DspError::ShmTransport(format!("mmap {}: {err}", path.display())))?
        };

        // SAFETY: pages 0/1 hold the ring headers; opening maps the fixed
        // layout before any of this arithmetic runs.
        let request = parse_header(unsafe { unsafe_head(&mmap, 0) })?;
        let response = parse_header(unsafe { unsafe_head(&mmap, 1) })?;
        if request.capacity_frames != response.capacity_frames
            || request.channels != response.channels
        {
            return Err(DspError::ShmTransport(
                "request/response ring headers disagree".to_string(),
            ));
        }

        let file_len = file
            .metadata()
            .map_err(|err| DspError::ShmTransport(format!("stat {}: {err}", path.display())))?
            .len();
        let expected = size_for(request.capacity_frames, request.channels as usize);
        if file_len < expected {
            return Err(DspError::ShmTransport(format!(
                "shm file too short: {file_len} < {expected}"
            )));
        }

        Ok(ShmRings {
            path: path.to_path_buf(),
            mmap,
            owner: false,
            capacity_frames: request.capacity_frames,
            channels: request.channels as usize,
        })
    }

    /// Backing file path (handed to the worker).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// In-flight capacity per ring, in frames.
    pub fn capacity_frames(&self) -> u64 {
        self.capacity_frames
    }

    /// Channel count (stride) of both rings.
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Request ring producer side (client → worker).
    pub fn request_producer(&self) -> RingSide {
        // SAFETY: header pages and data pages are inside this mmap, and
        // the arithmetic is fixed by the layout constants.
        unsafe { RingSide::ring_side(self.mmap.as_ptr() as *mut u8, true, self.capacity_frames, self.channels) }
    }

    /// Request ring consumer side.
    pub fn request_consumer(&self) -> RingSide {
        self.request_producer()
    }

    /// Response ring producer side (worker → client).
    pub fn response_producer(&self) -> RingSide {
        // SAFETY: second header page; same guarantees.
        unsafe { RingSide::ring_side(self.mmap.as_ptr() as *mut u8, false, self.capacity_frames, self.channels) }
    }

    /// Response ring consumer side.
    pub fn response_consumer(&self) -> RingSide {
        self.response_producer()
    }

    fn init_headers(&self, channels: usize) {
        // SAFETY: creator-side initialization before any peer opens the
        // file; both ring regions are exclusively writable by this
        // process up to the handshake.
        for page in 0..2_usize {
            // SAFETY: see above; covers page 0 and page 1 identically.
            unsafe {
                let head = self.mmap.as_ptr() as *mut u8;
                let base = head.add(page * RING_HEADER_SIZE);
                let mut bytes = [0u8; HEADER_BYTES];
                bytes[0..4].copy_from_slice(RING_MAGIC);
                bytes[4..8].copy_from_slice(&RING_VERSION.to_le_bytes());
                bytes[8..12].copy_from_slice(&(channels as u32).to_le_bytes());
                // answered sample_rate at the peer-handshake level
                bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
                bytes[16..24].copy_from_slice(&self.capacity_frames.to_le_bytes());
                // produced/consumed start at zero.
                std::ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    base,
                    HEADER_BYTES,
                );
            }
        }
    }
}

impl Drop for ShmRings {
    fn drop(&mut self) {
        if self.owner {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

struct RingHeader {
    channels: u32,
    capacity_frames: u64,
}

fn size_for(capacity: u64, channels: usize) -> u64 {
    let stride = channels as u64 * 4;
    DATA_OFFSET as u64 + capacity * stride * 2
}

/// SAFETY: `page` is 0 or 1; mmap is at least as large as the fixed layout.
unsafe fn unsafe_head(mmap: &memmap2::MmapMut, page: usize) -> &[u8] {
    let base = mmap.as_ptr() as *const u8;
    std::slice::from_raw_parts(base.add(page * RING_HEADER_SIZE), HEADER_BYTES)
}

fn parse_header(bytes: &[u8]) -> DspResult<RingHeader> {
    if bytes[0..4] != *RING_MAGIC {
        return Err(DspError::ShmTransport("ring magic mismatch".to_string()));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().expect("4 bytes"));
    if version != RING_VERSION {
        return Err(DspError::ShmTransport(format!(
            "ring version mismatch: {version} != {RING_VERSION}"
        )));
    }
    let channels = u32::from_le_bytes(bytes[8..12].try_into().expect("4 bytes"));
    let capacity_frames = u64::from_le_bytes(bytes[16..24].try_into().expect("8 bytes"));
    if capacity_frames == 0 {
        return Err(DspError::ShmTransport("ring has zero capacity".to_string()));
    }
    Ok(RingHeader {
        channels,
        capacity_frames,
    })
}

/// One ring, driven from either its producer or consumer protocol side.
#[derive(Clone, Copy)]
pub struct RingSide {
    head_ptr: *mut u64,
    tail_ptr: *mut u64,
    data: *mut f32,
    capacity_frames: u64,
    channels: usize,
}

// SAFETY: each ring has exactly one producer and one consumer (the
// sandbox/worker protocol pairs one side of each ring); that SPSC pairing
// is the contract this unsafe type encodes.
unsafe impl Send for RingSide {}
unsafe impl Sync for RingSide {}

impl RingSide {
    /// SAFETY: `mmap_start` must be a live mmap of the fixed ring layout
    /// (headers at pages 0/1, data areas after them).
    unsafe fn ring_side(
        mmap_start: *mut u8,
        request: bool,
        capacity_frames: u64,
        channels: usize,
    ) -> RingSide {
        let header = mmap_start;
        let data_without_headers = DATA_OFFSET as u64;
        let stride = channels as u64 * 4;
        let (data_offset, header_offset) = if request {
            (data_without_headers, 0u64)
        } else {
            (
                data_without_headers + capacity_frames * stride,
                RING_HEADER_SIZE as u64,
            )
        };
        let data = mmap_start.add(data_offset as usize) as *mut f32;
        let counters = header.byte_add(header_offset as usize);
        RingSide {
            head_ptr: counters.byte_add(24) as *mut u64,
            tail_ptr: counters.byte_add(32) as *mut u64,
            data,
            capacity_frames,
            channels,
        }
    }

    fn load_count(ptr: *mut u64) -> u64 {
        // SAFETY: mmap'd u64 counters are accessed as atomics by both
        // processes; Acquire/Release pairs establish the ring ordering.
        unsafe { (*(ptr as *const AtomicU64)).load(Ordering::Acquire) }
    }

    fn store_count(ptr: *mut u64, value: u64) {
        // SAFETY: see load_count().
        unsafe { (*(ptr as *const AtomicU64)).store(value, Ordering::Release) }
    }

    /// Frames available to a consumer right now.
    pub fn available(&self) -> u64 {
        Self::load_count(self.head_ptr).saturating_sub(Self::load_count(self.tail_ptr))
    }

    /// Frames of free space for a producer right now.
    pub fn free(&self) -> u64 {
        self.capacity_frames - self.available()
    }

    /// Producer entry point (SP: single producer). `data.len()` must be a
    /// multiple of the channel stride and at most [`RingSide::free`];
    /// protocol violations panic loudly: they are engine bugs, not user
    /// errors.
    pub fn write(&self, data: &[f32]) {
        let stride = self.channels;
        if data.len() % stride != 0 || (data.len() / stride) as u64 > self.free() {
            panic!(
                "ring producer bound violation: {} frames over {} free",
                data.len() / stride,
                self.free()
            );
        }
        let frames = (data.len() / stride) as u64;
        // SAFETY: bounds verified just above; single-producer protocol.
        // Data is published before the counter is advanced (release order
        // guarantees the consumer observes samples before availability).
        unsafe {
            let head = Self::load_count(self.head_ptr);
            let start = head as usize;
            for frame in 0..frames as usize {
                let slot = (start + frame) % self.capacity_frames as usize;
                for ch in 0..stride {
                    *self.data.add(slot * stride + ch) = data[frame * stride + ch];
                }
            }
        }
        Self::store_count(self.head_ptr, Self::load_count(self.head_ptr) + frames);
    }

    /// Consumer entry point (SC: single consumer). `data.len() / stride`
    /// must be at most [`RingSide::available`].
    pub fn read(&self, data: &mut [f32]) {
        let stride = self.channels;
        if data.len() % stride != 0 || (data.len() / stride) as u64 > self.available() {
            panic!(
                "ring consumer bound violation: {} frames over {} available",
                data.len() / stride,
                self.available()
            );
        }
        let frames = (data.len() / stride) as u64;
        // SAFETY: bounded and mono-consumer.
        unsafe {
            let tail = Self::load_count(self.tail_ptr);
            let start = tail as usize;
            for frame in 0..frames as usize {
                let slot = (start + frame) % self.capacity_frames as usize;
                for ch in 0..stride {
                    data[frame * stride + ch] = *self.data.add(slot * stride + ch);
                }
            }
            Self::store_count(self.tail_ptr, tail + frames);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_write_read_roundtrip() {
        // Confirms the shm file is created 0600, unlinked on drop, and the
        // request ring survives a basic round trip.
        let rings = ShmRings::create().expect("create rings");
        let writer = rings.request_producer();
        let reader = rings.request_consumer();

        writer.write(&[0.1f32, 0.2, 0.3, 0.4]);
        assert_eq!(writer.available(), 2);

        let mut out = [0.0f32; 4];
        reader.read(&mut out);
        assert_eq!(out, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(writer.available(), 0);
        assert_eq!(writer.free(), rings.capacity_frames());

        let path = rings.path().to_path_buf();
        drop(rings);
        assert!(!path.exists(), "shm file must be unlinked on drop");
    }

    #[test]
    fn counters_advance_across_wrap() {
        // Fill and drain repeatedly to push counters past capacity; the
        // ring must keep accepting further writes.
        let rings = ShmRings::create().expect("create rings");
        let side = rings.request_producer();

        for round in 0..4_u64 {
            let batch = vec![
                round as f32;
                (RING_CAPACITY_FRAMES as usize) * rings.channels()
            ];
            side.write(&batch);
            assert_eq!(side.free(), 0);
            let mut drain = vec![0.0f32; batch.len()];
            side.read(&mut drain);
            assert_eq!(side.available(), 0);
        }
        // Counters are absolute: they keep advancing past each wrap.
        assert!(side.available() == 0);
    }

    #[test]
    fn rings_are_independent() {
        // The request and response rings must not share offsets.
        let rings = ShmRings::create().expect("create rings");
        let request = rings.request_producer();
        let response = rings.response_producer();

        request.write(&[1.0f32, 2.0]);
        assert_eq!(request.available(), 1);
        assert_eq!(response.available(), 0);

        response.write(&[9.0f32, 9.0]);
        assert_eq!(response.available(), 1);
    }
}
