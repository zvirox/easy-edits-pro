use crate::project::{Clip, MediaAsset, MediaKind, ProjectDocument, Track, TrackKind};
use crate::time::Tick;
use std::path::PathBuf;

/// Exports a `ProjectDocument` sequence into an OpenTimelineIO (.otio) JSON string.
pub fn export_otio(doc: &ProjectDocument) -> Result<String, String> {
    let fps = doc.project.settings.frame_rate;
    let rate = (fps.numerator as f64) / (fps.denominator.max(1) as f64);
    let frame_ticks = fps.frame_ticks().unwrap_or(10_584_000_000).max(1);

    let mut otio_tracks = Vec::new();

    for track in &doc.project.sequence.tracks {
        let mut children = Vec::new();
        let mut sorted_clips = track.clips.clone();
        sorted_clips.sort_by_key(|c| c.timeline_start.0);

        let mut current_pos = 0i64;

        for clip in sorted_clips {
            let start = clip.timeline_start.0;
            // Check for gap before this clip
            if start > current_pos {
                let gap_dur = start - current_pos;
                let gap_frames = (gap_dur / frame_ticks).max(0) as f64;
                children.push(serde_json::json!({
                    "OTIO_SCHEMA": "Gap.1",
                    "metadata": {},
                    "name": "Gap",
                    "source_range": {
                        "OTIO_SCHEMA": "TimeRange.1",
                        "start_time": {
                            "OTIO_SCHEMA": "RationalTime.1",
                            "rate": rate,
                            "value": 0.0
                        },
                        "duration": {
                            "OTIO_SCHEMA": "RationalTime.1",
                            "rate": rate,
                            "value": gap_frames
                        }
                    }
                }));
            }

            let dur_frames = (clip.duration.0 / frame_ticks).max(1) as f64;
            let in_frames = (clip.source_in.0 / frame_ticks).max(0) as f64;
            let media_asset = doc.project.media.iter().find(|m| m.id == clip.media_id);
            let media_name = media_asset
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "Clip".into());
            let media_path = media_asset
                .map(|m| m.path.to_string_lossy().into_owned())
                .unwrap_or_default();

            children.push(serde_json::json!({
                "OTIO_SCHEMA": "Clip.1",
                "metadata": {
                    "saturn": {
                        "media_id": clip.media_id,
                        "speed": clip.speed,
                    }
                },
                "name": media_name,
                "source_range": {
                    "OTIO_SCHEMA": "TimeRange.1",
                    "start_time": {
                        "OTIO_SCHEMA": "RationalTime.1",
                        "rate": rate,
                        "value": in_frames
                    },
                    "duration": {
                        "OTIO_SCHEMA": "RationalTime.1",
                        "rate": rate,
                        "value": dur_frames
                    }
                },
                "media_reference": {
                    "OTIO_SCHEMA": "ExternalReference.1",
                    "metadata": {},
                    "target_url": if media_path.starts_with("file://") { media_path } else { format!("file://{media_path}") }
                }
            }));

            current_pos = start + clip.duration.0;
        }

        otio_tracks.push(serde_json::json!({
            "OTIO_SCHEMA": "Track.1",
            "metadata": {},
            "name": track.name,
            "kind": match track.kind {
                TrackKind::Video => "Video",
                TrackKind::Audio => "Audio",
            },
            "children": children
        }));
    }

    let otio_timeline = serde_json::json!({
        "OTIO_SCHEMA": "Timeline.1",
        "metadata": {
            "saturn_generator": format!("Easy Edits Pro (Project Saturn {})", env!("CARGO_PKG_VERSION")),
            "fcm": "NON-DROP FRAME"
        },
        "name": doc.project.sequence.name,
        "global_start_time": null,
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "metadata": {},
            "name": "tracks",
            "children": otio_tracks
        }
    });

    serde_json::to_string_pretty(&otio_timeline)
        .map_err(|e| format!("Failed to serialize OTIO: {e}"))
}

/// Imports an OpenTimelineIO (.otio) JSON string into a new `ProjectDocument`.
pub fn import_otio(content: &str, project_name: &str) -> Result<ProjectDocument, String> {
    let root: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid OpenTimelineIO JSON: {e}"))?;

    let schema = root
        .get("OTIO_SCHEMA")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if !schema.starts_with("Timeline.") {
        return Err(format!(
            "Unsupported or invalid OTIO root schema `{schema}`, expected `Timeline.1`"
        ));
    }

    let seq_name = root
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported OTIO Sequence");
    let mut doc = ProjectDocument::new(project_name);
    doc.project.sequence.name = seq_name.to_string();
    doc.project.sequence.tracks.clear();

    let fps = doc.project.settings.frame_rate;
    let default_rate = (fps.numerator as f64) / (fps.denominator.max(1) as f64);
    let frame_ticks = fps.frame_ticks().unwrap_or(10_584_000_000).max(1);

    let tracks_stack = root.get("tracks");
    let track_children = tracks_stack
        .and_then(|t| t.get("children"))
        .and_then(|c| c.as_array())
        .ok_or_else(|| "OTIO contains no track children".to_string())?;

    let mut next_media_id = 1u64;
    let mut next_track_id = 1u64;

    for (t_idx, track_val) in track_children.iter().enumerate() {
        let kind_str = track_val
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("Video");
        let track_kind = if kind_str.eq_ignore_ascii_case("audio") {
            TrackKind::Audio
        } else {
            TrackKind::Video
        };
        let t_name = track_val.get("name").and_then(|n| n.as_str()).unwrap_or(
            if track_kind == TrackKind::Video {
                "V1"
            } else {
                "A1"
            },
        );

        let mut clips = Vec::new();
        let mut current_timeline_tick = 0i64;

        if let Some(children) = track_val.get("children").and_then(|c| c.as_array()) {
            for item in children {
                let item_schema = item
                    .get("OTIO_SCHEMA")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();
                let dur_val = item
                    .pointer("/source_range/duration/value")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let rate_val = item
                    .pointer("/source_range/duration/rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(default_rate);
                let rate_ratio = if rate_val > 0.0 {
                    default_rate / rate_val
                } else {
                    1.0
                };
                let dur_ticks = (dur_val * rate_ratio * frame_ticks as f64) as i64;

                if item_schema.starts_with("Gap.") {
                    current_timeline_tick += dur_ticks;
                    continue;
                }

                if item_schema.starts_with("Clip.") {
                    let in_val = item
                        .pointer("/source_range/start_time/value")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let in_ticks = (in_val * rate_ratio * frame_ticks as f64) as i64;

                    let raw_url = item
                        .pointer("/media_reference/target_url")
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    let clean_path = raw_url.strip_prefix("file://").unwrap_or(raw_url);
                    let clip_name =
                        item.get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or_else(|| {
                                std::path::Path::new(clean_path)
                                    .file_name()
                                    .and_then(|f| f.to_str())
                                    .unwrap_or("Asset")
                            });

                    // Find or create MediaAsset
                    let media_id = if let Some(existing) = doc
                        .project
                        .media
                        .iter()
                        .find(|m| m.name == clip_name || m.path.to_string_lossy() == clean_path)
                    {
                        existing.id
                    } else {
                        let id = next_media_id;
                        next_media_id += 1;
                        doc.project.media.push(MediaAsset {
                            id,
                            name: clip_name.to_string(),
                            path: PathBuf::from(clean_path),
                            kind: if track_kind == TrackKind::Audio {
                                MediaKind::Audio
                            } else {
                                MediaKind::Video
                            },
                            duration: Some(Tick(dur_ticks.max(frame_ticks * 30))),
                            width: if track_kind == TrackKind::Video {
                                Some(1920)
                            } else {
                                None
                            },
                            height: if track_kind == TrackKind::Video {
                                Some(1080)
                            } else {
                                None
                            },
                            ..Default::default()
                        });
                        id
                    };

                    clips.push(Clip {
                        media_id,
                        timeline_start: Tick(current_timeline_tick),
                        source_in: Tick(in_ticks.max(0)),
                        duration: Tick(dur_ticks.max(frame_ticks)),
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
                    });

                    current_timeline_tick += dur_ticks.max(frame_ticks);
                }
            }
        }

        doc.project.sequence.tracks.push(Track {
            id: next_track_id,
            kind: track_kind,
            name: format!("{t_name} {}", t_idx + 1),
            group_id: None,
            gain_db: 0.0,
            pan: 0.0,
            muted: false,
            solo: false,
            locked: false,
            audio_processing: Default::default(),
            bus: Default::default(),
            automation: Default::default(),
            clips,
            transitions: Vec::new(),
        });
        next_track_id += 1;
    }

    Ok(doc)
}
