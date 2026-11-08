// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! Sandbox integration tests: a real `saturn-plugin-worker` process hosting
//! the fixture plugin chain, driven end to end over the UDS + shm transport.
//!
//! These tests build the worker and fixture binaries first; if either is
//! missing they are skipped, keeping `cargo test --workspace` green before
//! full builds have run.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use saturn_audio_dsp::errors::DspError;
use saturn_audio_dsp::sandbox::SandboxSession;
use saturn_core::project::{DspPluginFormat, DspPluginSlot, DspPluginSlotParam};

/// All tests that mutate `SATURN_PLUGIN_ROOT` / `SATURN_PLUGIN_WORKER_BIN`
/// share one serialized gate: environment variables are process-global.
fn env_gate() -> MutexGuard<'static, ()> {
    static GATE: OnceLock<Mutex<()>> = OnceLock::new();
    GATE.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// Unit gain slot pointing at the built fixture library.
fn gain_slot(value: f64) -> DspPluginSlot {
    DspPluginSlot {
        format: DspPluginFormat::Clap,
        path: fixture_path().display().to_string(),
        plugin_id: "org.eep.fixture.gain".to_string(),
        enabled: true,
        params: vec![DspPluginSlotParam {
            param_id: 0,
            value,
        }],
    }
}

fn fixture_path() -> PathBuf {
    const LIB: &str = "libsaturn_plugin_fixture.so";
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for candidate in [
        manifest.join("../../target/debug").join(LIB),
        manifest.join("../../target/debug/deps").join(LIB),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    let exe = std::env::current_exe().expect("current_exe");
    for ancestor in exe.ancestors() {
        for candidate in [
            ancestor.join("target/debug").join(LIB),
            ancestor.join("debug").join(LIB),
            ancestor.join(LIB),
        ] {
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    panic!("fixture {LIB} not found; build it: cargo build -p saturn-plugin-fixture");
}


/// Sends SIGKILL to a pid through the standard `kill` utility. Test-only.
fn kill_pid(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn set_worker_env(root: &str, worker: Option<&PathBuf>) {
    // SAFETY: every env swap in this suite is serialized through env_gate().
    unsafe {
        std::env::set_var("SATURN_PLUGIN_ROOT", root);
        match worker {
            Some(path) => std::env::set_var("SATURN_PLUGIN_WORKER_BIN", path),
            None => std::env::remove_var("SATURN_PLUGIN_WORKER_BIN"),
        }
    }
}

fn clear_worker_env() {
    // SAFETY: see set_worker_env().
    unsafe {
        std::env::remove_var("SATURN_PLUGIN_ROOT");
        std::env::remove_var("SATURN_PLUGIN_WORKER_BIN");
    }
}

/// Drives real worker processes end to end; each test builds the fixture
/// and worker first and fails with a build hint if either is missing.
#[test]
fn sandbox_round_trip_doubles_full_scale() {
    let gate = env_gate();
    set_worker_env(&fixture_path().parent().unwrap().display().to_string(), None);

    let slots = vec![gain_slot(1.0)];
    let mut session = SandboxSession::spawn(&slots, 48_000, 2).expect("spawn sandbox");

    let pid = session.worker_pid().expect("worker pid");
    let mut block = vec![-1.0f32; 1024 * 2]; // 1024 stereo frames at -1
    session.process(&mut block).expect("process");

    for (index, sample) in block.iter().enumerate() {
        assert!(
            (sample + 2.0).abs() < 1e-4,
            "sample {index} became {sample}, want -2.0"
        );
    }

    // The worker is still healthy after a successful round trip.
    assert_eq!(session.worker_pid(), Some(pid), "worker must not respawn");

    session.shutdown();
    clear_worker_env();
    drop(gate);
}

#[test]
fn kill_worker_respawns_once_and_replays() {
    let gate = env_gate();
    set_worker_env(&fixture_path().parent().unwrap().display().to_string(), None);

    let slots = vec![gain_slot(0.5)]; // unity
    let mut session = SandboxSession::spawn(&slots, 48_000, 2).expect("spawn sandbox");

    let first_pid = session.worker_pid().expect("worker pid");
    assert!(kill_pid(first_pid), "SIGKILL must be delivered");

    // The next command trips liveness monitoring → one typed auto-respawn.
    let mut block = vec![0.25f32; 512 * 2];
    session.process(&mut block).expect("respawn + replay");

    let second_pid = session.worker_pid().expect("worker pid after respawn");
    assert_ne!(second_pid, first_pid, "a fresh worker must have spawned");

    // Unity replay: the buffer round-trips unchanged.
    for sample in &block {
        assert!((sample - 0.25).abs() < 1e-4, "got {sample}, want 0.25");
    }

    session.shutdown();
    clear_worker_env();
    drop(gate);
}

#[test]
fn second_consecutive_crash_is_typed_sandbox_error() {
    let gate = env_gate();
    set_worker_env(&fixture_path().parent().unwrap().display().to_string(), None);

    let slots = vec![gain_slot(0.5)];
    let mut session = SandboxSession::spawn(&slots, 48_000, 2).expect("spawn sandbox");

    // First crash: auto-respawns.
    let first_pid = session.worker_pid().expect("worker pid");
    assert!(kill_pid(first_pid));
    let mut warm = vec![0.0f32; 256 * 2];
    session.process(&mut warm).expect("first respawn works");

    // Second crash with the respawn budget already spent: typed error.
    let second_pid = session.worker_pid().expect("worker pid");
    assert!(kill_pid(second_pid));
    let mut block = vec![0.0f32; 256 * 2];
    let err = match session.process(&mut block) {
        Err(err @ DspError::Sandbox(_)) => err,
        Err(other) => panic!("wrong error kind: {other}"),
        Ok(()) => panic!("a second consecutive crash must not silently succeed"),
    };
    assert!(
        err.to_string().contains("worker exited"),
        "unhelpful sandbox error: {err}"
    );

    clear_worker_env();
    drop(gate);
}

#[test]
fn missing_worker_binary_is_typed_spawn_error() {
    let gate = env_gate();
    // A nonexistent override is a refused spawn before any child exists.
    set_worker_env(
        &fixture_path().parent().unwrap().display().to_string(),
        Some(&PathBuf::from("/nonexistent/saturn-plugin-worker")),
    );

    let slots = vec![gain_slot(0.5)];
    let err = match SandboxSession::spawn(&slots, 48_000, 2) {
        Err(err @ DspError::Sandbox(_)) => err,
        Err(other) => panic!("wrong error kind: {other}"),
        Ok(session) => {
            session.shutdown();
            panic!("spawn must fail with a missing worker binary");
        }
    };
    assert!(
        err.to_string().contains("/nonexistent/saturn-plugin-worker"),
        "spawn error must name the override: {err}"
    );

    clear_worker_env();
    drop(gate);
}

/// `/bin/false` exits immediately after exec, so the worker never
/// connects: this exercises the exited-during-handshake path.
#[test]
fn worker_that_exits_immediately_is_typed_handshake_error() {
    let gate = env_gate();
    set_worker_env(
        &fixture_path().parent().unwrap().display().to_string(),
        Some(&PathBuf::from("/bin/false")),
    );

    let slots = vec![gain_slot(0.5)];
    let err = match SandboxSession::spawn(&slots, 48_000, 2) {
        Err(err @ DspError::Sandbox(_)) => err,
        Err(other) => panic!("wrong error kind: {other}"),
        Ok(session) => {
            session.shutdown();
            panic!("spawn must fail when the worker dies during handshake");
        }
    };
    let text = err.to_string();
    assert!(
        text.contains("exited before connecting") || text.contains("exited during handshake")
            || text.contains("spawn worker"),
        "unhelpful handshake error: {text}"
    );

    clear_worker_env();
    drop(gate);
}
