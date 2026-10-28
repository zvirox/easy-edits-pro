use super::*;
use crate::project::{Clip, MediaAsset, MediaKind, ProjectDocument, Track, TrackKind};
use crate::time::{FrameRate, Tick};
use std::path::PathBuf;

fn sample_project() -> ProjectDocument {
    let fps = FrameRate::FPS_24;
    let clip = Clip {
        media_id: 1,
        timeline_start: Tick::from_millis(0),
        source_in: Tick::from_millis(1000),
        duration: Tick::from_millis(5000),
        speed: 1.0,
        group_id: None,
        color: Default::default(),
        effects: Vec::new(),
        keyframes: Vec::new(),
        multicam_id: None,
        multicam_angle_id: None,
        tracking: None,
        node_graph: None,
        speed_ramp: None,
        motion_blur: None,
        compound_sequence_id: None,
                label: String::new(),
                volume_db: 0.0,
                enabled: true,
    };

    let mut doc = ProjectDocument::new("Test Interop");
    doc.project.settings.frame_rate = fps;
    doc.project.media = vec![MediaAsset {
        id: 1,
        name: "interview.mp4".to_string(),
        path: PathBuf::from("/media/interview.mp4"),
        kind: MediaKind::Video,
        duration: Some(Tick::from_millis(10_000)),
        width: Some(1920),
        height: Some(1080),
        ..Default::default()
    }];
    doc.project.sequence.name = "Main Sequence".to_string();
    doc.project.sequence.tracks = vec![Track {
        id: 1,
        group_id: None,
        kind: TrackKind::Video,
        name: "V1".to_string(),
        gain_db: 0.0,
        pan: 0.0,
        muted: false,
        solo: false,
        locked: false,
        audio_processing: Default::default(),
        bus: Default::default(),
        automation: Default::default(),
        clips: vec![clip],
        transitions: Vec::new(),
    }];
    doc
}

#[test]
fn test_edl_export_and_import() {
    let proj = sample_project();
    let edl = export_cmx3600_edl(&proj).expect("edl export succeeds");
    assert!(edl.contains("TITLE: Main Sequence"));
    assert!(edl.contains("001  AX01     V    C"));
    assert!(edl.contains("* FROM CLIP NAME: interview.mp4"));

    let imported = import_cmx3600_edl(&edl, "Reimported").expect("edl import succeeds");
    assert_eq!(imported.project.name, "Reimported");
    assert_eq!(imported.project.sequence.tracks[0].clips.len(), 1);
    let c = &imported.project.sequence.tracks[0].clips[0];
    assert_eq!(c.timeline_start.as_millis(), 0);
    assert_eq!(c.duration.as_millis(), 5000);
}

#[test]
fn test_fcpxml_export_and_import() {
    let proj = sample_project();
    let xml = export_fcpxml(&proj).expect("fcpxml export succeeds");
    assert!(xml.contains("<fcpxml version=\"1.9\">"));
    assert!(xml.contains("<asset-clip name=\"interview.mp4\""));

    let imported = import_fcpxml(&xml).expect("fcpxml import succeeds");
    assert_eq!(imported.project.name, "Test Interop");
    assert_eq!(imported.project.sequence.tracks[0].clips.len(), 1);
    let c = &imported.project.sequence.tracks[0].clips[0];
    assert_eq!(c.timeline_start.as_millis(), 0);
    assert_eq!(c.duration.as_millis(), 5000);
}

#[test]
fn test_xmeml_xml_export_and_import() {
    let proj = sample_project();
    let xml = export_xmeml_xml(&proj).expect("xmeml xml export succeeds");
    assert!(xml.contains("<xmeml version=\"5\">"));
    assert!(xml.contains("<clipitem id=\"clipitem-0-0\">"));
    assert!(xml.contains("<name>interview.mp4</name>"));

    let imported = import_xmeml_xml(&xml).expect("xmeml xml import succeeds");
    assert_eq!(imported.project.sequence.tracks[0].clips.len(), 1);
    let c = &imported.project.sequence.tracks[0].clips[0];
    assert_eq!(c.timeline_start.as_millis(), 0);
    assert_eq!(c.duration.as_millis(), 5000);
}

#[test]
fn test_otio_export_and_import() {
    let proj = sample_project();
    let otio = export_otio(&proj).expect("otio export succeeds");
    assert!(otio.contains("\"OTIO_SCHEMA\": \"Timeline.1\""));
    assert!(otio.contains("\"OTIO_SCHEMA\": \"Track.1\""));
    assert!(otio.contains("\"OTIO_SCHEMA\": \"Clip.1\""));
    assert!(otio.contains("\"name\": \"interview.mp4\""));

    let imported = import_otio(&otio, "OTIO Imported").expect("otio import succeeds");
    assert_eq!(imported.project.name, "OTIO Imported");
    assert_eq!(imported.project.sequence.name, "Main Sequence");
    assert_eq!(imported.project.sequence.tracks[0].clips.len(), 1);
    let c = &imported.project.sequence.tracks[0].clips[0];
    assert_eq!(c.timeline_start.as_millis(), 0);
    assert_eq!(c.duration.as_millis(), 5000);
}

// --- Import path hardening ---------------------------------------------------
// Media paths parsed from untrusted interchange documents may never carry
// parent (`..`) references: an imported project registers its media paths
// into the shell's asset scope, so `../../` traversal would point the app
// at arbitrary local files.

use super::xml_utils::sanitize_untrusted_media_path;

#[test]
fn sanitize_collapses_or_rejects_parent_references() {
    // Relative paths that climb above their root are rejected outright.
    assert_eq!(sanitize_untrusted_media_path("../../.ssh/id_rsa"), None);
    assert_eq!(sanitize_untrusted_media_path("assets/../../.ssh/id_rsa"), None);
    assert_eq!(sanitize_untrusted_media_path("file://../../.ssh/id_rsa"), None);
    // Interior `..` components collapse exactly like OS path resolution.
    assert_eq!(
        sanitize_untrusted_media_path("assets/../media/interview.mov"),
        Some("media/interview.mov".into())
    );
    assert_eq!(
        sanitize_untrusted_media_path("file:///safe/%2e%2e/etc/passwd"),
        Some("/etc/passwd".into())
    );
    // Ordinary absolute and percent-encoded paths pass through unchanged.
    assert_eq!(
        sanitize_untrusted_media_path("file:///media/interview.mp4"),
        Some("/media/interview.mp4".into())
    );
    assert_eq!(
        sanitize_untrusted_media_path("file://localhost/Volumes/A/a%20b.mov"),
        Some("/Volumes/A/a b.mov".into())
    );
    assert_eq!(sanitize_untrusted_media_path("./clip.mov"), Some("clip.mov".into()));
    assert_eq!(sanitize_untrusted_media_path(""), None);
    assert_eq!(sanitize_untrusted_media_path("   "), None);
}

#[test]
fn edl_import_never_stores_parent_escape_paths() {
    let edl = "TITLE: Pwn\n\n001  AX       V     C        00:00:00:00 00:00:01:00 00:00:00:00 00:00:01:00\n* FROM CLIP NAME: ../../.ssh/id_rsa\n";
    let doc = import_cmx3600_edl(edl, "Pwned").expect("edl import succeeds");
    assert_eq!(doc.project.media.len(), 1);
    let path = doc.project.media[0].path.to_string_lossy();
    assert!(
        !path.contains(".."),
        "EDL clip-name traversal must not reach the stored path, got: {path}"
    );
}

#[test]
fn xmeml_import_never_stores_parent_escape_paths() {
    let xml = "<xmeml version=\"5\"><project><name>Pwn</name>\
        <clipitem id=\"1\"><name>Stealer</name><pathurl>file://../../.ssh/id_rsa</pathurl>\
        <start>0</start><end>24</end><in>0</in></clipitem></project></xmeml>";
    let doc = import_xmeml_xml(xml).expect("xmeml import succeeds");
    assert_eq!(doc.project.media.len(), 1);
    let path = doc.project.media[0].path.to_string_lossy();
    assert!(
        !path.contains(".."),
        "xmeml pathurl traversal must not reach the stored path, got: {path}"
    );

    // Percent-encoded traversal is decoded before the check, not smuggled by it.
    let xml = "<xmeml version=\"5\"><project><name>Pwn2</name>\
        <clipitem id=\"1\"><name>Stealer</name><pathurl>assets/%2e%2e/%2e%2e/.ssh/id_rsa</pathurl>\
        <start>0</start><end>24</end><in>0</in></clipitem></project></xmeml>";
    let doc = import_xmeml_xml(xml).expect("xmeml import succeeds");
    assert!(!doc.project.media[0].path.to_string_lossy().contains(".."));

    // A well-formed encoded path still survives the round trip.
    let xml = "<xmeml version=\"5\"><project><name>Ok</name>\
        <clipitem id=\"1\"><name>Ok</name><pathurl>file:///media/a%20b.mov</pathurl>\
        <start>0</start><end>24</end><in>0</in></clipitem></project></xmeml>";
    let doc = import_xmeml_xml(xml).expect("xmeml import succeeds");
    assert_eq!(
        doc.project.media[0].path.to_string_lossy(),
        "/media/a b.mov"
    );
}

#[test]
fn otio_import_never_stores_parent_escape_paths() {
    let otio = export_otio(&sample_project()).expect("otio export succeeds");
    let evil = otio.replace(
        "\"file:///media/interview.mp4\"",
        "\"file://../../.ssh/id_rsa\"",
    );
    let doc = import_otio(&evil, "Pwned").expect("otio import succeeds");
    assert!(!doc.project.media.is_empty());
    for media in &doc.project.media {
        let path = media.path.to_string_lossy();
        assert!(
            !path.contains(".."),
            "OTIO target_url traversal must not reach the stored path, got: {path}"
        );
    }
}

#[test]
fn fcpxml_import_never_stores_parent_escape_paths() {
    let xml = "<fcpxml version=\"1.9\">\n<project name=\"Pwn\">\n\
        <asset id=\"r1\" name=\"Stealer\" src=\"file://../../.ssh/id_rsa\"/>\n\
        </project>\n</fcpxml>";
    let doc = import_fcpxml(xml).expect("fcpxml import succeeds");
    assert!(!doc.project.media.is_empty());
    let path = doc.project.media[0].path.to_string_lossy();
    assert!(
        !path.contains(".."),
        "fcpxml src traversal must not reach the stored path, got: {path}"
    );
}

#[test]
fn xmeml_import_scales_linearly_over_many_clipitems() {
    // Guard against a return of the quadratic-parse regression: 20k clipitems
    // must import in bounded time (linear parse is well under 1s even in
    // debug builds; a quadratic rescan would take minutes).
    let mut xml = String::from("<xmeml version=\"5\"><project><name>Perf</name>");
    for i in 0..20_000u64 {
        xml.push_str(&format!(
            "<clipitem id=\"{i}\"><name>Clip {i}</name><pathurl>file:///media/{i}.mov</pathurl>\
             <start>{}</start><end>{}</end><in>0</in><out>{}</out></clipitem>",
            i * 10,
            i * 10 + 10,
            i * 10 + 10
        ));
    }
    xml.push_str("</project></xmeml>");

    let started = std::time::Instant::now();
    let doc = import_xmeml_xml(&xml).expect("large xmeml import succeeds");
    let elapsed = started.elapsed();
    assert_eq!(doc.project.media.len(), 20_000);
    assert_eq!(doc.project.sequence.tracks[0].clips.len(), 20_000);
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "xmeml import of 20k clipitems took {elapsed:?} - parser is not linear"
    );
}
