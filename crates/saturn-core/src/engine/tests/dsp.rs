// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use crate::project::dsp_trust_gate;
use serde_json::{json, Value};

// The dsp.* commands validate plugin paths against the trusted roots. The
// trust policy is process-global env state, so these tests share the same
// serialized gate as the dsp_trust tests.

fn temp_plugin_file(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join("eep-engine-dsp-tests");
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join(name);
    std::fs::write(&file, b"").unwrap();
    std::fs::canonicalize(&file).unwrap()
}

fn load_payload(track_id: u64, path: &std::path::Path) -> Value {
    json!({
        "track_id": track_id,
        "format": "clap",
        "path": path.display().to_string(),
        "plugin_id": "org.eep.fixture.gain",
        "params": [{"param_id": 0, "value": 0.5}],
    })
}

// --- dsp.load_plugin --------------------------------------------------------

#[test]
fn load_plugin_records_slot_and_participates_in_undo() {
    let _guard = dsp_trust_gate();
    // SAFETY: serialized by the shared trust gate for the whole test body.
    unsafe {
        std::env::set_var("SATURN_PLUGIN_ROOT", std::env::temp_dir().join("eep-engine-dsp-tests"));
        std::env::set_var("HOME", std::env::temp_dir().join("eep-engine-dsp-tests-home"));
    }
    let plugin = temp_plugin_file("gain-chain-1.so");

    let mut engine = EditorEngine::default();
    let reply = engine
        .execute(CommandRequest::new("dsp.load_plugin", load_payload(1, &plugin)))
        .unwrap();
    assert_eq!(reply["added"], json!(true));
    assert_eq!(reply["slot_index"], json!(0));
    assert_eq!(reply["hosted"], json!(true));

    let track = &engine.document().project.sequence.tracks[0];
    assert_eq!(track.dsp_plugin_chain.len(), 1);
    let slot = &track.dsp_plugin_chain[0];
    assert_eq!(slot.format.as_tag(), "clap");
    assert_eq!(slot.plugin_id, "org.eep.fixture.gain");
    assert!(slot.enabled);
    assert_eq!(
        slot.params,
        vec![crate::project::DspPluginSlotParam { param_id: 0, value: 0.5 }]
    );

    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    assert!(
        engine.document().project.sequence.tracks[0]
            .dsp_plugin_chain
            .is_empty(),
        "loading a plugin participates in undo"
    );
    // SAFETY: cleanup under the same gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn load_plugin_rejects_unhosted_formats_up_front() {
    let _guard = dsp_trust_gate();
    let mut engine = EditorEngine::default();
    let error = engine
        .execute(CommandRequest::new(
            "dsp.load_plugin",
            json!({
                "track_id": 1,
                "format": "lv2",
                "path": "/anywhere/gain.so",
                "plugin_id": "org.some.lv2",
            }),
        ))
        .unwrap_err();
    let text = error.to_string();
    assert!(text.contains("not hosted"), "{text}");
    assert!(text.contains("clap"), "{text}");
    assert!(
        engine.document().project.sequence.tracks[0]
            .dsp_plugin_chain
            .is_empty(),
        "a rejected plugin must not be stored"
    );
}

#[test]
fn load_plugin_rejects_bad_paths_and_ids() {
    let _guard = dsp_trust_gate();
    // SAFETY: no SATURN_PLUGIN_ROOT trust must be configured here.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
    let mut engine = EditorEngine::default();

    // Wrong suffix: the file exists but is not a plugin library.
    let impostor = temp_plugin_file("not-a-plugin.txt");
    let error = engine
        .execute(CommandRequest::new(
            "dsp.load_plugin",
            json!({"track_id": 1, "format": "clap", "path": impostor.display().to_string(), "plugin_id": "x"}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("known suffix"), "{error}");

    // Path outside every trusted root (including dangling paths).
    let error = engine
        .execute(CommandRequest::new(
            "dsp.load_plugin",
            json!({"track_id": 1, "format": "clap", "path": "/nonexistent/gain.so", "plugin_id": "x"}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("not found"), "{error}");

    // Empty plugin id.
    let real_plugin = temp_plugin_file("gain-chain-2.so");
    // SAFETY: the opt-in root is needed only for the remaining checks.
    unsafe {
        std::env::set_var(
            "SATURN_PLUGIN_ROOT",
            std::env::temp_dir().join("eep-engine-dsp-tests"),
        )
    };
    let error = engine
        .execute(CommandRequest::new(
            "dsp.load_plugin",
            json!({"track_id": 1, "format": "clap", "path": real_plugin.display().to_string(), "plugin_id": "  "}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("non-empty"), "{error}");
    // SAFETY: cleanup under the same gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn load_plugin_rejects_out_of_range_param_values() {
    let _guard = dsp_trust_gate();
    let root = std::env::temp_dir().join("eep-engine-dsp-tests");
    let plugin = {
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("gain-chain-3.so");
        std::fs::write(&file, b"").unwrap();
        std::fs::canonicalize(&file).unwrap()
    };
    // SAFETY: serialized by the gate for the whole test body.
    unsafe { std::env::set_var("SATURN_PLUGIN_ROOT", root.display().to_string()) };

    let mut engine = EditorEngine::default();
    let error = engine
        .execute(CommandRequest::new(
            "dsp.load_plugin",
            json!({
                "track_id": 1, "format": "clap",
                "path": plugin.display().to_string(),
                "plugin_id": "org.eep.fixture.gain",
                "params": [{"param_id": 0, "value": 1.5}],
            }),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("0.0..=1.0"), "{error}");
    // SAFETY: cleanup under the same gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

// --- dsp.set_plugin_param ---------------------------------------------------

#[test]
fn set_plugin_param_updates_and_inserts_and_undoes() {
    let _guard = dsp_trust_gate();
    let root = std::env::temp_dir().join("eep-engine-dsp-tests");
    let plugin = {
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("gain-chain-4.so");
        std::fs::write(&file, b"").unwrap();
        std::fs::canonicalize(&file).unwrap()
    };
    // SAFETY: serialized by the gate for the whole test body.
    unsafe { std::env::set_var("SATURN_PLUGIN_ROOT", root.display().to_string()) };

    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("dsp.load_plugin", load_payload(1, &plugin)))
        .unwrap();

    // Existing parameter: value is replaced in place.
    let reply = engine
        .execute(CommandRequest::new(
            "dsp.set_plugin_param",
            json!({"track_id": 1, "slot_index": 0, "param_id": 0, "value": 0.0}),
        ))
        .unwrap();
    assert_eq!(reply["updated"], json!(true));
    assert_eq!(
        engine.document().project.sequence.tracks[0].dsp_plugin_chain[0].params[0].value,
        0.0
    );

    // New parameter: appended, keeping order stable for round-trips.
    engine
        .execute(CommandRequest::new(
            "dsp.set_plugin_param",
            json!({"track_id": 1, "slot_index": 0, "param_id": 7, "value": 1.0}),
        ))
        .unwrap();
    let params = &engine.document().project.sequence.tracks[0].dsp_plugin_chain[0].params;
    assert_eq!(params.len(), 2);
    assert_eq!(params[1].param_id, 7);

    // Undo: the last set (param 7 insert) is undone, not the whole chain.
    engine
        .execute(CommandRequest::new("edit.undo", Value::Null))
        .unwrap();
    let params = &engine.document().project.sequence.tracks[0].dsp_plugin_chain[0].params;
    assert_eq!(params.len(), 1, "only the last parameter change undoes");
    assert_eq!(params[0].value, 0.0);

    // SAFETY: cleanup under the same gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn set_plugin_param_validates_slot_and_value() {
    let _guard = dsp_trust_gate();
    let root = std::env::temp_dir().join("eep-engine-dsp-tests");
    let plugin = {
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("gain-chain-5.so");
        std::fs::write(&file, b"").unwrap();
        std::fs::canonicalize(&file).unwrap()
    };
    // SAFETY: serialized by the gate for the whole test body.
    unsafe { std::env::set_var("SATURN_PLUGIN_ROOT", root.display().to_string()) };

    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new("dsp.load_plugin", load_payload(1, &plugin)))
        .unwrap();

    let error = engine
        .execute(CommandRequest::new(
            "dsp.set_plugin_param",
            json!({"track_id": 1, "slot_index": 4, "param_id": 0, "value": 0.5}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("out of range"), "{error}");

    let error = engine
        .execute(CommandRequest::new(
            "dsp.set_plugin_param",
            json!({"track_id": 1, "slot_index": 0, "param_id": 0, "value": -0.1}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("0.0..=1.0"), "{error}");
    // SAFETY: cleanup under the same gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}

#[test]
fn track_chain_cap_is_enforced_at_the_command_layer() {
    let _guard = dsp_trust_gate();
    let root = std::env::temp_dir().join("eep-engine-dsp-tests");
    let plugin = {
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("gain-chain-6.so");
        std::fs::write(&file, b"").unwrap();
        std::fs::canonicalize(&file).unwrap()
    };
    // SAFETY: serialized by the gate for the whole test body.
    unsafe { std::env::set_var("SATURN_PLUGIN_ROOT", root.display().to_string()) };

    let mut engine = EditorEngine::default();
    for _ in 0..crate::project::MAX_TRACK_SLOTS {
        engine
            .execute(CommandRequest::new("dsp.load_plugin", load_payload(1, &plugin)))
            .unwrap();
    }
    let error = engine
        .execute(CommandRequest::new("dsp.load_plugin", load_payload(1, &plugin)))
        .unwrap_err();
    assert!(error.to_string().contains("limit 64"), "{error}");
    // SAFETY: cleanup under the same gate.
    unsafe { std::env::remove_var("SATURN_PLUGIN_ROOT") };
}
