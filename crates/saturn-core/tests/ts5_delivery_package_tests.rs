// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Integration and verification tests for TS5: Delivery packaging (`delivery.package`).

use std::fs;
use std::path::Path;

use saturn_core::actor::ActorContext;
use saturn_core::brief::ConfidenceLevel;
use saturn_core::catalogue::{find_command, PolicyError, SessionMode, SessionPolicy, Tier};
use saturn_core::interop::{import_fcpxml, import_otio};
use saturn_core::project::{CaptionTrack, MediaKind, ProjectDocument, SubtitleItem, TrackKind};
use saturn_core::tasksheet::delivery::{
    compute_blake3_hex, compute_sha256_hex, inspect_delivery_package_integrity,
    verify_delivery_package, AttributedChangeLog, DeliveryManifest, DeliveryProvenance,
    DELIVERY_MANIFEST_SCHEMA_VERSION,
};
use saturn_core::tasksheet::SheetStatus;
use saturn_core::verification::VerifyReport;
use saturn_core::{CommandRequest, EditorEngine, TICKS_PER_SECOND};
use serde_json::json;

fn setup_approved_project_with_history() -> (EditorEngine, ActorContext, ActorContext, u64) {
    let mut engine = EditorEngine::default();
    let human_ctx = ActorContext::human();
    let agent_ctx = ActorContext::agent("agent-editor", "session-ts5");

    engine
        .execute_with_context(
            CommandRequest::new("project.new", json!({ "name": "TS5 Podcast Recap" })),
            &human_ctx,
        )
        .expect("project.new");

    let media_res = engine
        .execute_with_context(
            CommandRequest::new(
                "project.add_media",
                json!({
                    "path": "/tmp/ts5_main_interview.mp4",
                    "name": "Interview_CamA.mp4",
                    "kind": "video",
                    "duration": 30 * TICKS_PER_SECOND,
                }),
            ),
            &human_ctx,
        )
        .expect("project.add_media");

    let media_id = media_res
        .get("media_id")
        .and_then(|v| v.as_u64())
        .or_else(|| engine.document().project.media.first().map(|m| m.id))
        .expect("media_id");

    let video_track_id = engine
        .document()
        .project
        .sequence
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Video)
        .map(|t| t.id)
        .expect("video track");

    // Add initial clip as human
    engine
        .execute_with_context(
            CommandRequest::new(
                "timeline.add_clip",
                json!({
                    "track_id": video_track_id,
                    "media_id": media_id,
                    "timeline_start": 0,
                    "source_in": 0,
                    "duration": 4 * TICKS_PER_SECOND,
                }),
            ),
            &human_ctx,
        )
        .expect("human clip 1");

    // Seed caption track so provenance tracks untrusted media text
    let mut doc = engine.document().clone();
    doc.project.sequence.caption_tracks.push(CaptionTrack {
        id: 50,
        name: "Auto Captions".into(),
        language: "en".into(),
        visible: true,
        cues: vec![SubtitleItem {
            id: 1,
            start_ticks: saturn_core::time::Tick(0),
            end_ticks: saturn_core::time::Tick(2 * TICKS_PER_SECOND),
            text: "Welcome to our episode".into(),
            speaker: Some("Host".into()),
            style_id: None,
        }],
        styles: Vec::new(),
    });
    engine.restore_recovery(doc, None).expect("restore captions");

    // Create task sheet with inputs, deliverables, handoff expectations
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.create",
                json!({
                    "id": "sheet_ts5",
                    "title": "Podcast Recap Cut",
                    "requester": "human:producer",
                    "inputs": [
                        { "id": "cam_a", "media": "/tmp/ts5_main_interview.mp4", "role": "main_video", "trust": "trusted" },
                        { "id": "cap_auto", "media": "captions:50", "role": "captions", "trust": "untrusted" }
                    ],
                    "deliverables": [{
                        "id": "main_16x9",
                        "name": "Main Highlight",
                        "preset": "youtube-1080p",
                        "duration": { "target_s": 8.0, "tolerance_s": 2.0 },
                        "loudness_lufs": -14.0,
                        "true_peak_dbtp": -1.0,
                        "captions": true,
                        "must_include": ["hook"]
                    }],
                    "handoff": {
                        "exports": true,
                        "project_file": true,
                        "interchange": ["fcpxml", "otio"],
                        "qc_report": true,
                        "change_log": true
                    }
                }),
            ),
            &human_ctx,
        )
        .expect("tasksheet.create");

    engine
        .execute_with_context(CommandRequest::new("tasksheet.submit", json!({})), &human_ctx)
        .expect("tasksheet.submit");

    // Agent proposes plan, human approves plan
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.plan_submit",
                json!({
                    "steps": [
                        { "id": 1, "title": "Append B-roll highlight", "status": "pending", "target_tick": 4 * TICKS_PER_SECOND }
                    ]
                }),
            ),
            &agent_ctx,
        )
        .expect("plan_submit");
    engine
        .execute_with_context(
            CommandRequest::new("tasksheet.plan_approve", json!({ "reason": "Approved plan" })),
            &human_ctx,
        )
        .expect("plan_approve");

    // Agent adds second clip with rationale + confidence
    let mut add_req = CommandRequest::new(
        "timeline.add_clip",
        json!({
            "track_id": video_track_id,
            "media_id": media_id,
            "timeline_start": 4 * TICKS_PER_SECOND,
            "source_in": 5 * TICKS_PER_SECOND,
            "duration": 4 * TICKS_PER_SECOND,
        }),
    )
    .with_rationale("Place highlight segment after intro")
    .with_confidence(ConfidenceLevel::High);
    add_req.rationale = Some("Place highlight segment after intro".into());

    engine
        .execute_with_context(add_req, &agent_ctx)
        .expect("agent add clip");

    // Attach clip-level agent note & human correction provenance on clip 2
    let clip_id = engine.document().project.sequence.tracks[0].clips[1].id;
    let mut doc_with_note = engine.document().clone();
    let mut clip_note = saturn_core::brief::ClipAgentNote::new(
        "Selected key quote",
        ConfidenceLevel::Medium,
        saturn_core::actor::ActorId::Agent("agent-editor".into()),
        "session-ts5",
        1_700_000_000_000,
        42,
    )
    .expect("clip agent note");
    clip_note.record_correction(
        saturn_core::actor::ActorId::Human,
        1_700_000_001_000,
        "Tightened outro frame by 2f",
    );
    doc_with_note.project.sequence.tracks[0].clips[1].agent_note = Some(clip_note);
    engine
        .restore_recovery(doc_with_note, None)
        .expect("attach clip agent note");

    // Open review round, add & resolve note, then human approves cut
    engine
        .execute_with_context(
            CommandRequest::new("review.round_open", json!({ "label": "v1" })),
            &human_ctx,
        )
        .expect("review.round_open");
    let note_res = engine
        .execute_with_context(
            CommandRequest::new(
                "review.note_add",
                json!({
                    "text": "Verify outro tail pacing",
                    "target_tick": 6 * TICKS_PER_SECOND,
                    "clip_id": clip_id
                }),
            ),
            &human_ctx,
        )
        .expect("review.note_add");
    let note_id = note_res["note"]["id"].as_str().expect("note_id").to_string();

    engine
        .execute_with_context(
            CommandRequest::new(
                "review.round_close",
                json!({ "outcome": "revision_requested", "reason": "Address outro note" }),
            ),
            &human_ctx,
        )
        .expect("review.round_close revision_requested");

    engine
        .execute_with_context(
            CommandRequest::new(
                "review.note_resolve",
                json!({
                    "note_id": note_id,
                    "resolution": "Trimmed tail clean before outro beat",
                    "status": "resolved"
                }),
            ),
            &agent_ctx,
        )
        .expect("review.note_resolve");

    engine
        .execute_with_context(
            CommandRequest::new("review.round_open", json!({ "label": "v2" })),
            &agent_ctx,
        )
        .expect("review.round_open v2");

    engine
        .execute_with_context(
            CommandRequest::new(
                "review.round_close",
                json!({ "outcome": "approved", "reason": "Ready for delivery" }),
            ),
            &human_ctx,
        )
        .expect("review.round_close approved");

    assert_eq!(
        engine
            .document()
            .project
            .task_sheet
            .as_ref()
            .unwrap()
            .status,
        SheetStatus::Approved
    );

    (engine, human_ctx, agent_ctx, media_id)
}

#[test]
fn delivery_package_policy_requires_filesystem_tier_and_allowed_profile() {
    let spec = find_command("delivery.package").expect("delivery.package in catalogue");
    assert!(spec.tiers.contains(&Tier::Filesystem));
    assert!(spec.profiles.contains(&"tasksheet"));
    assert!(spec.profiles.contains(&"export"));

    // Default autopilot has tasksheet/rough-cut without Filesystem tier
    let mut no_fs = SessionPolicy::new(
        SessionMode::Autopilot,
        &["tasksheet"],
        &[Tier::Read, Tier::Edit],
    );
    match no_fs.check_command(spec) {
        Err(PolicyError::PermissionDenied { required_tier, .. }) => {
            assert_eq!(required_tier, Tier::Filesystem);
        }
        other => panic!("expected PermissionDenied for missing Filesystem tier, got {other:?}"),
    }

    // Read-only review session cannot execute delivery.package
    assert!(matches!(
        SessionPolicy::review().check_command(spec),
        Err(PolicyError::ReadOnlySession { .. })
    ));

    // Filesystem grant without tasksheet/export/admin profile fails ProfileRequired
    let fs_wrong_profile = SessionPolicy::new(
        SessionMode::Autopilot,
        &["rough-cut"],
        &[Tier::Read, Tier::Edit, Tier::Filesystem],
    );
    assert!(matches!(
        fs_wrong_profile.check_command(spec),
        Err(PolicyError::ProfileRequired { .. })
    ));

    // Loading `tasksheet` or `export` profile with Filesystem grant succeeds
    assert!(no_fs.load_profile("export").is_ok());
    let fs_tasksheet = SessionPolicy::new(
        SessionMode::Autopilot,
        &["tasksheet"],
        &[Tier::Read, Tier::Edit, Tier::Filesystem],
    );
    fs_tasksheet.check_command(spec).expect("granted session allowed");

    let verify_spec = find_command("delivery.verify").expect("delivery.verify in catalogue");
    assert!(verify_spec.tiers.contains(&Tier::Filesystem));
    let fs_review = SessionPolicy::new(
        SessionMode::Review,
        &["review"],
        &[Tier::Read, Tier::Filesystem],
    );
    fs_review
        .check_command(verify_spec)
        .expect("review + filesystem can verify delivery");
}

#[test]
fn delivery_package_assembles_all_artifacts_and_passes_checksum_verification() {
    let (mut engine, _human_ctx, agent_ctx, _media_id) = setup_approved_project_with_history();

    let tmp = tempfile::tempdir().expect("tempdir");
    let rendered_src = tmp.path().join("main_16x9_master.mp4");
    let fake_media_bytes = b"ftypisom-ts5-rendered-deliverable-bytes";
    fs::write(&rendered_src, fake_media_bytes).expect("write rendered export");

    let pkg_dir = tmp.path().join("ts5-handoff-bundle");

    let res = engine
        .execute_with_context(
            CommandRequest::new(
                "delivery.package",
                json!({
                    "path": pkg_dir.to_string_lossy(),
                    "exports": [{
                        "path": rendered_src.to_string_lossy(),
                        "deliverable_id": "main_16x9",
                        "label": "Main Highlight Master"
                    }],
                    "require_approved": true,
                    "require_rendered_exports": true,
                }),
            )
            .with_rationale("Package final approved cut"),
            &agent_ctx,
        )
        .expect("delivery.package should succeed on approved sheet");

    assert_eq!(res["packaged"], json!(true));
    assert_eq!(res["verified"], json!(true));
    assert_eq!(res["transitioned_to_delivered"], json!(true));
    assert_eq!(res["task_sheet_status"], json!("delivered"));
    assert_eq!(
        engine
            .document()
            .project
            .task_sheet
            .as_ref()
            .unwrap()
            .status,
        SheetStatus::Delivered
    );

    // Verify checksums independently (both helper and command)
    let verification = verify_delivery_package(&pkg_dir).expect("verify_delivery_package");
    assert!(verification.valid);
    assert!(verification.blake3_verified);
    assert!(verification.sha256_verified);
    assert!(verification.manifest_checksum_verified);
    assert!(verification.checksum_file_blake3_present);
    assert!(verification.checksum_file_sha256_present);
    assert!(verification.checked_files >= 7);

    let cmd_verify = engine
        .execute_with_context(
            CommandRequest::new(
                "delivery.verify",
                json!({ "path": pkg_dir.to_string_lossy() }),
            ),
            &agent_ctx,
        )
        .expect("delivery.verify command");
    assert_eq!(cmd_verify["verified"], json!(true));

    // Check manifest & checksums match exact file contents
    let manifest_bytes = fs::read(pkg_dir.join("manifest.json")).expect("read manifest.json");
    let manifest: DeliveryManifest =
        serde_json::from_slice(&manifest_bytes).expect("parse DeliveryManifest");
    assert_eq!(manifest.schema, DELIVERY_MANIFEST_SCHEMA_VERSION);
    assert_eq!(manifest.task_sheet_id.as_deref(), Some("sheet_ts5"));
    assert_eq!(manifest.task_sheet_status.as_deref(), Some("delivered"));
    assert_eq!(manifest.staged_exports_count, 1);
    assert_eq!(manifest.interchange_formats, vec!["fcpxml", "otio"]);
    assert!(manifest.qc_summary.is_some());

    // 1. Staged export bytes & hashes
    let packaged_export = pkg_dir.join("exports/main_16x9_master.mp4");
    assert_eq!(fs::read(&packaged_export).expect("read export"), fake_media_bytes);
    let export_entry = manifest
        .files
        .iter()
        .find(|f| f.relative_path == "exports/main_16x9_master.mp4")
        .expect("export manifest entry");
    assert_eq!(export_entry.blake3, compute_blake3_hex(fake_media_bytes));
    assert_eq!(export_entry.sha256, compute_sha256_hex(fake_media_bytes));
    assert_eq!(export_entry.deliverable_id.as_deref(), Some("main_16x9"));

    // 2. Project file round-trips via ProjectDocument::load
    let project_entry = manifest
        .files
        .iter()
        .find(|f| f.role == "project_file")
        .expect("project_file entry");
    let loaded_doc = ProjectDocument::load(&pkg_dir.join(&project_entry.relative_path))
        .expect("packaged .saturn reloads cleanly");
    assert_eq!(loaded_doc.project.name, "TS5 Podcast Recap");
    assert_eq!(
        loaded_doc.project.task_sheet.as_ref().unwrap().status,
        SheetStatus::Delivered
    );

    // 3. Valid FCPXML & OTIO interchange files parse cleanly back
    let fcpxml_entry = manifest
        .files
        .iter()
        .find(|f| f.role == "interchange" && f.format == "fcpxml")
        .expect("fcpxml entry");
    let fcpxml_text =
        fs::read_to_string(pkg_dir.join(&fcpxml_entry.relative_path)).expect("read fcpxml");
    let reimported_fcpxml = import_fcpxml(&fcpxml_text).expect("valid FCPXML");
    assert!(
        !reimported_fcpxml.project.media.is_empty(),
        "FCPXML retains media resources"
    );

    let otio_entry = manifest
        .files
        .iter()
        .find(|f| f.role == "interchange" && f.format == "otio")
        .expect("otio entry");
    let otio_text = fs::read_to_string(pkg_dir.join(&otio_entry.relative_path)).expect("read otio");
    let reimported_otio = import_otio(&otio_text, "Reimported OTIO").expect("valid OTIO");
    assert!(
        reimported_otio.project.sequence.tracks[0].clips.len() >= 2,
        "OTIO retains both sequence clips"
    );

    // 4. Complete QC report from verify.run
    let qc_json =
        fs::read_to_string(pkg_dir.join("qc/verify-report.json")).expect("read verify-report.json");
    let qc_report: VerifyReport = serde_json::from_str(&qc_json).expect("parse VerifyReport");
    assert!(qc_report.summary.total_checks > 0);
    assert_eq!(
        qc_report.summary.total_checks,
        manifest.qc_summary.as_ref().unwrap().total_checks
    );

    // 5. Attributed change log & markdown summary
    let changelog_json =
        fs::read_to_string(pkg_dir.join("changelog/changelog.json")).expect("read changelog.json");
    let changelog: AttributedChangeLog =
        serde_json::from_str(&changelog_json).expect("parse AttributedChangeLog");
    assert!(changelog.total_changes > 0);
    assert!(changelog.ai_changes > 0);
    assert!(changelog.human_changes > 0);
    assert!(changelog
        .records
        .iter()
        .any(|r| r.rationale.as_deref() == Some("Place highlight segment after intro")));
    assert_eq!(changelog.review_rounds.len(), 2);
    assert!(changelog
        .transitions
        .iter()
        .any(|t| t.to == SheetStatus::Delivered));
    assert_eq!(changelog.clip_notes.len(), 1);
    assert_eq!(
        changelog.clip_notes[0].human_correction_note.as_deref(),
        Some("Tightened outro frame by 2f")
    );
    assert!(pkg_dir.join("changelog/CHANGELOG.md").is_file());

    // 6. Provenance metadata
    let prov_json = fs::read_to_string(pkg_dir.join("provenance/provenance.json"))
        .expect("read provenance.json");
    let provenance: DeliveryProvenance =
        serde_json::from_str(&prov_json).expect("parse DeliveryProvenance");
    assert_eq!(provenance.task_sheet_inputs.len(), 2);
    assert!(provenance.untrusted_origin_count > 0);
    assert_eq!(provenance.clip_attribution.len(), 1);
    assert_eq!(provenance.media_assets.len(), 1);
    assert_eq!(MediaKind::Video, engine.document().project.media[0].kind);
}

#[test]
fn delivery_checksum_verification_detects_tampering_missing_and_extra_files() {
    let (mut engine, _human_ctx, agent_ctx, _media_id) = setup_approved_project_with_history();
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg_dir = tmp.path().join("tamper-check-bundle");

    engine
        .execute_with_context(
            CommandRequest::new(
                "delivery.package",
                json!({ "path": pkg_dir.to_string_lossy() }),
            ),
            &agent_ctx,
        )
        .expect("delivery.package");

    let manifest: DeliveryManifest = serde_json::from_slice(
        &fs::read(pkg_dir.join("manifest.json")).expect("read manifest"),
    )
    .expect("parse manifest");
    let fcpxml_rel = manifest
        .files
        .iter()
        .find(|f| f.role == "interchange" && f.format == "fcpxml")
        .unwrap()
        .relative_path
        .clone();
    let fcpxml_path = pkg_dir.join(&fcpxml_rel);
    let orig_fcpxml = fs::read(&fcpxml_path).expect("read original fcpxml");

    // 1. Tamper artifact file contents
    let mut corrupted = orig_fcpxml.clone();
    corrupted.push_str_or_flip();
    fs::write(&fcpxml_path, &corrupted).expect("corrupt fcpxml");

    let report = inspect_delivery_package_integrity(&pkg_dir).expect("inspect tampered");
    assert!(!report.valid);
    assert!(!report.blake3_verified);
    assert!(!report.sha256_verified);
    assert!(verify_delivery_package(&pkg_dir).is_err());
    assert!(engine
        .execute_with_context(
            CommandRequest::new(
                "delivery.verify",
                json!({ "path": pkg_dir.to_string_lossy() }),
            ),
            &agent_ctx,
        )
        .is_err());

    // Restore .fcpxml
    fs::write(&fcpxml_path, &orig_fcpxml).expect("restore fcpxml");
    verify_delivery_package(&pkg_dir).expect("restored verifies");

    // 2. Tamper manifest.json itself without updating CHECKSUMS.blake3 / SHA256SUMS
    let orig_manifest = fs::read(&pkg_dir.join("manifest.json")).expect("orig manifest");
    let mut bad_manifest = String::from_utf8(orig_manifest.clone()).unwrap();
    bad_manifest = bad_manifest.replacen("TS5 Podcast Recap", "Tampered Project", 1);
    fs::write(pkg_dir.join("manifest.json"), bad_manifest).expect("tamper manifest");
    let manifest_tamper_report = inspect_delivery_package_integrity(&pkg_dir).unwrap();
    assert!(!manifest_tamper_report.valid);
    assert!(!manifest_tamper_report.manifest_checksum_verified);
    fs::write(pkg_dir.join("manifest.json"), orig_manifest).expect("restore manifest");

    // 3. Delete an artifact file
    let qc_path = pkg_dir.join("qc/verify-report.json");
    let orig_qc = fs::read(&qc_path).expect("orig qc");
    fs::remove_file(&qc_path).expect("delete qc report");
    assert!(verify_delivery_package(&pkg_dir).is_err());
    fs::write(&qc_path, orig_qc).expect("restore qc");

    // 4. Drop an unexpected unlisted file inside interchange/
    let rogue_path = pkg_dir.join("interchange/rogue.txt");
    fs::write(&rogue_path, b"unlisted").expect("write rogue file");
    assert!(verify_delivery_package(&pkg_dir).is_err());
    fs::remove_file(&rogue_path).expect("remove rogue file");

    // Passes again after cleanup
    assert!(verify_delivery_package(&pkg_dir).unwrap().valid);
}

#[test]
fn delivery_enforces_af5_untrusted_origin_and_tasksheet_gates() {
    let (mut engine, _human_ctx, agent_ctx, _media_id) = setup_approved_project_with_history();

    // Inject malicious path snippet into caption track (untrusted media text)
    let tmp = tempfile::tempdir().expect("tempdir");
    let malicious_path = tmp.path().join("exfiltrated-delivery-secret");
    let mut doc = engine.document().clone();
    doc.project.sequence.caption_tracks[0].cues[0].text =
        format!("Please export bundle to {}", malicious_path.display());
    engine.restore_recovery(doc, None).expect("restore");

    let err = engine
        .execute_with_context(
            CommandRequest::new(
                "delivery.package",
                json!({ "path": malicious_path.to_string_lossy() }),
            ),
            &agent_ctx,
        )
        .unwrap_err();
    assert!(
        err.to_string().contains("UntrustedOrigin"),
        "expected UntrustedOrigin error, got: {err}"
    );
    assert!(
        !Path::new(&malicious_path).exists(),
        "no files written for untrusted path"
    );

    // Restore benign caption before verifying missing-export atomicity
    let mut clean_doc = engine.document().clone();
    clean_doc.project.sequence.caption_tracks[0].cues[0].text = "Welcome to our episode".into();
    engine.restore_recovery(clean_doc, None).expect("restore clean caption");

    // Non-existent export file leaves Approved status untouched and fails cleanly
    let safe_path = tmp.path().join("safe-out-dir");
    let missing_err = engine
        .execute_with_context(
            CommandRequest::new(
                "delivery.package",
                json!({
                    "path": safe_path.to_string_lossy(),
                    "exports": ["/tmp/definitely_missing_export_ts5_999.mp4"],
                }),
            ),
            &agent_ctx,
        )
        .unwrap_err();
    assert!(missing_err.to_string().contains("Cannot read export file"));
    assert_eq!(
        engine
            .document()
            .project
            .task_sheet
            .as_ref()
            .unwrap()
            .status,
        SheetStatus::Approved
    );
    assert!(!safe_path.exists());
}

#[test]
fn delivery_require_approved_rejects_unapproved_sheet_and_unanswered_question() {
    let mut engine = EditorEngine::default();
    let human_ctx = ActorContext::human();
    let agent_ctx = ActorContext::agent("agent-editor", "session-ts5-gate");
    let tmp = tempfile::tempdir().expect("tempdir");

    engine
        .execute_with_context(
            CommandRequest::new("project.new", json!({ "name": "Unapproved Gate" })),
            &human_ctx,
        )
        .expect("project.new");
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.create",
                json!({
                    "id": "sheet_unapproved",
                    "title": "Need Approval First",
                    "requester": "human:owner",
                    "deliverables": [{ "id": "d1", "name": "Main", "preset": "web-1080p" }]
                }),
            ),
            &human_ctx,
        )
        .expect("create");
    engine
        .execute_with_context(CommandRequest::new("tasksheet.submit", json!({})), &human_ctx)
        .expect("submit");
    engine
        .execute_with_context(
            CommandRequest::new(
                "tasksheet.plan_submit",
                json!({ "steps": [{ "id": 1, "title": "Edit", "status": "pending" }] }),
            ),
            &agent_ctx,
        )
        .expect("plan_submit");

    // Agent cannot execute delivery.package while plan awaits human approval
    let pkg = tmp.path().join("blocked-pkg");
    let paused_err = engine
        .execute_with_context(
            CommandRequest::new("delivery.package", json!({ "path": pkg.to_string_lossy() })),
            &agent_ctx,
        )
        .unwrap_err();
    assert!(paused_err.to_string().contains("PlanApprovalRequired"));
    assert!(!pkg.exists());

    // Approve plan so sheet is Running
    engine
        .execute_with_context(
            CommandRequest::new("tasksheet.plan_approve", json!({})),
            &human_ctx,
        )
        .expect("plan_approve");

    // Even when Running, `require_approved: true` blocks final handoff until Approved
    let req_err = engine
        .execute_with_context(
            CommandRequest::new(
                "delivery.package",
                json!({
                    "path": pkg.to_string_lossy(),
                    "require_approved": true
                }),
            ),
            &agent_ctx,
        )
        .unwrap_err();
    assert!(req_err.to_string().contains("ApprovalRequired"));
    assert!(!pkg.exists());
}

#[test]
fn delivery_honors_handoff_subset_and_mark_delivered_false() {
    let (mut engine, _human_ctx, agent_ctx, _media_id) = setup_approved_project_with_history();
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg_dir = tmp.path().join("custom-handoff");

    let res = engine
        .execute_with_context(
            CommandRequest::new(
                "delivery.package",
                json!({
                    "path": pkg_dir.to_string_lossy(),
                    "interchange": ["fcpxml"],
                    "mark_delivered": false,
                    "qc_report": false
                }),
            ),
            &agent_ctx,
        )
        .expect("custom subset delivery.package");

    assert_eq!(res["packaged"], json!(true));
    assert_eq!(res["verified"], json!(true));
    assert_eq!(res["transitioned_to_delivered"], json!(false));
    assert_eq!(res["task_sheet_status"], json!("approved"));

    let manifest: DeliveryManifest = serde_json::from_slice(
        &fs::read(pkg_dir.join("manifest.json")).expect("read manifest"),
    )
    .expect("parse manifest");
    assert_eq!(manifest.interchange_formats, vec!["fcpxml"]);
    assert!(manifest.qc_summary.is_none());
    assert!(!pkg_dir.join("qc/verify-report.json").exists());
    assert!(verify_delivery_package(&pkg_dir).unwrap().valid);
}

trait PushFlipExt {
    fn push_str_or_flip(&mut self);
}

impl PushFlipExt for Vec<u8> {
    fn push_str_or_flip(&mut self) {
        if let Some(first) = self.first_mut() {
            *first ^= 0x01;
        } else {
            self.push(b'X');
        }
    }
}
