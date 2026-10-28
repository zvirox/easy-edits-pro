use super::xml_utils::{escape_xml, extract_attr, parse_seconds_attr, sanitize_untrusted_media_path};
use crate::project::{Clip, MediaAsset, MediaKind, ProjectDocument, Track, TrackKind};
use crate::time::Tick;
use std::path::PathBuf;

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
        let dur_s = media
            .duration
            .map(|d| d.as_millis() as f64 / 1000.0)
            .unwrap_or(60.0);
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

    if let Some(v_track) = doc
        .project
        .sequence
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Video)
    {
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
                let id: u64 = id_str
                    .trim_start_matches('r')
                    .parse()
                    .unwrap_or(media_list.len() as u64 + 1);
                media_list.push(MediaAsset {
                    id,
                    name,
                    path: sanitize_untrusted_media_path(&src)
                        .map(PathBuf::from)
                        .unwrap_or_default(),
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
                label: String::new(),
                volume_db: 0.0,
                enabled: true,
                });
            }
        }
    }

    let track = Track {
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
// 3. Editor XML exchange (xmeml v5) Export & Import
// =========================================================================
