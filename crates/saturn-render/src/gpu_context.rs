// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Real `wgpu` device context for the scene-referred colour engine
//! (Packet 3.1).
//!
//! Owns a `wgpu::Device`/`Queue`, creates `Rgba32Float` textures for 32-bit
//! float colour work, and implements the Linux DMA-BUF zero-copy import path
//! through wgpu-hal's `texture_from_dmabuf_fd` (Vulkan, gated on the
//! `VULKAN_EXTERNAL_MEMORY_DMA_BUF` device feature). Every failure mode is a
//! typed error: an unavailable device or unsupported import is reported,
//! never silently degraded.

use std::os::fd::OwnedFd;

use thiserror::Error;
use wgpu::hal::vulkan::Api as VulkanApi;

/// Typed failures of GPU context creation and texture import.
#[derive(Debug, Error)]
pub enum GpuContextError {
    /// No Vulkan adapter could be initialised on this system.
    #[error("no Vulkan adapter available for the render engine: {0}")]
    NoVulkanAdapter(String),
    /// `request_device` failed (driver, features or limits).
    #[error("wgpu device request failed: {0}")]
    RequestDevice(String),
    /// The adapter cannot import DMA-BUF external memory.
    #[error("adapter does not support the DMA-BUF external-memory feature")]
    DmaBufUnsupported,
    /// The requested pixel format is not supported for DMA-BUF import.
    #[error("DMA-BUF import does not support format {0:?}")]
    DmaBufFormatUnsupported(wgpu::TextureFormat),
    /// The underlying Vulkan external-memory import failed.
    #[error("DMA-BUF import failed: {0}")]
    DmaBufImport(String),
    /// WGSL module failed naga validation.
    #[error("shader compilation failed: {0}")]
    ShaderCompile(String),
    /// A GPU readback failed.
    #[error("GPU readback failed: {0}")]
    Readback(String),
}

/// Descriptor of a Linux DMA-BUF plane for zero-copy import.
///
/// Only single-plane DMA-BUFs are supported (multi-planar YUV import is not
/// provided by the underlying `texture_from_dmabuf_fd`; callers get a typed
/// error, never a silent fallback). On success the driver takes ownership of
/// the file descriptor.
#[derive(Debug)]
pub struct DmaBufDescriptor {
    /// Owned file descriptor of the DMA-BUF (consumed by the driver).
    pub fd: OwnedFd,
    /// Pixel width in texels.
    pub width: u32,
    /// Pixel height in texels.
    pub height: u32,
    /// Pixel format (explicit single-plane whitelist).
    pub format: wgpu::TextureFormat,
    /// DRM format modifier; callers must resolve the producer's actual
    /// modifier (`DRM_FORMAT_MOD_INVALID` is rejected by the driver).
    pub drm_modifier: u64,
    /// Row stride in bytes.
    pub stride: u64,
    /// Byte offset of the plane inside the buffer.
    pub offset: u64,
}

/// A live `wgpu` device plus queue for the colour engine.
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// Capability flag: whether the device was created with the DMA-BUF
    /// external-memory feature (negotiated at creation time, see [`new`]).
    dmabuf_supported: bool,
}

impl GpuContext {
    /// Initialises a Vulkan device synchronously. Fails honestly when no
    /// Vulkan adapter exists — there is no CPU path masquerading as GPU.
    ///
    /// The DMA-BUF external-memory feature is negotiated: a device lacking
    /// it is still usable for compute, with [`Self::supports_dmabuf`] →
    /// `false` and `import_dmabuf` returning a typed error.
    pub fn new() -> Result<Self, GpuContextError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| GpuContextError::NoVulkanAdapter(e.to_string()))?;

        let base_desc = |label: &'static str, required_features: wgpu::Features| {
            wgpu::DeviceDescriptor {
                label: Some(label),
                required_features,
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            }
        };
        // Preferred request: with DMA-BUF external memory enabled.
        let dmabuf_capable = adapter
            .features()
            .contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_DMA_BUF);
        let (device, queue) = if dmabuf_capable {
            match pollster::block_on(adapter.request_device(
                &base_desc("saturn-render gpu context (dmabuf)", wgpu::Features::VULKAN_EXTERNAL_MEMORY_DMA_BUF),
            )) {
                Ok(dq) => dq,
                Err(e) => return Err(GpuContextError::RequestDevice(e.to_string())),
            }
        } else {
            match pollster::block_on(adapter.request_device(&base_desc(
                "saturn-render gpu context",
                wgpu::Features::empty(),
            ))) {
                Ok(dq) => dq,
                Err(e) => return Err(GpuContextError::RequestDevice(e.to_string())),
            }
        };

        Ok(Self {
            device,
            queue,
            dmabuf_supported: dmabuf_capable,
        })
    }

    /// Reference to the underlying logical device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Reference to the submission queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Whether this device can import DMA-BUF external memory.
    pub fn supports_dmabuf(&self) -> bool {
        self.dmabuf_supported
    }

    /// Creates an empty `Rgba32Float` 2-D texture (32-bit float, per the
    /// packet spec).
    pub fn create_rgba32float_texture(
        &self,
        width: u32,
        height: u32,
        usage: wgpu::TextureUsages,
    ) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("saturn-render rgba32float"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage,
            view_formats: &[],
        })
    }

    /// Uploads packed RGBA pixels into a `Rgba32Float` texture
    /// (`data.len() == width * height * 4` f32 values, row-major).
    pub fn upload_rgba32float(&self, texture: &wgpu::Texture, width: u32, height: u32, data: &[f32]) {
        debug_assert_eq!(data.len(), (width * height * 4) as usize);
        let bytes: &[u8] = f32_slice_as_bytes(data);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4 * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Reads a `Rgba32Float` texture back into packed f32 RGBA values.
    /// Row staging uses 256-byte-aligned blocks (`wgpu` copy alignment).
    pub fn readback_rgba32float(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Vec<f32>, GpuContextError> {
        let bytes_per_row = (width * 4 * 4) as usize;
        let aligned_bytes_per_row = bytes_per_row.div_ceil(256) * 256;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("saturn-render rgba32float readback"),
            });
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("saturn-render rgba32float readback staging"),
            size: (aligned_bytes_per_row as u64) * (height as u64),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        // Per-row copies keep offsets 256-aligned for any width. The source
        // origin must advance with the row — `as_image_copy()` is always the
        // (0, 0) origin, so it would read row 0 for every row.
        for row in 0..height {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: 0, y: row, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: (row as u64) * (aligned_bytes_per_row as u64),
                        bytes_per_row: Some(aligned_bytes_per_row as u32),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }        self.queue.submit(Some(encoder.finish()));
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).expect("readback channel closed");
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| GpuContextError::Readback(format!("device poll failed: {e:?}")))?;
        rx.recv()
            .map_err(|e| GpuContextError::Readback(format!("readback signal: {e}")))?
            .map_err(|e| GpuContextError::Readback(e.to_string()))?;
        let raw = slice
            .get_mapped_range()
            .map_err(|e| GpuContextError::Readback(e.to_string()))?;
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let offset = row as usize * aligned_bytes_per_row;
            let row_bytes = &raw[offset..offset + bytes_per_row];
            out.extend(
                row_bytes
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]])),
            );
        }
        drop(raw);
        buffer.unmap();
        Ok(out)
    }

    /// Zero-copy imports a single-plane DMA-BUF as a sampled wgpu texture
    /// through the Vulkan external-memory path. The driver takes ownership
    /// of the file descriptor on success; on a failed import the fd may
    /// already be closed (documented behaviour of wgpu-hal
    /// `texture_from_dmabuf_fd`).
    pub fn import_dmabuf(
        &self,
        dmabuf: DmaBufDescriptor,
    ) -> Result<wgpu::Texture, GpuContextError> {
        if !self.supports_dmabuf() {
            return Err(GpuContextError::DmaBufUnsupported);
        }
        if !dmabuf_format_supported(dmabuf.format) {
            return Err(GpuContextError::DmaBufFormatUnsupported(dmabuf.format));
        }
        let DmaBufDescriptor {
            fd,
            width,
            height,
            format,
            drm_modifier,
            stride,
            offset,
        } = dmabuf;

        let wgpu_desc = wgpu::TextureDescriptor {
            label: Some("saturn-render dmabuf import"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let hal_desc = wgpu::hal::TextureDescriptor {
            label: wgpu_desc.label,
            size: wgpu_desc.size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUses::RESOURCE | wgpu::TextureUses::COPY_SRC,
            memory_flags: wgpu::hal::MemoryFlags::empty(),
            view_formats: Vec::new(),
        };

        // SAFETY: the hal texture is created from this device's own Vulkan
        // device; the fd is a valid DMA-BUF matching the descriptor per the
        // caller's contract; extents, format, modifier, stride and offset
        // all mirror the caller-supplied descriptor.
        let hal_texture: wgpu::hal::vulkan::Texture = {
            let vulkan_device = unsafe { self.device.as_hal::<VulkanApi>() }
                .ok_or_else(|| {
                    GpuContextError::DmaBufImport("device is not on the Vulkan backend".into())
                })?;
            unsafe {
                vulkan_device.texture_from_dmabuf_fd(fd, &hal_desc, drm_modifier, stride, offset)
            }
            .map_err(|e| GpuContextError::DmaBufImport(e.to_string()))?
        };
        // SAFETY: `hal_texture` was created for this device and respects
        // `wgpu_desc`; contents are producer-initialised, so
        // UNINITIALIZED hands state tracking to wgpu.
        Ok(unsafe {
            self.device.create_texture_from_hal::<VulkanApi>(
                hal_texture,
                &wgpu_desc,
                wgpu::TextureUses::UNINITIALIZED,
            )
        })
    }
}

/// Explicit single-plane format whitelist for DMA-BUF import.
pub fn dmabuf_format_supported(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::R8Unorm
            | wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Rgba16Float
            | wgpu::TextureFormat::Rgba32Float
    )
}

/// Public wrapper: casts an array-of-f32-words to bytes (uniform packing).
pub fn f32_words_as_bytes(data: &[f32]) -> &[u8] {
    f32_slice_as_bytes(data)
}

/// Public wrapper: casts u32 words to bytes without copying.
pub fn u32_words_as_bytes(data: &[u32]) -> &[u8] {
    // SAFETY: same layout argument as `f32_slice_as_bytes`.
    unsafe {
        std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), std::mem::size_of_val(data))
    }
}

/// Casts a packed f32 slice to bytes without copying.
fn f32_slice_as_bytes(data: &[f32]) -> &[u8] {
    // SAFETY: `f32` and `u8` slices share contiguity and `u8` has alignment
    // 1; the total byte length equals `size_of_val(data)`.
    unsafe {
        std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), std::mem::size_of_val(data))
    }
}
