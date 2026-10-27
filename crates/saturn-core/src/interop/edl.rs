use super::timecode::{ticks_to_timecode, timecode_to_ticks};
use crate::project::{Clip, MediaAsset, MediaKind, ProjectDocument, Track, TrackKind};
use crate::time::{FrameRate, Tick};
use std::path::PathBuf;

/// Exports a `ProjectDocument` sequence into a standard CMX 3600 EDL string.
pub fn export_cmx3600_edl(doc: &ProjectDocument) -> Result<String, String> {
    let fps = doc.project.settings.frame_rate;
    let mut out = String::new();

    out.push_str(&format!("TITLE: {}\n", doc.project.sequence.name));
    out.push_str("FCM: NON-DROP FRAME\n\n");

    let mut event_num = 1;

    for track in &doc.project.sequence.tracks {
        let track_code = match track.kind {
            TrackKind::Video => "V",
            TrackKind::Audio => "A",
        };

        for clip in &track.clips {
            let reel = format!("AX{:02}", clip.media_id % 100);
            let src_in = ticks_to_timecode(clip.source_in, fps);
            let src_out = ticks_to_timecode(Tick(clip.source_in.0 + clip.duration.0), fps);
            let dst_in = ticks_to_timecode(clip.timeline_start, fps);
            let dst_out = ticks_to_timecode(Tick(clip.timeline_start.0 + clip.duration.0), fps);

            let clip_name = doc
                .project
                .media
                .iter()
                .find(|m| m.id == clip.media_id)
                .map(|m| m.name.as_str())
                .unwrap_or("Clip");

            out.push_str(&format!(
                "{:03}  {:<8} {:<4} C        {} {} {} {}\n",
                event_num, reel, track_code, src_in, src_out, dst_in, dst_out
            ));
            out.push_str(&format!("* FROM CLIP NAME: {}\n\n", clip_name));

            event_num += 1;
        }
    }

    Ok(out)
}

/// Imports a CMX 3600 EDL string into a new `ProjectDocument`.
pub fn import_cmx3600_edl(edl: &str, project_name: &str) -> Result<ProjectDocument, String> {
    let fps = FrameRate::FPS_24;
    let mut video_clips = Vec::new();
    let mut audio_clips = Vec::new();
    let mut media_list = Vec::new();
    let mut media_id_counter = 1u64;

    let normalized = edl.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().map(str::trim).collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.is_empty() || line.starts_with("TITLE:") || line.starts_with("FCM:") {
            i += 1;
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        // CMX line format: EventNum Reel Track Type [Modifiers] SrcIn SrcOut DstIn DstOut
        if parts.len() >= 8 {
            let track_type = parts[2];
            let src_in_tc = parts[parts.len() - 4];
            let src_out_tc = parts[parts.len() - 3];
            let dst_in_tc = parts[parts.len() - 2];
            let dst_out_tc = parts[parts.len() - 1];

            if let (Ok(s_in), Ok(s_out), Ok(d_in), Ok(d_out)) = (
                timecode_to_ticks(src_in_tc, fps),
                timecode_to_ticks(src_out_tc, fps),
                timecode_to_ticks(dst_in_tc, fps),
                timecode_to_ticks(dst_out_tc, fps),
            ) {
                let duration = Tick((d_out.0 - d_in.0).max(s_out.0 - s_in.0).max(1));

                // Check next line for "* FROM CLIP NAME: ..."
                let clip_name =
                    if i + 1 < lines.len() && lines[i + 1].starts_with("* FROM CLIP NAME:") {
                        lines[i + 1]
                            .trim_start_matches("* FROM CLIP NAME:")
                            .trim()
                            .to_string()
                    } else {
                        format!("Event {}", parts[0])
                    };

                // Add or find media asset
                let media_id = if let Some(existing) = media_list
                    .iter()
                    .find(|m: &&MediaAsset| m.name == clip_name)
                {
                    existing.id
                } else {
                    let id = media_id_counter;
                    media_id_counter += 1;
                    media_list.push(MediaAsset {
                        id,
                        name: clip_name.clone(),
                        path: PathBuf::from(format!("assets/{}", clip_name)),
                        kind: if track_type.starts_with('V') {
                            MediaKind::Video
                        } else {
                            MediaKind::Audio
                        },
                        duration: Some(Tick(s_out.0 + 100_000)),
                        width: Some(1920),
                        height: Some(1080),
                        ..Default::default()
                    });
                    id
                };

                let clip = Clip {
                    media_id,
                    timeline_start: d_in,
                    source_in: s_in,
                    duration,
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

                if track_type.starts_with('V') {
                    video_clips.push(clip);
                } else {
                    audio_clips.push(clip);
                }
            }
        }
        i += 1;
    }

    let tracks = vec![
        Track {
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
            clips: video_clips,
            transitions: Vec::new(),
        },
        Track {
            id: 2,
            group_id: None,
            kind: TrackKind::Audio,
            name: "A1".to_string(),
            gain_db: 0.0,
            pan: 0.0,
            muted: false,
            solo: false,
            locked: false,
            audio_processing: Default::default(),
            bus: Default::default(),
            automation: Default::default(),
            clips: audio_clips,
            transitions: Vec::new(),
        },
    ];

    let mut doc = ProjectDocument::new(project_name);
    doc.project.media = media_list;
    doc.project.sequence.name = project_name.to_string();
    doc.project.sequence.tracks = tracks;
    doc.project.settings.frame_rate = fps;

    Ok(doc)
}

// =========================================================================
// 2. Final Cut Pro XML (FCPXML v1.9) Export & Import
// =========================================================================
