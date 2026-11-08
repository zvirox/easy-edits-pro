// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Saturn plugin worker: the dedicated sandbox process that actually hosts
//! CLAP plugin chains. A crash in any third-party plugin therefore takes
//! down this process, not the renderer.
//!
//! Invoked as `saturn-plugin-worker <control-socket-path> <shm-path>`:
//!
//! 1. Connects to the pre-bound control socket.
//! 2. Validates the `/dev/shm` ring headers (never rewrites them).
//! 3. Reads the handshake `{protocol, sample_rate, channels, slots}` line,
//!    re-validates every plugin path through the trusted roots, and
//!    instantiates the chain host-side.
//! 4. Replies `{"status":"ready","slots":N}` or
//!    `{"status":"error","error":..}`.
//! 5. Serves one command at a time:
//!    * `{"cmd":"process","frames":N}` — drain N frames from the request
//!      ring, run the chain, publish N frames to the response ring, reply
//!      `{"status":"done"}` (or an error describing the failure).
//!    * `{"cmd":"shutdown"}` — clean exit.
//!
//! Diagnostics go to stderr only; stdout stays empty (the client never
//! drains this process's standard output).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use saturn_audio_dsp::{PluginChain, RingSide, ShmRings, RING_MAX_COMMAND_FRAMES};
use saturn_core::project::DspPluginSlot;
use serde::{Deserialize, Serialize};

/// Poll slice for ring waits.
const POLL_SLICE: Duration = Duration::from_millis(2);
/// Longest the worker waits for its peer to feed or drain a ring; the
/// client's own chunk timeout is shorter, so a stall is reported to the
/// client first and this only catches a wedged renderer.
const RING_WAIT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Deserialize)]
struct HandshakeRequest {
    protocol: u32,
    sample_rate: u32,
    channels: usize,
    slots: Vec<DspPluginSlot>,
}

#[derive(Deserialize)]
struct WorkerCommand {
    cmd: String,
    #[serde(default)]
    frames: u64,
}

#[derive(Serialize)]
struct WorkerReply {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slots: Option<usize>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(report) = run(&args) {
        eprintln!("saturn-plugin-worker: {report}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let [socket_path, shm_path] = args else {
        return Err("usage: saturn-plugin-worker <control-socket-path> <shm-path>".to_string());
    };

    let stream = UnixStream::connect(socket_path)
        .map_err(|err| format!("connect {socket_path}: {err}"))?;
    let rings = ShmRings::open(std::path::Path::new(shm_path))
        .map_err(|err| format!("open rings: {err}"))?;

    let request = read_json_line::<HandshakeRequest>(&stream)?;

    if request.protocol != 1 {
        let detail = format!("protocol mismatch: {}", request.protocol);
        let _ = reply_error(&stream, &detail);
        return Err(detail);
    }

    let mut chain = match PluginChain::load(&request.slots, request.sample_rate, request.channels) {
        Ok(chain) => chain,
        Err(err) => {
            let _ = reply_error(&stream, &err.to_string());
            return Err(format!("chain load failed: {err}"));
        }
    };

    let loaded = chain.slot_count();
    if let Err(err) = reply(
        &stream,
        &WorkerReply {
            status: "ready",
            error: None,
            slots: Some(loaded),
        },
    ) {
        return Err(format!("handshake reply: {err}"));
    }

    // Command loop: one line, one command, one reply.
    loop {
        let command = read_json_line::<WorkerCommand>(&stream)?;
        match command.cmd.as_str() {
            "process" => {
                if let Err(report) = worker_process(&rings, &mut chain, command.frames) {
                    // The chain may be in a half-run state; the client
                    // respawns this process on error, so exit cleanly with
                    // the report the renderer will surface.
                    let _ = reply_error(&stream, &report);
                    return Err(report);
                }
                if let Err(err) = reply(
                    &stream,
                    &WorkerReply {
                        status: "done",
                        error: None,
                        slots: None,
                    },
                ) {
                    return Err(format!("done reply: {err}"));
                }
            }
            "shutdown" => {
                let _ = reply(
                    &stream,
                    &WorkerReply {
                        status: "done",
                        error: None,
                        slots: None,
                    },
                );
                return Ok(());
            }
            other => {
                let report = format!("unknown command: {other}");
                let _ = reply_error(&stream, &report);
                return Err(report);
            }
        }
    }
}

fn read_json_line<T: serde::de::DeserializeOwned>(stream: &UnixStream) -> Result<T, String> {
    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    let read = reader
        .read_line(&mut line)
        .map_err(|err| format!("control read: {err}"))?;
    if read == 0 {
        return Err("client closed the control connection".to_string());
    }
    serde_json::from_str(line.trim())
        .map_err(|err| format!("control line decode: {err}"))
}

/// One process command: drain the chain frame count, run the chain,
/// publish the output. The done reply travels through the caller.
fn worker_process(rings: &ShmRings, chain: &mut PluginChain, frames: u64) -> Result<(), String> {
    if frames == 0 {
        return Ok(());
    }
    if frames > RING_MAX_COMMAND_FRAMES {
        return Err(format!(
            "command exceeds the transport chunk ceiling: {frames} > {RING_MAX_COMMAND_FRAMES}"
        ));
    }

    let channels = rings.channels();
    let buffer_len = (frames as usize) * channels;
    let mut buffer = vec![0.0f32; buffer_len];

    let request_ring = rings.request_consumer();
    wait_for_produced(&request_ring, frames)?;
    request_ring.read(&mut buffer);

    chain
        .process_interleaved(&mut buffer)
        .map_err(|err| format!("chain process failed: {err}"))?;

    let response_ring = rings.response_producer();
    // Defensive: by protocol the client has fully drained the response
    // ring between commands, so there is always room for one chunk. If
    // the renderer wedges instead, time out into a typed worker error.
    wait_for_space(&response_ring, frames)?;
    response_ring.write(&buffer);
    Ok(())
}

fn wait_for_produced(ring: &RingSide, frames: u64) -> Result<(), String> {
    let deadline = Instant::now() + RING_WAIT_TIMEOUT;
    while ring.available() < frames {
        if Instant::now() >= deadline {
            return Err(format!(
                "request ring never fed {frames} frames within {RING_WAIT_TIMEOUT:?} (renderer stopped)"
            ));
        }
        std::thread::sleep(POLL_SLICE);
    }
    Ok(())
}

fn wait_for_space(ring: &RingSide, frames: u64) -> Result<(), String> {
    let deadline = Instant::now() + RING_WAIT_TIMEOUT;
    while ring.free() < frames {
        if Instant::now() >= deadline {
            return Err(format!(
                "response ring never drained for {frames} frames within {RING_WAIT_TIMEOUT:?} (renderer stopped)"
            ));
        }
        std::thread::sleep(POLL_SLICE);
    }
    Ok(())
}

fn reply(mut stream: &UnixStream, payload: &WorkerReply) -> Result<(), String> {
    let mut line = serde_json::to_string(payload).map_err(|err| err.to_string())?;
    line.push('\n');
    stream
        .write_all(line.as_bytes())
        .map_err(|err| format!("reply write: {err}"))
}

fn reply_error(stream: &UnixStream, message: &str) -> Result<(), String> {
    reply(
        stream,
        &WorkerReply {
            status: "error",
            error: Some(message.to_string()),
            slots: None,
        },
    )
}
