use std::path::PathBuf;
use crate::project::{Clip, MediaAsset, MediaKind, ProjectDocument, Track, TrackKind};
use crate::time::{FrameRate, Tick};

/// Formats a time `Tick` into a standard SMPTE non-drop timecode `HH:MM:SS:FF`.
pub fn ticks_to_timecode(tick: Tick, fps: FrameRate) -> String {
    let frame_dur = fps.frame_ticks().unwrap_or(10_584_000_000).max(1);
    let total_frames = (tick.0.max(0) / frame_dur) as u64;
    let nom_fps = (fps.numerator / fps.denominator.max(1)).max(1) as u64;

    let frames = total_frames % nom_fps;
    let total_secs = total_frames / nom_fps;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;

    format!("{hours:02}:{mins:02}:{secs:02}:{frames:02}")
}

/// Parses an SMPTE timecode `HH:MM:SS:FF` or `HH:MM:SS;FF` into a `Tick`.
pub fn timecode_to_ticks(tc: &str, fps: FrameRate) -> Result<Tick, String> {
    let clean = tc.trim();
    let tokens: Vec<&str> = clean
        .split(|c| c == ':' || c == ';')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.len() != 4 {
        return Err(format!("Invalid timecode format `{tc}`, expected HH:MM:SS:FF"));
    }

    let h: u64 = tokens[0].parse().map_err(|_| format!("Invalid hours in `{tc}`"))?;
    let m: u64 = tokens[1].parse().map_err(|_| format!("Invalid minutes in `{tc}`"))?;
    let s: u64 = tokens[2].parse().map_err(|_| format!("Invalid seconds in `{tc}`"))?;
    let f: u64 = tokens[3].parse().map_err(|_| format!("Invalid frames in `{tc}`"))?;

    let nom_fps = (fps.numerator / fps.denominator.max(1)).max(1) as u64;
    let total_frames = (h * 3600 + m * 60 + s) * nom_fps + f;
    let frame_dur = fps.frame_ticks().unwrap_or(10_584_000_000);

    Ok(Tick(total_frames as i64 * frame_dur))
}

// =========================================================================
// 1. CMX 3600 EDL (Edit Decision List) Export & Import
// =========================================================================

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
                let clip_name = if i + 1 < lines.len() && lines[i + 1].starts_with("* FROM CLIP NAME:") {
                    lines[i + 1].trim_start_matches("* FROM CLIP NAME:").trim().to_string()
                } else {
                    format!("Event {}", parts[0])
                };

                // Add or find media asset
                let media_id = if let Some(existing) = media_list.iter().find(|m: &&MediaAsset| m.name == clip_name) {
                    existing.id
                } else {
                    let id = media_id_counter;
                    media_id_counter += 1;
                    media_list.push(MediaAsset {
                        id,
                        name: clip_name.clone(),
                        path: PathBuf::from(format!("assets/{}", clip_name)),
                        kind: if track_type.starts_with('V') { MediaKind::Video } else { MediaKind::Audio },
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
            clips: video_clips,
            transitions: Vec::new(),
        },
        Track {
            id: 2,
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
            plugins: Vec::new(),
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

/// Exports a `ProjectDocument` into standard Final Cut Pro XML (FCPXML v1.9).
pub fn export_fcpxml(doc: &ProjectDocument) -> Result<String, String> {
    let settings = &doc.project.settings;
    let nom_fps = (settings.frame_rate.numerator / settings.frame_rate.denominator.max(1)).max(1);
    let seq_dur_secs = doc.project.sequence.duration().as_millis() as f64 / 1000.0;

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!DOCTYPE fcpxml>\n");
    out.push_str("<fcpxml version=\"1.9\">\n");
    out.push_str("  <resources>\n");
    out.push_str(&format!(
        "    <format id=\"r1\" name=\"FFVideoFormat1080p\" frameDuration=\"1/{}s\" width=\"{}\" height=\"{}\"/>\n",
        nom_fps,
        settings.width,
        settings.height
    ));

    for media in &doc.project.media {
        let dur_s = media.duration.map(|d| d.as_millis() as f64 / 1000.0).unwrap_or(60.0);
        out.push_str(&format!(
            "    <asset id=\"r{}\" name=\"{}\" src=\"{}\" duration=\"{:.3}s\" format=\"r1\" hasVideo=\"true\" hasAudio=\"true\"/>\n",
            media.id,
            escape_xml(&media.name),
            escape_xml(&media.path.to_string_lossy()),
            dur_s
        ));
    }
    out.push_str("  </resources>\n");

    out.push_str("  <library>\n");
    out.push_str("    <event name=\"EasyEditsPro Event\">\n");
    out.push_str(&format!(
        "      <project name=\"{}\">\n",
        escape_xml(&doc.project.name)
    ));
    out.push_str(&format!(
        "        <sequence format=\"r1\" duration=\"{:.3}s\" tcStart=\"0s\">\n",
        seq_dur_secs
    ));
    out.push_str("          <spine>\n");

    if let Some(v_track) = doc.project.sequence.tracks.iter().find(|t| t.kind == TrackKind::Video) {
        for clip in &v_track.clips {
            let offset_s = clip.timeline_start.as_millis() as f64 / 1000.0;
            let dur_s = clip.duration.as_millis() as f64 / 1000.0;
            let start_s = clip.source_in.as_millis() as f64 / 1000.0;
            let name = doc
                .project
                .media
                .iter()
                .find(|m| m.id == clip.media_id)
                .map(|m| m.name.as_str())
                .unwrap_or("Clip");

            out.push_str(&format!(
                "            <asset-clip name=\"{}\" ref=\"r{}\" offset=\"{:.3}s\" duration=\"{:.3}s\" start=\"{:.3}s\"/>\n",
                escape_xml(name),
                clip.media_id,
                offset_s,
                dur_s,
                start_s
            ));
        }
    }

    out.push_str("          </spine>\n");
    out.push_str("        </sequence>\n");
    out.push_str("      </project>\n");
    out.push_str("    </event>\n");
    out.push_str("  </library>\n");
    out.push_str("</fcpxml>\n");

    Ok(out)
}

/// Imports an FCPXML v1.9 string into a `ProjectDocument`.
pub fn import_fcpxml(xml: &str) -> Result<ProjectDocument, String> {
    let mut media_list = Vec::new();
    let mut clips = Vec::new();
    let mut proj_name = "Imported FCPXML Project".to_string();

    let lines = xml.lines();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("<project name=\"") {
            if let Some(name) = extract_attr(trimmed, "name") {
                proj_name = name;
            }
        } else if trimmed.starts_with("<asset id=\"") {
            if let (Some(id_str), Some(name), Some(src)) = (
                extract_attr(trimmed, "id"),
                extract_attr(trimmed, "name"),
                extract_attr(trimmed, "src"),
            ) {
                let id: u64 = id_str.trim_start_matches('r').parse().unwrap_or(media_list.len() as u64 + 1);
                media_list.push(MediaAsset {
                    id,
                    name,
                    path: PathBuf::from(src),
                    kind: MediaKind::Video,
                    duration: Some(Tick::from_millis(60_000)),
                    width: Some(1920),
                    height: Some(1080),
                    ..Default::default()
                });
            }
        } else if trimmed.starts_with("<asset-clip ") {
            if let (Some(ref_str), Some(offset_str), Some(dur_str), Some(start_str)) = (
                extract_attr(trimmed, "ref"),
                extract_attr(trimmed, "offset"),
                extract_attr(trimmed, "duration"),
                extract_attr(trimmed, "start"),
            ) {
                let media_id: u64 = ref_str.trim_start_matches('r').parse().unwrap_or(1);
                let offset_ms = parse_seconds_attr(&offset_str);
                let dur_ms = parse_seconds_attr(&dur_str);
                let start_ms = parse_seconds_attr(&start_str);

                clips.push(Clip {
                    media_id,
                    timeline_start: Tick::from_millis(offset_ms),
                    source_in: Tick::from_millis(start_ms),
                    duration: Tick::from_millis(dur_ms.max(100)),
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
                });
            }
        }
    }

    let track = Track {
        id: 1,
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
        clips,
        transitions: Vec::new(),
    };

    let mut doc = ProjectDocument::new(&proj_name);
    doc.project.media = media_list;
    doc.project.sequence.name = proj_name;
    doc.project.sequence.tracks = vec![track];

    Ok(doc)
}

// =========================================================================
// 3. Premiere Pro XML (xmeml v5) Export & Import
// =========================================================================

/// Exports a `ProjectDocument` into standard Premiere Pro XML (xmeml v5).
pub fn export_premiere_xml(doc: &ProjectDocument) -> Result<String, String> {
    let settings = &doc.project.settings;
    let timebase = (settings.frame_rate.numerator / settings.frame_rate.denominator.max(1)).max(1);
    let frame_dur = settings.frame_rate.frame_ticks().unwrap_or(10_584_000_000).max(1);
    let total_frames = doc.project.sequence.duration().0 / frame_dur;

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!DOCTYPE xmeml>\n");
    out.push_str("<xmeml version=\"5\">\n");
    out.push_str("  <project>\n");
    out.push_str(&format!("    <name>{}</name>\n", escape_xml(&doc.project.name)));
    out.push_str("    <children>\n");
    out.push_str("      <sequence id=\"sequence-1\">\n");
    out.push_str(&format!("        <name>{}</name>\n", escape_xml(&doc.project.sequence.name)));
    out.push_str(&format!("        <duration>{}</duration>\n", total_frames));
    out.push_str("        <rate>\n");
    out.push_str(&format!("          <timebase>{}</timebase>\n", timebase));
    out.push_str("          <ntsc>FALSE</ntsc>\n");
    out.push_str("        </rate>\n");
    out.push_str("        <media>\n");
    out.push_str("          <video>\n");
    out.push_str("            <format>\n");
    out.push_str("              <samplecharacteristics>\n");
    out.push_str(&format!("                <width>{}</width>\n", settings.width));
    out.push_str(&format!("                <height>{}</height>\n", settings.height));
    out.push_str("              </samplecharacteristics>\n");
    out.push_str("            </format>\n");

    for (t_idx, track) in doc.project.sequence.tracks.iter().filter(|t| t.kind == TrackKind::Video).enumerate() {
        out.push_str("            <track>\n");
        for (c_idx, clip) in track.clips.iter().enumerate() {
            let start_f = clip.timeline_start.0 / frame_dur;
            let end_f = (clip.timeline_start.0 + clip.duration.0) / frame_dur;
            let in_f = clip.source_in.0 / frame_dur;
            let out_f = (clip.source_in.0 + clip.duration.0) / frame_dur;

            let name = doc
                .project
                .media
                .iter()
                .find(|m| m.id == clip.media_id)
                .map(|m| m.name.as_str())
                .unwrap_or("Clip");
            let path = doc
                .project
                .media
                .iter()
                .find(|m| m.id == clip.media_id)
                .map(|m| m.path.to_string_lossy().into_owned())
                .unwrap_or_default();

            out.push_str(&format!("              <clipitem id=\"clipitem-{}-{}\">\n", t_idx, c_idx));
            out.push_str(&format!("                <name>{}</name>\n", escape_xml(name)));
            out.push_str(&format!("                <start>{}</start>\n", start_f));
            out.push_str(&format!("                <end>{}</end>\n", end_f));
            out.push_str(&format!("                <in>{}</in>\n", in_f));
            out.push_str(&format!("                <out>{}</out>\n", out_f));
            out.push_str(&format!("                <file id=\"file-{}\">\n", clip.media_id));
            out.push_str(&format!("                  <name>{}</name>\n", escape_xml(name)));
            out.push_str(&format!("                  <pathurl>{}</pathurl>\n", escape_xml(&path)));
            out.push_str("                </file>\n");
            out.push_str("              </clipitem>\n");
        }
        out.push_str("            </track>\n");
    }

    out.push_str("          </video>\n");
    out.push_str("        </media>\n");
    out.push_str("      </sequence>\n");
    out.push_str("    </children>\n");
    out.push_str("  </project>\n");
    out.push_str("</xmeml>\n");

    Ok(out)
}

/// Imports a Premiere Pro XML (xmeml v5) string into a `ProjectDocument`.
pub fn import_premiere_xml(xml: &str) -> Result<ProjectDocument, String> {
    let mut media_list = Vec::new();
    let mut clips = Vec::new();
    let mut proj_name = "Imported Premiere Project".to_string();
    let fps = FrameRate::FPS_24;
    let frame_dur = fps.frame_ticks().unwrap_or(10_584_000_000);

    let normalized = xml.replace("\r\n", "\n").replace('\r', "\n");
    let clipitem_blocks = normalized.split("<clipitem");

    for (idx, block) in clipitem_blocks.enumerate() {
        if idx == 0 {
            if let Some(name) = extract_tag_content(block, "name") {
                if !name.trim().is_empty() {
                    proj_name = name;
                }
            }
            continue;
        }

        let name = extract_tag_content(block, "name").unwrap_or_else(|| format!("Clip {}", idx));
        let path = extract_tag_content(block, "pathurl").unwrap_or_default();
        let start_f: i64 = extract_tag_content(block, "start").and_then(|s| s.parse().ok()).unwrap_or(0);
        let end_f: i64 = extract_tag_content(block, "end").and_then(|s| s.parse().ok()).unwrap_or(start_f + 120);
        let in_f: i64 = extract_tag_content(block, "in").and_then(|s| s.parse().ok()).unwrap_or(0);

        let media_id = idx as u64;
        media_list.push(MediaAsset {
            id: media_id,
            name: name.clone(),
            path: PathBuf::from(path),
            kind: MediaKind::Video,
            duration: Some(Tick((end_f - start_f).max(1) * frame_dur)),
            width: Some(1920),
            height: Some(1080),
            ..Default::default()
        });

        clips.push(Clip {
            media_id,
            timeline_start: Tick(start_f * frame_dur),
            source_in: Tick(in_f * frame_dur),
            duration: Tick((end_f - start_f).max(1) * frame_dur),
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
        });
    }

    let track = Track {
        id: 1,
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
        clips,
        transitions: Vec::new(),
    };

    let mut doc = ProjectDocument::new(&proj_name);
    doc.project.media = media_list;
    doc.project.sequence.name = proj_name;
    doc.project.sequence.tracks = vec![track];

    Ok(doc)
}

// =========================================================================
// XML Utilities
// =========================================================================

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let key = format!("{}=\"", attr);
    let start_idx = tag.find(&key)? + key.len();
    let rest = &tag[start_idx..];
    let end_idx = rest.find('"')?;
    Some(rest[..end_idx].to_string())
}

fn extract_tag_content(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = text.find(&open)? + open.len();
    let rest = &text[start..];
    let end = rest.find(&close)?;
    Some(rest[..end].trim().to_string())
}

fn parse_seconds_attr(val: &str) -> i64 {
    let num_str = val.trim_end_matches('s').trim();
    if let Ok(secs) = num_str.parse::<f64>() {
        (secs * 1000.0) as i64
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
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
}
