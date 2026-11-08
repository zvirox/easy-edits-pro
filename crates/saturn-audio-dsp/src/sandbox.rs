// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Sandbox client: spawns the dedicated `saturn-plugin-worker` process and
//! drives it over a Unix-domain control connection plus the `/dev/shm` SPSC
//! ring transport.
//!
//! Protocol (strictly one command in flight):
//!
//! 1. The client creates the ring transport, binds a control socket, then
//!    spawns `saturn-plugin-worker <socket-path> <shm-path>`.
//! 2. The client sends the handshake line over the accepted connection:
//!    `{"protocol":1,"sample_rate":..,"channels":..,"slots":[..]}`
//! 3. The worker instantiates the chain (re-validating every plugin path
//!    against the trusted roots) and replies `{"status":"ready",..}` or
//!    `{"status":"error","error":..}`.
//! 4. Per chunk: the client writes frames into the request ring, sends
//!    `{"cmd":"process","frames":N}`, waits until the response ring holds
//!    `N` frames, reads them back, and consumes the `{"status":"done"}`
//!    reply. `{"cmd":"shutdown"}` ends the worker instead of `done`.
//!
//! One command in flight keeps both rings deadlock-free: a chunk never
//! exceeds half the ring capacity, and both rings are fully drained
//! between commands.
//!
//! Crash handling: if the worker dies mid-command the session is torn down
//! and respawned once with the same chain, replaying the current buffer
//! against the fresh worker. A second consecutive failure surfaces the
//! original blocking condition, word for word, as [`DspError::Sandbox`].

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use saturn_core::project::DspPluginSlot;
use serde::{Deserialize, Serialize};

use crate::errors::{DspError, DspResult};
use crate::shm_ring::{RingSide, ShmRings, RING_MAX_COMMAND_FRAMES};
use crate::trust::validate_plugin_path;

/// Control-socket connect/handshake timeout.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(20);
/// Per-chunk stall timeout before the worker is declared unresponsive.
pub const CHUNK_TIMEOUT: Duration = Duration::from_secs(10);
/// Grace period when letting a responsive worker shut down on its own.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(3);
/// Poll slice for ring waits.
const POLL_SLICE: Duration = Duration::from_millis(2);
/// Protocol version pinned between client and worker.
const PROTOCOL_VERSION: u32 = 1;

#[derive(Serialize)]
struct HandshakeRequest<'a> {
    protocol: u32,
    sample_rate: u32,
    channels: usize,
    slots: &'a [DspPluginSlot],
}

#[derive(Deserialize, Debug)]
struct WorkerReply {
    status: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    slots: Option<usize>,
}

#[derive(Serialize)]
struct ProcessCommand {
    cmd: &'static str,
    frames: u64,
}

/// A live sandbox: one worker process plus its ring transport.
pub struct SandboxSession {
    /// Chain descriptor state, reused for the single automatic respawn.
    slots: Vec<DspPluginSlot>,
    sample_rate: u32,
    channels: usize,
    /// 0 = pristine, 1 = the worker has already been auto-respawned once.
    respawns_used: u32,

    rings: Option<ShmRings>,
    listener: Option<UnixListener>,
    socket: Option<UnixStream>,
    socket_path: Option<PathBuf>,
    worker_path: PathBuf,
    child: Option<std::process::Child>,
}

impl SandboxSession {
    /// Spawns a worker holding `slots`' chain at `sample_rate`.
    ///
    /// Plugin paths are validated on the client before the worker sees
    /// them, so an untrusted path is reported without a child process.
    pub fn spawn(
        slots: &[DspPluginSlot],
        sample_rate: u32,
        channels: usize,
    ) -> DspResult<SandboxSession> {
        for slot in slots.iter().filter(|slot| slot.enabled) {
            validate_plugin_path(&slot.path)?;
        }
        let worker_path = locate_worker_bin()?;

        let mut session = SandboxSession {
            slots: slots.to_vec(),
            sample_rate,
            channels,
            respawns_used: 0,
            rings: None,
            listener: None,
            socket: None,
            socket_path: None,
            worker_path,
            child: None,
        };
        session.respawn()?;
        Ok(session)
    }

    /// Chain (re)spawner: builds rings, socket, worker; runs the handshake.
    fn respawn(&mut self) -> DspResult<()> {
        self.teardown();

        let rings = ShmRings::create()?;
        let socket_path = control_socket_path();
        let listener = UnixListener::bind(&socket_path)
            .map_err(|err| DspError::Sandbox(format!("bind {}: {err}", socket_path.display())))?;

        let mut child = spawn_worker(&self.worker_path, &socket_path, rings.path())?;
        let stream = accept(&listener, &mut child)?;

        let request = HandshakeRequest {
            protocol: PROTOCOL_VERSION,
            sample_rate: self.sample_rate,
            channels: self.channels,
            slots: &self.slots,
        };
        let mut line = serde_json::to_string(&request)
            .map_err(|err| DspError::Sandbox(format!("handshake encode: {err}")))?;
        line.push('\n');
        let mut socket = stream;
        socket
            .write_all(line.as_bytes())
            .map_err(|err| DspError::Sandbox(format!("handshake write: {err}")))?;

        socket
            .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
            .map_err(|err| DspError::Sandbox(format!("handshake timeout setting: {err}")))?;
        let reply = read_reply(&socket)?;
        if reply.status != "ready" {
            let detail = reply.error.unwrap_or_else(|| "unknown".to_string());
            return Err(DspError::Sandbox(format!(
                "worker rejected the chain: {detail}"
            )));
        }
        let expected_slots = self.slots.iter().filter(|slot| slot.enabled).count();
        if reply.slots != Some(expected_slots) {
            return Err(DspError::Sandbox(format!(
                "worker loaded {} slots, expected {expected_slots}",
                reply.slots.unwrap_or(0)
            )));
        }
        socket
            .set_read_timeout(Some(CHUNK_TIMEOUT))
            .map_err(|err| DspError::Sandbox(format!("chunk timeout setting: {err}")))?;

        self.rings = Some(rings);
        self.listener = Some(listener);
        self.socket = Some(socket);
        self.socket_path = Some(socket_path);
        self.child = Some(child);
        Ok(())
    }

    /// Processes `block` (interleaved f32, overwritten in place) through
    /// the sandboxed chain.
    ///
    /// The first failure respawns the worker once and replays the whole
    /// buffer; a second consecutive failure reports the original blocking
    /// condition.
    pub fn process(&mut self, block: &mut [f32]) -> DspResult<()> {
        let first_failure = self.process_once(block);
        if first_failure.is_ok() {
            return Ok(());
        }
        if self.respawns_used >= 1 {
            return first_failure;
        }
        self.respawns_used += 1;
        // Recovery resets: the fresh worker re-instantiates the chain from
        // the persisted slot values, so no cross-block state must be
        // preserved (documented deviation for stateful plugins).
        self.respawn()?;
        self.process_once(block)
    }

    fn process_once(&mut self, block: &mut [f32]) -> DspResult<()> {
        let channels = self
            .rings
            .as_ref()
            .map(|rings| rings.channels())
            .ok_or_else(|| DspError::Sandbox("session has no ring transport".to_string()))?;

        let chunk_frames = RING_MAX_COMMAND_FRAMES as usize;
        let total_frames = block.len() / channels;
        let mut offset_frames = 0usize;

        while offset_frames < total_frames {
            let frames = (total_frames - offset_frames).min(chunk_frames);
            let start = offset_frames * channels;
            let end = start + frames * channels;

            {
                let input_view: &[f32] = &block[start..end];
                self.wait_request_room(frames as u64)?;
                self.request_ring()?.write(input_view);
            }

            self.send_command(&ProcessCommand {
                cmd: "process",
                frames: frames as u64,
            })?;

            let mut chunk_out = vec![0.0f32; frames * channels];
            self.wait_response_ready(frames as u64, &mut chunk_out)?;
            block[start..end].copy_from_slice(&chunk_out);

            self.finish_command()?;
            offset_frames += frames;
        }
        Ok(())
    }

    fn request_ring(&self) -> DspResult<RingSide> {
        self.rings
            .as_ref()
            .map(|rings| rings.request_producer())
            .ok_or_else(|| DspError::Sandbox("session has no ring transport".to_string()))
    }

    fn response_ring(&self) -> DspResult<RingSide> {
        self.rings
            .as_ref()
            .map(|rings| rings.response_consumer())
            .ok_or_else(|| DspError::Sandbox("session has no ring transport".to_string()))
    }

    /// Waits until the request ring has room for `frames`, checking the
    /// worker for early death while polling.
    fn wait_request_room(&mut self, frames: u64) -> DspResult<()> {
        let deadline = Instant::now() + CHUNK_TIMEOUT;
        let ring = self.request_ring()?;
        loop {
            if ring.free() >= frames {
                return Ok(());
            }
            check_child_liveness(self.child.as_mut())?;
            if Instant::now() >= deadline {
                return Err(DspError::Sandbox(format!(
                    "request ring stalled: no room for {frames} frames within {CHUNK_TIMEOUT:?} (worker unresponsive or chain blocked)"
                )));
            }
            std::thread::sleep(POLL_SLICE);
        }
    }

    /// Waits until the response ring holds `frames` produced frames and
    /// copies them out. The child is checked for early death on each poll.
    fn wait_response_ready(&mut self, frames: u64, out: &mut [f32]) -> DspResult<()> {
        let deadline = Instant::now() + CHUNK_TIMEOUT;
        let ring = self.response_ring()?;
        loop {
            if ring.available() >= frames {
                ring.read(out);
                return Ok(());
            }
            check_child_liveness(self.child.as_mut())?;
            if Instant::now() >= deadline {
                return Err(DspError::Sandbox(format!(
                    "response ring stalled: {frames} frames not produced within {CHUNK_TIMEOUT:?} (worker unresponsive or chain blocked)"
                )));
            }
            std::thread::sleep(POLL_SLICE);
        }
    }

    fn send_command(&mut self, command: &ProcessCommand) -> DspResult<()> {
        // A worker that just died would otherwise surface as an opaque
        // read/write syscall error; liveness first keeps the typed report
        // specific on the respawn boundary.
        check_child_liveness(self.child.as_mut())?;
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| DspError::Sandbox("no control socket".to_string()))?;
        let mut line = serde_json::to_string(command)
            .map_err(|err| DspError::Sandbox(format!("command encode: {err}")))?;
        line.push('\n');
        socket.write_all(line.as_bytes()).map_err(|err| {
            // A worker can exit after the liveness check but before this
            // write. Normalize that unavoidable TOCTOU window instead of
            // leaking an opaque `Broken pipe` / `Connection reset` syscall
            // error to render logs.
            match err.kind() {
                std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::ConnectionReset => DspError::Sandbox(format!(
                    "worker control connection closed during command write: {err}"
                )),
                _ => DspError::Sandbox(format!("command write failed: {err}")),
            }
        })
    }

    /// Reads the worker's terminal reply for the last command; maps
    /// worker-reported errors to typed sandbox failures.
    fn finish_command(&mut self) -> DspResult<()> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| DspError::Sandbox("no control socket".to_string()))?;
        let reply = read_reply(socket)?;
        if reply.status == "done" {
            Ok(())
        } else {
            Err(DspError::Sandbox(
                reply.error.unwrap_or_else(|| "unknown worker error".to_string()),
            ))
        }
    }

    /// Tears the transport down (socket, listener file, rings, worker).
    fn teardown(&mut self) {
        if let Some(socket) = self.socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Both);
        }
        self.listener = None;
        if let Some(path) = self.socket_path.take() {
            let _ = std::fs::remove_file(path);
        }
        self.rings = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// PID of the running worker (diagnostics and integration tests).
    pub fn worker_pid(&self) -> Option<u32> {
        self.child.as_ref().map(std::process::Child::id)
    }

    /// Asks the worker to exit; escalates to kill after the grace period.
    /// This consumes the session.
    pub fn shutdown(mut self) {
        if let Some(socket) = self.socket.as_mut() {
            if let Ok(mut json) =
                serde_json::to_string(&ProcessCommand { cmd: "shutdown", frames: 0 })
            {
                json.push('\n');
                let _ = socket.write_all(json.as_bytes());
            }
        }
        if let Some(child) = self.child.as_mut() {
            let deadline = Instant::now() + SHUTDOWN_GRACE;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() >= deadline => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    Ok(None) => std::thread::sleep(POLL_SLICE),
                    Err(_) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
        }
        self.teardown();
    }
}

impl Drop for SandboxSession {
    fn drop(&mut self) {
        self.teardown();
    }
}

fn check_child_liveness(child: Option<&mut std::process::Child>) -> DspResult<()> {
    let Some(child) = child else {
        return Ok(());
    };
    match child.try_wait() {
        Ok(None) => Ok(()),
        Ok(Some(status)) => Err(DspError::Sandbox(format!(
            "worker exited mid-render: {status}"
        ))),
        Err(err) => Err(DspError::Sandbox(format!(
            "worker liveness check failed: {err}"
        ))),
    }
}

fn accept(listener: &UnixListener, child: &mut std::process::Child) -> DspResult<UnixStream> {
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    loop {
        // Blocking accept with a deadline wheel: the worker connects right
        // after spawn, so this loop exits within a poll slice.
        listener.set_nonblocking(true).map_err(|err| {
            DspError::Sandbox(format!("listener nonblocking: {err}"))
        })?;
        match listener.accept() {
            Ok((stream, _addr)) => return Ok(stream),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if let Some(status) = child.try_wait().ok().flatten() {
                    return Err(DspError::Sandbox(format!(
                        "worker exited before connecting: {status}"
                    )));
                }
                if Instant::now() >= deadline {
                    return Err(DspError::Sandbox(
                        "worker never connected within the handshake window".to_string(),
                    ));
                }
                std::thread::sleep(POLL_SLICE);
            }
            Err(err) => return Err(DspError::Sandbox(format!("accept: {err}"))),
        }
    }
}

/// Control-socket path for a one-off session (0600-adjacent semantics: the
/// socket itself is unlinkable only by its owner; `/tmp` sticky bit
/// enforces that).
fn control_socket_path() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let slot = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    PathBuf::from(format!(
        "/tmp/eep-dsp-ctl-{}-{}.sock",
        std::process::id(),
        slot
    ))
}

/// Spawns the worker with its two protocol arguments.
fn spawn_worker(
    worker_path: &Path,
    socket_path: &Path,
    shm_path: &Path,
) -> DspResult<std::process::Child> {
    std::process::Command::new(worker_path)
        .arg(socket_path)
        .arg(shm_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|err| {
            DspError::Sandbox(format!("spawn worker {}: {err}", worker_path.display()))
        })
}

/// Reads a single reply line from the socket. The socket's read timeout
/// bounds the wait; the connection is one-reply-per-command, so re-reading
/// through a fresh [`BufReader`] cannot swallow a sibling reply.
fn read_reply(socket: &UnixStream) -> DspResult<WorkerReply> {
    let mut line = String::new();
    let read = BufReader::new(socket)
        .read_line(&mut line)
        .map_err(|err| DspError::Sandbox(format!("worker reply read: {err}")))?;
    if read == 0 {
        return Err(DspError::Sandbox(
            "worker closed the control connection".to_string(),
        ));
    }
    serde_json::from_str(line.trim())
        .map_err(|err| DspError::Sandbox(format!("worker reply decode: {err}")))
}

/// Locates the worker binary:
///
/// 1. `SATURN_PLUGIN_WORKER_BIN` (operator override)
/// 2. `saturn-plugin-worker` next to the current executable (packaged
///    layout)
/// 3. `target/debug`/`target/release` profile directories in an ancestor
///    of the current executable (cargo test/dev layout)
pub fn locate_worker_bin() -> DspResult<PathBuf> {
    let mut searched = Vec::new();

    if let Ok(env_path) = std::env::var("SATURN_PLUGIN_WORKER_BIN") {
        let path = PathBuf::from(env_path);
        // An operator override is an attestation: honor it exactly or fail.
        if path.is_file() {
            return Ok(path);
        }
        return Err(DspError::Sandbox(format!(
            "SATURN_PLUGIN_WORKER_BIN override is not a file: {}",
            path.display()
        )));
    }

    let exe = std::env::current_exe()
        .map_err(|err| DspError::Sandbox(format!("current_exe unavailable: {err}")))?;
    let exe_dir = exe
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let sibling = exe_dir.join("saturn-plugin-worker");
    if sibling.is_file() {
        return Ok(sibling);
    }
    searched.push(sibling.display().to_string());

    for ancestor in exe.ancestors().skip(1) {
        for profile in ["target/debug", "target/release"] {
            let candidate = ancestor.join(profile).join("saturn-plugin-worker");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        // cargo puts test executables in <target>/deps/ one level down from
        // the profile dir; the workspace target may also be a symlink (build
        // cache), in which case no ancestor carries a `target/` segment. The
        // bare profile-dir forms cover both.
        for candidate in [
            ancestor.join("debug/saturn-plugin-worker"),
            ancestor.join("release/saturn-plugin-worker"),
            ancestor.join("saturn-plugin-worker"),
        ] {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        searched.push(
            ancestor
                .join("target/{debug,release}/saturn-plugin-worker")
                .display()
                .to_string(),
        );
    }

    Err(DspError::Sandbox(
        format!(
            "saturn-plugin-worker binary not found — searched: {}",
            searched.join("; ")
        ),
    ))
}

/// Human-readable error text describing a worker lookup failure, for logs
/// and command responses.
pub fn worker_search_error_text(err: &DspError) -> String {
    err.to_string()
}
