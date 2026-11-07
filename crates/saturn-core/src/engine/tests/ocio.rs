// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Packet 3.1 colour-management command tests: pipeline configuration,
//! per-asset IDTs, and honest validation of unknown identifiers.

use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use crate::project::MediaKind;
use serde_json::json;

fn engine_with_media() -> (EditorEngine, u64) {
    let mut engine = EditorEngine::default();
    let response = engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "a.mov", "path": "/a.mov", "kind": MediaKind::Video}),
        ))
        .unwrap();
    let media_id = response["media_id"].as_u64().expect("media_id");
    (engine, media_id)
}

#[test]
fn set_pipeline_updates_project_state_and_echoes_selection() {
    let (mut engine, _) = engine_with_media();
    let response = engine
        .execute(CommandRequest::new(
            "ocio.set_pipeline",
            json!({
                "working_space": "aces_cg",
                "display_odt": "rec2020_pq",
                "default_idt": "arri_logc4",
            }),
        ))
        .unwrap();
    assert_eq!(response["workingSpace"], "aces_cg");
    assert_eq!(response["displayOdt"], "rec2020_pq");
    assert_eq!(response["defaultIdt"], "arri_logc4");
    let cm = &engine.document().project.color_management;
    assert_eq!(cm.working_space, crate::color::WorkingSpace::AcesCg);
    assert_eq!(cm.display_odt, crate::color::DisplayOdt::Rec2020Pq);
    assert_eq!(
        cm.default_idt,
        Some(crate::color::CameraIdt::ArriLogC4)
    );
}

#[test]
fn set_pipeline_accepts_cct_and_clears_default_idt_when_omitted() {
    let (mut engine, _) = engine_with_media();
    engine
        .execute(CommandRequest::new(
            "ocio.set_pipeline",
            json!({"working_space": "aces_cg", "display_odt": "rec709",
                   "default_idt": "sony_slog3"}),
        ))
        .unwrap();
    // Omitting default_idt on a later call must clear it, not keep stale state.
    let response = engine
        .execute(CommandRequest::new(
            "ocio.set_pipeline",
            json!({"working_space": "aces_cct", "display_odt": "display_p3"}),
        ))
        .unwrap();
    assert_eq!(response["workingSpace"], "aces_cct");
    assert_eq!(response["defaultIdt"], serde_json::Value::Null);
}

#[test]
fn set_pipeline_rejects_unknown_space_names_with_invalid_parameters() {
    let (mut engine, _) = engine_with_media();
    for payload in [
        json!({"working_space": "lin_rec709", "display_odt": "rec709"}),
        json!({"working_space": "aces_cg", "display_odt": "p3_d65_pq"}),
        json!({"working_space": "aces_cg", "display_odt": "rec709",
               "default_idt": "bmd_log3g12"}),
    ] {
        let error = engine
            .execute(CommandRequest::new("ocio.set_pipeline", payload))
            .expect_err("unknown identifier must fail");
        assert!(
            matches!(
                error,
                crate::engine::DispatchError::InvalidParameters(_)
            ),
            "unexpected error: {error:?}"
        );
    }
}

#[test]
fn set_media_idt_sets_asset_idt() {
    let (mut engine, media_id) = engine_with_media();
    let response = engine
        .execute(CommandRequest::new(
            "ocio.set_media_idt",
            json!({"media_id": media_id, "idt": "blackmagic_gen5"}),
        ))
        .unwrap();
    assert_eq!(response["mediaId"], media_id);
    assert_eq!(response["idt"], "blackmagic_gen5");
    assert_eq!(
        engine.document().project.media[0].idt,
        Some(crate::color::CameraIdt::BlackmagicGen5)
    );
}

#[test]
fn set_media_idt_unknown_media_id_is_a_clean_error() {
    let (mut engine, _) = engine_with_media();
    let error = engine
        .execute(CommandRequest::new(
            "ocio.set_media_idt",
            json!({"media_id": 999, "idt": "apple_log"}),
        ))
        .expect_err("unknown media id must fail");
    assert!(matches!(
        error,
        crate::engine::DispatchError::InvalidParameters(_)
    ));
}

#[test]
fn new_media_imports_inherit_project_default_idt() {
    let (mut engine, _) = engine_with_media();
    engine
        .execute(CommandRequest::new(
            "ocio.set_pipeline",
            json!({"working_space": "aces_cg", "display_odt": "rec709",
                   "default_idt": "red_log3g10"}),
        ))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "b.r3d", "path": "/b.r3d", "kind": MediaKind::Video}),
        ))
        .unwrap();
    assert_eq!(
        engine.document().project.media[1].idt,
        Some(crate::color::CameraIdt::RedLog3G10),
        "imports after the pipeline call must seed the default IDT"
    );
}

#[test]
fn serde_names_match_as_str_for_every_selection() {
    // The serde representation is persisted in project files *and* parsed from
    // command params; it must stay identical to the documented `as_str` names.
    for idt in crate::color::CameraIdt::ALL {
        let decoded: crate::color::CameraIdt =
            serde_json::from_value(json!(idt.as_str())).expect("as_str must deserialize");
        assert_eq!(idt, &decoded);
        assert_eq!(
            serde_json::to_value(idt).unwrap(),
            json!(idt.as_str()),
            "serde name for {:?}",
            idt
        );
    }
    for space in [crate::color::WorkingSpace::AcesCg, crate::color::WorkingSpace::AcesCct] {
        assert_eq!(
            serde_json::to_value(space).unwrap(),
            json!(space.as_str()),
            "serde name for {:?}",
            space
        );
    }
    for odt in [
        crate::color::DisplayOdt::Rec709,
        crate::color::DisplayOdt::DisplayP3,
        crate::color::DisplayOdt::Rec2020Pq,
    ] {
        assert_eq!(
            serde_json::to_value(odt).unwrap(),
            json!(odt.as_str()),
            "serde name for {:?}",
            odt
        );
    }
}
