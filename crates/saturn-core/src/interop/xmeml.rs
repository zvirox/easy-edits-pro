use super::xml_utils::{escape_xml, extract_tag_content, sanitize_untrusted_media_path};
use crate::project::{Clip, MediaAsset, MediaKind, ProjectDocument, Track, TrackKind};
use crate::time::{FrameRate, Tick};
use std::path::PathBuf;

/// Exports a `ProjectDocument` into editor XML exchange format (xmeml v5).
pub fn export_xmeml_xml(doc: &ProjectDocument) -> Result<String, String> {
    let settings = &doc.project.settings;
    let timebase = (settings.frame_rate.numerator / settings.frame_rate.denominator.max(1)).max(1);
    let frame_dur = settings
        .frame_rate
        .frame_ticks()
        .unwrap_or(10_584_000_000)
        .max(1);
    let total_frames = doc.project.sequence.duration().0 / frame_dur;

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!DOCTYPE xmeml>\n");
    out.push_str("<xmeml version=\"5\">\n");
    out.push_str("  <project>\n");
    out.push_str(&format!(
        "    <name>{}</name>\n",
        escape_xml(&doc.project.name)
    ));
    out.push_str("    <children>\n");
    out.push_str("      <sequence id=\"sequence-1\">\n");
    out.push_str(&format!(
        "        <name>{}</name>\n",
        escape_xml(&doc.project.sequence.name)
    ));
    out.push_str(&format!("        <duration>{}</duration>\n", total_frames));
    out.push_str("        <rate>\n");
    out.push_str(&format!("          <timebase>{}</timebase>\n", timebase));
    out.push_str("          <ntsc>FALSE</ntsc>\n");
    out.push_str("        </rate>\n");
    out.push_str("        <media>\n");
    out.push_str("          <video>\n");
    out.push_str("            <format>\n");
    out.push_str("              <samplecharacteristics>\n");
    out.push_str(&format!(
        "                <width>{}</width>\n",
        settings.width
    ));
    out.push_str(&format!(
        "                <height>{}</height>\n",
        settings.height
    ));
    out.push_str("              </samplecharacteristics>\n");
    out.push_str("            </format>\n");

    for (t_idx, track) in doc
        .project
        .sequence
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Video)
        .enumerate()
    {
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

            out.push_str(&format!(
                "              <clipitem id=\"clipitem-{}-{}\">\n",
                t_idx, c_idx
            ));
            out.push_str(&format!(
                "                <name>{}</name>\n",
                escape_xml(name)
            ));
            out.push_str(&format!("                <start>{}</start>\n", start_f));
            out.push_str(&format!("                <end>{}</end>\n", end_f));
            out.push_str(&format!("                <in>{}</in>\n", in_f));
            out.push_str(&format!("                <out>{}</out>\n", out_f));
            out.push_str(&format!(
                "                <file id=\"file-{}\">\n",
                clip.media_id
            ));
            out.push_str(&format!(
                "                  <name>{}</name>\n",
                escape_xml(name)
            ));
            out.push_str(&format!(
                "                  <pathurl>{}</pathurl>\n",
                escape_xml(&path)
            ));
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

/// Imports an editor XML exchange (xmeml v5) string into a `ProjectDocument`.
pub fn import_xmeml_xml(xml: &str) -> Result<ProjectDocument, String> {
    let mut media_list = Vec::new();
    let mut clips = Vec::new();
    let mut proj_name = "Imported Editor Project".to_string();
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
        let path = extract_tag_content(block, "pathurl")
            .as_deref()
            .and_then(sanitize_untrusted_media_path)
            .unwrap_or_default();
        let start_f: i64 = extract_tag_content(block, "start")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let end_f: i64 = extract_tag_content(block, "end")
            .and_then(|s| s.parse().ok())
            .unwrap_or(start_f + 120);
        let in_f: i64 = extract_tag_content(block, "in")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

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
                label: String::new(),
                volume_db: 0.0,
                enabled: true,
        });
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
// XML Utilities
// =========================================================================
