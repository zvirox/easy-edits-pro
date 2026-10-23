use super::*;
use crate::project::{Clip, MediaAsset, MediaKind, ProjectDocument, Track, TrackKind};
use crate::time::{FrameRate, Tick};
use std::path::PathBuf;

use super::*;

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
        plugins: Vec::new(),
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
fn test_premiere_xml_export_and_import() {
    let proj = sample_project();
    let xml = export_premiere_xml(&proj).expect("premiere xml export succeeds");
    assert!(xml.contains("<xmeml version=\"5\">"));
    assert!(xml.contains("<clipitem id=\"clipitem-0-0\">"));
    assert!(xml.contains("<name>interview.mp4</name>"));

    let imported = import_premiere_xml(&xml).expect("premiere xml import succeeds");
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
