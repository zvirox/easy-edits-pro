// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use crate::command::CommandRequest;
use crate::engine::EditorEngine;
use serde_json::{json, Value};

fn engine_with_media() -> EditorEngine {
    let mut engine = EditorEngine::default();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "a.mp4", "path": "/a.mp4", "kind": crate::project::MediaKind::Video}),
        ))
        .unwrap();
    engine
}

fn make_clip(engine: &mut EditorEngine) {
    let media_id = engine.document().project.media[0].id;
    engine
        .execute(CommandRequest::new(
            "timeline.add_clip",
            json!({"media_id": media_id, "track_id": 1, "timeline_start": 0}),
        ))
        .unwrap();
}

// --- Branch lifecycle -------------------------------------------------------

#[test]
fn branch_names_are_validated_and_unique() {
    let mut engine = engine_with_media();
    let error = engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "   "})))
        .unwrap_err();
    assert!(error.to_string().contains("cannot be empty"));

    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "fidget"})))
        .unwrap();
    let error = engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "fidget"})))
        .unwrap_err();
    assert!(error.to_string().contains("already exists"), "{error}");

    let error = engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "main"})))
        .unwrap_err();
    assert!(error.to_string().contains("reserved"), "{error}");
}

#[test]
fn branch_create_snapshots_without_disturbing_the_live_state() {
    let mut engine = engine_with_media();
    make_clip(&mut engine);
    let clips_before = engine.document().project.sequence.tracks[0].clips.len();

    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "stash"})))
        .unwrap();

    assert_eq!(clips_before, engine.document().project.sequence.tracks[0].clips.len());
    let listed = engine
        .execute(CommandRequest::new("project.branch_list", json!({})))
        .unwrap();
    assert_eq!(listed["active"], json!("main"));
    assert_eq!(listed["branches"][0]["name"], json!("stash"));
    assert_eq!(listed["branches"][0]["parent"], json!("main"));
}

#[test]
fn switch_stores_work_on_the_current_branch_and_restores_the_target() {
    let mut engine = engine_with_media();
    make_clip(&mut engine);
    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "alt"})))
        .unwrap();

    // Further main-line work after the fork.
    make_clip(&mut engine);
    let main_clips = engine.document().project.sequence.tracks[0].clips.len();
    assert!(main_clips >= 1);

    engine
        .execute(CommandRequest::new("project.branch_switch", json!({"name": "alt"})))
        .unwrap();
    // The fork was created before the second edit, so the alt branch has at
    // least one fewer clip than main.
    assert!(
        engine.document().project.sequence.tracks[0].clips.len() < main_clips,
        "switch must restore the forked snapshot"
    );
    let listed = engine
        .execute(CommandRequest::new("project.branch_list", json!({})))
        .unwrap();
    assert_eq!(listed["active"], json!("alt"));
    // The record set now carries the state main had at switch-away time.
    assert!(listed["branches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|branch| branch["name"] == json!("main")));

    // Switching back keeps the recent main-line work.
    engine
        .execute(CommandRequest::new("project.branch_switch", json!({"name": "main"})))
        .unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips.len(),
        main_clips
    );
    // The viewport records the active branch (Packet 0.1 field is now real).
    let inspect = engine
        .execute(CommandRequest::new("viewport.inspect", json!({})))
        .unwrap();
    assert_eq!(inspect["viewport"]["active_branch"], json!("main"));
}

#[test]
fn diff_reports_clip_changes_between_branches() {
    let mut engine = engine_with_media();
    make_clip(&mut engine);
    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "alt"})))
        .unwrap();
    make_clip(&mut engine); // main gains a second clip

    let report = engine
        .execute(CommandRequest::new("project.branch_diff", json!({"branch": "alt"})))
        .unwrap();
    // The report is the branch (old snapshot) versus live state: the second
    // clip appears as an addition.
    assert!(!report["identical"].as_bool().unwrap());
    assert!(
        report["sequences"][0]["tracks_matched"].as_bool().unwrap(),
        "sequence report: {report}"
    );
    let added = report["sequences"][0]["clip_changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|change| change["kind"] == json!("added"));
    assert!(added, "branch diff must show the added clip: {report}");
}

#[test]
fn merge_accept_absorbs_the_branch_and_is_undoable() {
    let mut engine = engine_with_media();
    make_clip(&mut engine);
    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "alt"})))
        .unwrap();
    make_clip(&mut engine); // extra main work that accept will discard

    let main_clips = engine.document().project.sequence.tracks[0].clips.len();
    // Merging without force fails because main has diverged (stale baseline).
    let stale_err = engine
        .execute(CommandRequest::new(
            "project.branch_merge",
            json!({"branch": "alt", "action": "accept"}),
        ))
        .unwrap_err();
    assert!(stale_err.to_string().contains("Baseline is stale"), "{stale_err}");

    // Merging with force: true overrides the stale baseline check and discards extra main work.
    let merged = engine
        .execute(CommandRequest::new(
            "project.branch_merge",
            json!({"branch": "alt", "action": "accept", "force": true}),
        ))
        .unwrap();
    assert_eq!(merged["merged"], json!("accept"));
    // Live state is now the branch snapshot (one fewer clip than main had).
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips.len(),
        main_clips - 1
    );
    // The absorbed branch record is gone.
    let listed = engine
        .execute(CommandRequest::new("project.branch_list", json!({})))
        .unwrap();
    assert_eq!(listed["branches"].as_array().unwrap().len(), 0);

    // Undo restores both the pre-merge state and the branch record.
    engine.execute(CommandRequest::new("edit.undo", Value::Null)).unwrap();
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips.len(),
        main_clips
    );
    assert!(
        !engine.document().branches.is_empty(),
        "merging participates in undo"
    );
}

#[test]
fn merge_reject_discards_the_branch_without_touching_live_work() {
    let mut engine = engine_with_media();
    make_clip(&mut engine);
    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "alt"})))
        .unwrap();
    make_clip(&mut engine);

    let main_clips = engine.document().project.sequence.tracks[0].clips.len();
    let merged = engine
        .execute(CommandRequest::new(
            "project.branch_merge",
            json!({"branch": "alt", "action": "reject"}),
        ))
        .unwrap();
    assert_eq!(merged["merged"], json!("reject"));
    assert_eq!(
        engine.document().project.sequence.tracks[0].clips.len(),
        main_clips
    );
    assert!(engine.document().branches.is_empty());
}

#[test]
fn merge_refuses_top_level_errors() {
    let mut engine = engine_with_media();
    make_clip(&mut engine);

    let error = engine
        .execute(CommandRequest::new(
            "project.branch_merge",
            json!({"branch": "nothere", "action": "accept"}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("does not exist"));

    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "alt"})))
        .unwrap();
    // A merge must not run against the branch you are standing on.
    engine
        .execute(CommandRequest::new("project.branch_switch", json!({"name": "alt"})))
        .unwrap();
    let error = engine
        .execute(CommandRequest::new(
            "project.branch_merge",
            json!({"branch": "alt", "action": "accept"}),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("active branch"), "{error}");
}

#[test]
fn switch_never_loses_work_on_a_stale_active_branch() {
    let mut engine = engine_with_media();
    make_clip(&mut engine);
    let main_clips = engine.document().project.sequence.tracks[0].clips.len();

    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "alt"})))
        .unwrap();
    // Create then edit more, so main differs from the alt snapshot.
    make_clip(&mut engine);
    engine
        .execute(CommandRequest::new("project.branch_switch", json!({"name": "alt"})))
        .unwrap();
    engine
        .execute(CommandRequest::new("project.branch_switch", json!({"name": "main"})))
        .unwrap();
    assert!(
        engine.document().project.sequence.tracks[0].clips.len() >= main_clips,
        "switching back must keep main-line work: {} >= {main_clips}",
        engine.document().project.sequence.tracks[0].clips.len()
    );
}

// --- Fingerprint-driven relink suggestions ----------------------------------

#[test]
fn fingerprint_media_records_head_tail_size_and_never_invents_content() {
    use std::io::Write as _;
    let mut engine = engine_with_media();
    let dir = tempfile::tempdir().unwrap();
    let media_path = dir.path().join("clip.mov");
    std::fs::File::create(&media_path)
        .and_then(|mut file| file.write_all(b"real media bytes here"))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "clip.mov", "path": media_path.to_string_lossy(), "kind": "video"}),
        ))
        .unwrap();

    let result = engine
        .execute(CommandRequest::new("project.fingerprint_media", json!({})))
        .unwrap();
    assert_eq!(result["fingerprinted"], json!(1));
    let media = &engine.document().project.media[1];
    let fingerprint = media.fingerprint.as_ref().unwrap();
    assert_eq!(fingerprint.size, "real media bytes here".len() as u64);
    assert_eq!(fingerprint.head, fingerprint.tail);
}

#[test]
fn relink_suggestion_cannot_rel_inkin_silently() {
    use std::io::Write as _;
    let mut engine = engine_with_media();
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("clip.mov");
    std::fs::File::create(&original)
        .and_then(|mut file| file.write_all(b"content worth suggesting"))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "clip.mov", "path": original.to_string_lossy(), "kind": "video"}),
        ))
        .unwrap()
        ;
    // Fingerprint while the file exists.
    engine
        .execute(CommandRequest::new("project.fingerprint_media", json!({})))
        .unwrap();

    // The file disappears (moved).
    let moved_dir = tempfile::tempdir().unwrap();
    let moved_path = moved_dir.path().join("clip.mov");
    std::fs::rename(&original, &moved_path).unwrap();

    let media_id = engine.document().project.media[1].id;
    let suggestions = engine
        .execute(CommandRequest::new(
            "media.relink_suggest",
            json!({"search_dirs": [moved_dir.path().to_string_lossy()]}),
        ))
        .unwrap();
    // The synthetic /a.mp4 also reports missing but has no candidates; only
    // clip.mov carries real suggestion candidates.
    assert_eq!(suggestions["suggested"], json!(1));
    let entry = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["candidates"].as_array().is_some_and(|c| !c.is_empty()))
        .unwrap();
    assert_eq!(entry["media_id"], json!(media_id));
    assert_eq!(entry["candidates"][0]["path"], json!(moved_path.to_string_lossy()));
    assert_eq!(entry["candidates"][0]["match"], json!("fingerprint"));
    // The suggestion itself must not relink anything.
    assert!(
        !engine.document().project.media[1].path.exists(),
        "suggestions are read-only"
    );

    // The user confirms through the existing relink command.
    engine
        .execute(CommandRequest::new(
            "project.relink_media",
            json!({"media_id": media_id, "path": moved_path.to_string_lossy()}),
        ))
        .unwrap();
    assert!(engine.document().project.media[1].path.is_file());
}

#[test]
fn relink_suggestion_skips_missing_fingerprints_and_still_lists_name_matches() {
    use std::io::Write as _;
    let mut engine = engine_with_media();
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("clip.mov");
    std::fs::File::create(&original)
        .and_then(|mut file| file.write_all(b"no fingerprint recorded"))
        .unwrap();
    engine
        .execute(CommandRequest::new(
            "project.add_media",
            json!({"name": "clip.mov", "path": original.to_string_lossy(), "kind": "video"}),
        ))
        .unwrap();

    let moved_dir = tempfile::tempdir().unwrap();
    let moved_path = moved_dir.path().join("clip.mov");
    std::fs::File::create(&moved_path)
        .and_then(|mut file| file.write_all(b"no fingerprint recorded"))
        .unwrap();
    std::fs::remove_file(&original).unwrap();

    let suggestions = engine
        .execute(CommandRequest::new(
            "media.relink_suggest",
            json!({"search_dirs": [moved_dir.path().to_string_lossy()]}),
        ))
        .unwrap();
    let entry = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["candidates"].as_array().is_some_and(|c| !c.is_empty()))
        .unwrap();
    assert_eq!(entry["candidates"][0]["match"], json!("name"));
}

/// Regression: the parked-main record must never make the document
/// unsavable. `main` is a reserved record name the engine may write when
/// switching away from the live branch; user creation stays forbidden.
#[test]
fn switching_branches_keeps_the_document_savable() {
    let mut engine = engine_with_media();
    make_clip(&mut engine);
    engine
        .execute(CommandRequest::new("project.branch_create", json!({"name": "alt"})))
        .unwrap();
    engine
        .execute(CommandRequest::new("project.branch_switch", json!({"name": "alt"})))
        .unwrap();
    engine
        .execute(CommandRequest::new("project.branch_switch", json!({"name": "main"})))
        .unwrap();

    let doc = engine.document().clone();
    doc.validate().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cut.saturn");
    doc.save_atomic(&path).unwrap();
    let loaded = crate::project::ProjectDocument::load(&path).unwrap();
    assert!(loaded.branches.iter().any(|branch| branch.name == "main"));
    assert!(loaded.branches.iter().any(|branch| branch.name == "alt"));
}
