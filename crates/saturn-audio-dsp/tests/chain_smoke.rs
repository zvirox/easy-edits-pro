// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

//! In-process chain smoke tests against the shipped fixture plugin.
//!
//! Contract exercised (fixture `org.eep.fixture.gain`):
//! * parameter id 0, range 0..1, playback gain `value * 2.0`
//!   (0 mutes, 0.5 is unity, 1.0 doubles)

use std::path::PathBuf;
use std::sync::OnceLock;

use saturn_audio_dsp::chain::PluginChain;
use saturn_audio_dsp::errors::DspError;
use saturn_core::project::{DspPluginFormat, DspPluginSlot, DspPluginSlotParam};

/// One serialized gate shared by every test that swaps
/// `SATURN_PLUGIN_ROOT`: the trusted-root lookup is process-global state.
fn trust_gate() -> std::sync::MutexGuard<'static, ()> {
    static GATE: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    GATE.get_or_init(std::sync::Mutex::default)
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

/// Locates the built fixture library under the workspace target dir.
///
/// The workspace target may be a symlink (e.g. into a build cache), so the
/// canonicalized exe path has no `target/` path segment; both the literal
/// `target/debug` form and the profile-dir form are checked, plus the
/// crate-relative workspace root as a last resort.
fn fixture_path() -> PathBuf {
    const LIB: &str = "libsaturn_plugin_fixture.so";

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_targets = [
        manifest.join("../../target/debug").join(LIB),
        manifest.join("../../target/debug/deps").join(LIB),
    ];
    for candidate in workspace_targets {
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
    panic!(
        "fixture {LIB} not found; build it: cargo build -p saturn-plugin-fixture"
    );
}

fn gain_slot(value: f64, enabled: bool) -> DspPluginSlot {
    DspPluginSlot {
        format: DspPluginFormat::Clap,
        path: fixture_path().display().to_string(),
        plugin_id: "org.eep.fixture.gain".to_string(),
        enabled,
        params: vec![DspPluginSlotParam {
            param_id: 0,
            value,
        }],
    }
}

fn stereo_interleaved(n_frames: usize, left: f32, right: f32) -> Vec<f32> {
    (0..n_frames)
        .flat_map(|frame| [frame as f32 / 1000.0 + left, right])
        .collect()
}

fn expected_gain(original: &[f32], gain: f64) -> Vec<f32> {
    original
        .iter()
        .map(|sample| sample * gain as f32)
        .collect()
}

#[test]
fn doubles_at_full_gain_unity_at_half_mutes_at_zero() {
    let _guard = trust_gate();
    // SAFETY: all env swaps in this suite are serialized by the gate.
    unsafe {
        std::env::set_var(
            "SATURN_PLUGIN_ROOT",
            fixture_path().parent().unwrap().display().to_string(),
        );
    }

    for (value, gain) in [(1.0, 2.0), (0.5, 1.0), (0.0, 0.0)] {
        let slots = [gain_slot(value, true)];
        let mut chain = PluginChain::load(&slots, 48_000, 2).expect("load chain");
        assert_eq!(chain.slot_count(), 1);

        let input = stereo_interleaved(512, 0.25, -0.25);
        let original: Vec<f32> = input.clone();
        let mut block = input;
        chain.process_interleaved(&mut block).expect("process");

        let expected = expected_gain(&original, gain);
        for (got, want) in block.iter().zip(&expected) {
            assert!((got - want).abs() < 1e-4, "got {got}, want {want}");
        }
    }
    // SAFETY: env cleanup serialized by the gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn long_input_splits_into_blocks_and_stays_continuous() {
    let _guard = trust_gate();
    // SAFETY: all env swaps in this suite are serialized by the gate.
    unsafe {
        std::env::set_var(
            "SATURN_PLUGIN_ROOT",
            fixture_path().parent().unwrap().display().to_string(),
        );
    }

    let slots = [gain_slot(0.5, true)];
    let mut chain = PluginChain::load(&slots, 48_000, 2).expect("load chain");

    let n_frames = 10_013; // deliberately not a multiple of the chain block
    let mut block: Vec<f32> = (0..n_frames * 2)
        .map(|i| ((i % 977) as f32) / 977.0 - 0.5)
        .collect();
    let original = block.clone();
    chain.process_interleaved(&mut block).expect("process");
    let expected = expected_gain(&original, 1.0);
    for (got, want) in block.iter().zip(&expected) {
        assert!((got - want).abs() < 1e-4);
    }
    // SAFETY: env cleanup serialized by the gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn empty_chain_is_passthrough() {
    let _guard = trust_gate();
    let slots = [gain_slot(0.5, false)];
    let mut chain = PluginChain::load(&slots, 48_000, 2).expect("load chain");
    assert_eq!(chain.slot_count(), 0);

    let mut block: Vec<f32> = (0..1024).map(|i| i as f32 * 0.001).collect();
    let original = block.clone();
    chain.process_interleaved(&mut block).expect("process");
    assert_eq!(block, original);
}

#[test]
fn wrong_descriptor_id_is_typed_error() {
    let _guard = trust_gate();
    // SAFETY: all env swaps in this suite are serialized by the gate.
    unsafe {
        std::env::set_var(
            "SATURN_PLUGIN_ROOT",
            fixture_path().parent().unwrap().display().to_string(),
        );
    }

    let mut slot = gain_slot(0.5, true);
    slot.plugin_id = "org.not.the.fixture".to_string();
    let err = match PluginChain::load(&[slot], 48_000, 2) {
        Err(DspError::PluginUnloadable(_, detail)) => detail,
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("must reject a wrong descriptor id"),
    };
    assert!(err.contains("org.not.the.fixture"), "{err}");
    // SAFETY: env cleanup serialized by the gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn unhosted_format_is_typed_error() {
    let mut slot = gain_slot(0.5, true);
    slot.format = DspPluginFormat::Vst3;
    let err = match PluginChain::load(&[slot], 48_000, 2) {
        Err(err @ DspError::FormatNotHosted(_)) => err,
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("must reject an unhosted format"),
    };
    assert!(err.to_string().contains("vst3"), "{err}");
}

#[test]
fn path_outside_trusted_roots_is_typed_error() {
    let _guard = trust_gate();
    // /tmp is never configured as a plugin root here.
    // SAFETY: all env swaps in this suite are serialized by the gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
    let mut slot = gain_slot(0.5, true);
    slot.path = std::env::temp_dir()
        .join("eep-dsp-not-ever-a-plugin.so")
        .display()
        .to_string();
    std::fs::write(&slot.path, b"").unwrap();
    let err = match PluginChain::load(&[slot], 48_000, 2) {
        Err(err @ DspError::PluginPathUntrusted(_)) => err,
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("must reject an untrusted path"),
    };
    assert!(err.to_string().contains("trusted plugin roots"), "{err}");
    // SAFETY: env cleanup serialized by the gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn out_of_range_param_is_typed_error() {
    let _guard = trust_gate();
    // SAFETY: all env swaps in this suite are serialized by the gate.
    unsafe {
        std::env::set_var(
            "SATURN_PLUGIN_ROOT",
            fixture_path().parent().unwrap().display().to_string(),
        );
    }

    let mut slot = gain_slot(0.5, true);
    slot.params = vec![DspPluginSlotParam {
        param_id: 0,
        value: 1.5,
    }];
    let err = match PluginChain::load(&[slot], 48_000, 2) {
        Err(err @ DspError::ParameterInvalid(..)) => err,
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("must reject an out-of-range parameter"),
    };
    assert!(err.to_string().contains("1.5"), "{err}");
    // SAFETY: env cleanup serialized by the gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn more_slots_than_the_cap_is_typed_error() {
    let _guard = trust_gate();
    // Disabled slots bypass loading, so the cap is the only guard here.
    // SAFETY: all env swaps in this suite are serialized by the gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
    let slots: Vec<DspPluginSlot> = (0..65).map(|_| gain_slot(0.5, false)).collect();
    let err = match PluginChain::load(&slots, 48_000, 2) {
        Err(err @ DspError::SlotInvalid(65)) => err,
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("must reject more than the slot cap"),
    };
    assert!(err.to_string().contains("out of range"), "{err}");
    // SAFETY: env cleanup serialized by the gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}
