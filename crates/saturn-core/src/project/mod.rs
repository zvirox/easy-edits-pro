use crate::time::FrameRate;
#[cfg(test)]
use crate::time::Tick;
#[cfg(test)]
use std::path::PathBuf;

pub const PROJECT_FORMAT: &str = "easy-edits-pro.project";
pub const LEGACY_PROJECT_FORMAT: &str = "project-saturn.project";
pub const PROJECT_SCHEMA_VERSION: u32 = 14;

mod io;
mod migrate;
mod schema;
pub(crate) use migrate::migrate_project;
pub use schema::*;

impl Default for ProjectDocument {
    fn default() -> Self {
        Self::new("Untitled project")
    }
}

impl ProjectDocument {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format: PROJECT_FORMAT.to_owned(),
            schema_version: PROJECT_SCHEMA_VERSION,
            generator: format!("Project Saturn {}", env!("CARGO_PKG_VERSION")),
            project: Project {
                name: name.into(),
                notes: String::new(),
                settings: ProjectSettings {
                    width: 1920,
                    height: 1080,
                    frame_rate: FrameRate::FPS_30,
                    audio_sample_rate: 48_000,
                    use_proxies: false,
                },
                media: Vec::new(),
                bins: Vec::new(),
                graphic_templates: Vec::new(),
                sequence: Sequence {
                    id: 1,
                    name: "Sequence 1".into(),
                    mark_in: None,
                    mark_out: None,
                    caption_tracks: Vec::new(),
                    graphics: Vec::new(),
                    markers: Vec::new(),
                    multicam_clips: Vec::new(),
                    buses: default_audio_buses(),
                    node_graph: None,
                    track_groups: Vec::new(),
                    tracks: vec![
                        Track {
                            id: 1,
                            kind: TrackKind::Video,
                            name: "V1".into(),
                            group_id: None,
                            gain_db: 0.0,
                            pan: 0.0,
                            muted: false,
                            solo: false,
                            locked: false,
                            audio_processing: TrackAudioProcessing::default(),
                            bus: BusTarget::Master,
                            automation: TrackAutomation::default(),
                            clips: Vec::new(),
                            transitions: Vec::new(),
                        },
                        Track {
                            id: 2,
                            kind: TrackKind::Audio,
                            name: "A1".into(),
                            group_id: None,
                            gain_db: 0.0,
                            pan: 0.0,
                            muted: false,
                            solo: false,
                            locked: false,
                            audio_processing: TrackAudioProcessing::default(),
                            bus: BusTarget::Master,
                            automation: TrackAutomation::default(),
                            clips: Vec::new(),
                            transitions: Vec::new(),
                        },
                    ],
                },
                sequences: Vec::new(),
                next_id: 3,
            },
            view: ProjectView {
                active_sequence: Some("Sequence 1".into()),
                timeline_zoom: 1.0,
                timeline_scroll: 0.0,
            },
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format != PROJECT_FORMAT {
            return Err(format!("Unsupported project format `{}`", self.format));
        }
        if self.project.name.trim().is_empty() {
            return Err("Project name cannot be empty".into());
        }
        if self.project.settings.width == 0 || self.project.settings.height == 0 {
            return Err("Project dimensions must be greater than zero".into());
        }
        if self.project.settings.audio_sample_rate == 0
            || self.project.settings.frame_rate.frame_ticks().is_none()
        {
            return Err("Project uses an unsupported frame or audio rate".into());
        }
        let mut ids = std::collections::HashSet::new();
        for item in &self.project.media {
            if !ids.insert(item.id) {
                return Err(format!("Duplicate media id {}", item.id));
            }
            if item.comment.len() > 4000 || item.scene.len() > 200 || item.take.len() > 200 {
                return Err(format!("Media {} has oversized metadata", item.name));
            }
        }
        for track in &self.project.sequence.tracks {
            if !track.gain_db.is_finite() || !(-60.0..=12.0).contains(&track.gain_db) {
                return Err(format!("Track {} has invalid gain", track.name));
            }
            let ap = &track.audio_processing;
            if !ap.eq_low_gain.is_finite()
                || !(-24.0..=24.0).contains(&ap.eq_low_gain)
                || !ap.eq_mid_gain.is_finite()
                || !(-24.0..=24.0).contains(&ap.eq_mid_gain)
                || !ap.eq_high_gain.is_finite()
                || !(-24.0..=24.0).contains(&ap.eq_high_gain)
                || !ap.comp_threshold.is_finite()
                || !(-80.0..=0.0).contains(&ap.comp_threshold)
                || !ap.comp_ratio.is_finite()
                || !(1.0..=50.0).contains(&ap.comp_ratio)
            {
                return Err(format!(
                    "Track {} has invalid audio processing parameters",
                    track.name
                ));
            }
            for clip in &track.clips {
                if clip.duration.0 <= 0
                    || !clip.color.exposure.is_finite()
                    || !(-5.0..=5.0).contains(&clip.color.exposure)
                    || !clip.color.contrast.is_finite()
                    || !(0.0..=2.0).contains(&clip.color.contrast)
                    || !clip.color.saturation.is_finite()
                    || !(0.0..=2.0).contains(&clip.color.saturation)
                    || !clip.color.hue.is_finite()
                    || !(-1.0..=1.0).contains(&clip.color.hue)
                {
                    return Err("Project contains invalid clip properties".into());
                }
                for effect in &clip.effects {
                    match &effect.payload {
                        EffectPayload::ColorCorrection(p) => {
                            if !p.exposure.is_finite()
                                || !(-5.0..=5.0).contains(&p.exposure)
                                || !p.contrast.is_finite()
                                || !(0.0..=2.0).contains(&p.contrast)
                                || !p.saturation.is_finite()
                                || !(0.0..=2.0).contains(&p.saturation)
                                || !p.hue.is_finite()
                                || !(-1.0..=1.0).contains(&p.hue)
                            {
                                return Err(
                                    "Project contains invalid color correction effect values"
                                        .into(),
                                );
                            }
                        }
                        EffectPayload::GaussianBlur(p) => {
                            if !p.sigma.is_finite() || !(0.0..=50.0).contains(&p.sigma) {
                                return Err(
                                    "Project contains invalid gaussian blur effect values".into()
                                );
                            }
                        }
                        EffectPayload::Crop(p) => {
                            if !p.top.is_finite()
                                || !(0.0..=100.0).contains(&p.top)
                                || !p.bottom.is_finite()
                                || !(0.0..=100.0).contains(&p.bottom)
                                || !p.left.is_finite()
                                || !(0.0..=100.0).contains(&p.left)
                                || !p.right.is_finite()
                                || !(0.0..=100.0).contains(&p.right)
                            {
                                return Err("Project contains invalid crop effect values".into());
                            }
                        }
                        EffectPayload::Vignette(p) => {
                            if !p.intensity.is_finite()
                                || !(0.0..=1.0).contains(&p.intensity)
                                || !p.radius.is_finite()
                                || !(0.0..=2.0).contains(&p.radius)
                                || !p.softness.is_finite()
                                || !(0.0..=1.0).contains(&p.softness)
                            {
                                return Err(
                                    "Project contains invalid vignette effect values".into()
                                );
                            }
                        }
                        EffectPayload::ChromaKey(p) => {
                            if p.color.trim().is_empty()
                                || !p.tolerance.is_finite()
                                || !(0.0..=100.0).contains(&p.tolerance)
                                || !p.softness.is_finite()
                                || !(0.0..=100.0).contains(&p.softness)
                                || !p.spill_suppression.is_finite()
                                || !(0.0..=1.0).contains(&p.spill_suppression)
                            {
                                return Err(
                                    "Project contains invalid chroma key effect values".into()
                                );
                            }
                        }
                        EffectPayload::Lut(p) => {
                            if !p.intensity.is_finite() || !(0.0..=1.0).contains(&p.intensity) {
                                return Err("Project contains invalid LUT effect intensity".into());
                            }
                        }
                        EffectPayload::CustomShader(p) => {
                            if p.shader_code.trim().is_empty() {
                                return Err("Project contains empty custom shader code".into());
                            }
                        }
                    }
                }
                for keyframe in &clip.keyframes {
                    let value_in_range = match &keyframe.property {
                        KeyframeProperty::PositionX | KeyframeProperty::PositionY => {
                            (-100_000.0..=100_000.0).contains(&keyframe.value)
                        }
                        KeyframeProperty::Scale => (0.01..=100.0).contains(&keyframe.value),
                        KeyframeProperty::Rotation => {
                            (-36_000.0..=36_000.0).contains(&keyframe.value)
                        }
                        KeyframeProperty::Opacity => (0.0..=1.0).contains(&keyframe.value),
                    };
                    if keyframe.time.0 < 0
                        || keyframe.time.0 > clip.duration.0
                        || !keyframe.value.is_finite()
                        || !value_in_range
                    {
                        return Err("Project contains an invalid keyframe".into());
                    }
                }
            }
            for transition in &track.transitions {
                if transition.duration.0 <= 0 || transition.cut_at.0 < 0 {
                    return Err("Project contains an invalid transition".into());
                }
            }
        }
        for track in self
            .project
            .sequence
            .caption_tracks
            .iter()
            .chain(
                self.project
                    .sequences
                    .iter()
                    .flat_map(|sequence| &sequence.caption_tracks),
            )
        {
            for sub in &track.cues {
                if sub.start_ticks.0 < 0
                    || sub.end_ticks.0 <= sub.start_ticks.0
                    || sub.text.trim().is_empty()
                {
                    return Err("Project contains an invalid caption cue".into());
                }
            }
            if track.name.trim().is_empty() {
                return Err("Project contains a caption track without a name".into());
            }
        }
        for sequence in std::iter::once(&self.project.sequence).chain(&self.project.sequences) {
            for graphic in &sequence.graphics {
                if graphic.duration_ticks.0 <= 0 || graphic.start_ticks.0 < 0 {
                    return Err("Project contains a graphic with invalid timing".into());
                }
                if graphic.transform.width < 0.0 || graphic.transform.height < 0.0 {
                    return Err("Project contains a graphic with negative size".into());
                }
                if !(0.0..=1.0).contains(&graphic.transform.opacity) {
                    return Err("Project contains a graphic with out-of-range opacity".into());
                }
                if graphic.kind == crate::project::GraphicKind::Text
                    && graphic
                        .text
                        .as_ref()
                        .is_some_and(|text| text.content.trim().is_empty())
                {
                    return Err("Project contains a text graphic without content".into());
                }
            }
            for marker in &sequence.markers {
                if marker.name.trim().is_empty() {
                    return Err("Project contains a marker without a name".into());
                }
                if marker.start_ticks.0 < 0 || marker.duration_ticks.0 < 0 {
                    return Err("Project contains a marker with invalid timing".into());
                }
                if marker
                    .start_ticks
                    .0
                    .checked_add(marker.duration_ticks.0)
                    .is_none()
                {
                    return Err("Project contains a marker that overflows the timebase".into());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_saves_atomically_and_keeps_external_media_paths() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("cut.saturn");
        let mut doc = ProjectDocument::new("Cut");
        doc.project.media.push(MediaAsset {
            id: 4,
            name: "shot.mov".into(),
            path: PathBuf::from("/media/shot.mov"),
            kind: MediaKind::Video,
            duration: Some(Tick(42)),
            width: Some(1920),
            height: Some(1080),
            ..Default::default()
        });
        doc.save_atomic(&path).unwrap();
        let loaded = ProjectDocument::load(&path).unwrap();
        assert_eq!(
            loaded.project.media[0].path,
            PathBuf::from("/media/shot.mov")
        );
        assert_eq!(loaded.project.media[0].duration, Some(Tick(42)));
    }

    #[test]
    fn test_sequence_playback_queries() {
        let mut doc = ProjectDocument::new("Playback test");
        let video_track = &mut doc.project.sequence.tracks[0];
        video_track.clips.push(Clip {
            media_id: 1,
            timeline_start: Tick(0),
            source_in: Tick(100),
            duration: Tick(500),
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
        });
        video_track.clips.push(Clip {
            media_id: 2,
            timeline_start: Tick(500),
            source_in: Tick(0),
            duration: Tick(400),
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
        });
        video_track.transitions.push(Transition {
            id: 1,
            kind: TransitionKind::Crossfade,
            cut_at: Tick(500),
            duration: Tick(100),
        });

        let seq = &doc.project.sequence;
        assert_eq!(seq.duration(), Tick(900));

        let active_mid_first = seq.active_clips_at(Tick(250));
        assert_eq!(active_mid_first.len(), 1);
        assert_eq!(active_mid_first[0].1.media_id, 1);

        let active_at_cut = seq.active_clips_at(Tick(500));
        assert_eq!(active_at_cut.len(), 1);
        assert_eq!(active_at_cut[0].1.media_id, 2);

        let trans = seq.transition_at(Tick(500));
        assert!(trans.is_some());
        assert_eq!(trans.unwrap().1.kind, TransitionKind::Crossfade);

        let trans_boundary = seq.transition_at(Tick(460));
        assert!(trans_boundary.is_some());

        let trans_outside = seq.transition_at(Tick(400));
        assert!(trans_outside.is_none());

        assert_eq!(
            seq.tracks[0].next_clip_after(Tick(200)).unwrap().media_id,
            2
        );
    }

    #[test]
    fn test_effect_stack_serialization_and_migration() {
        let mut doc = ProjectDocument::new("Effect test");
        let video_track = &mut doc.project.sequence.tracks[0];
        let mut clip = Clip {
            media_id: 1,
            timeline_start: Tick(0),
            source_in: Tick(0),
            duration: Tick(300),
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
        };

        clip.effects.push(EffectInstance::new_color_correction(
            10,
            ColorCorrectionParams {
                exposure: 0.5,
                contrast: 1.1,
                saturation: 1.2,
                hue: 0.05,
                wheels: ColorWheels::default(),
            },
        ));
        clip.effects.push(EffectInstance::new_gaussian_blur(
            11,
            GaussianBlurParams { sigma: 4.5 },
        ));
        clip.effects.push(EffectInstance::new_crop(
            12,
            CropParams {
                top: 5.0,
                bottom: 10.0,
                left: 2.0,
                right: 2.0,
            },
        ));
        clip.effects.push(EffectInstance::new_vignette(
            13,
            VignetteParams {
                intensity: 0.7,
                radius: 0.9,
                softness: 0.6,
            },
        ));
        clip.effects.push(EffectInstance::new_chroma_key(
            14,
            ChromaKeyParams {
                color: "#00ff00".into(),
                tolerance: 35.0,
                softness: 12.0,
                spill_suppression: 0.75,
            },
        ));

        video_track.clips.push(clip);
        assert!(doc.validate().is_ok());

        // Serialize and deserialize roundtrip
        let json = serde_json::to_string_pretty(&doc).unwrap();
        let loaded: ProjectDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        let loaded_clip = &loaded.project.sequence.tracks[0].clips[0];
        assert_eq!(loaded_clip.effects.len(), 5);
        assert_eq!(loaded_clip.effects[0].kind(), EffectKind::ColorCorrection);
        assert_eq!(loaded_clip.effects[1].kind(), EffectKind::GaussianBlur);
        assert_eq!(loaded_clip.effects[2].kind(), EffectKind::Crop);
        assert_eq!(loaded_clip.effects[3].kind(), EffectKind::Vignette);
        assert_eq!(loaded_clip.effects[4].kind(), EffectKind::ChromaKey);
    }

    #[test]
    fn test_track_audio_properties_and_audibility() {
        let mut t1 = Track {
            id: 1,
            group_id: None,
            kind: TrackKind::Audio,
            name: "A1".into(),
            gain_db: -3.0,
            pan: -0.5,
            muted: false,
            solo: false,
            locked: false,
            audio_processing: TrackAudioProcessing::default(),
            bus: BusTarget::Master,
            automation: TrackAutomation::default(),
            clips: Vec::new(),
            transitions: Vec::new(),
        };

        let mut t2 = Track {
            id: 2,
            group_id: None,
            kind: TrackKind::Audio,
            name: "A2".into(),
            gain_db: 0.0,
            pan: 0.5,
            muted: false,
            solo: false,
            locked: false,
            audio_processing: TrackAudioProcessing::default(),
            bus: BusTarget::Master,
            automation: TrackAutomation::default(),
            clips: Vec::new(),
            transitions: Vec::new(),
        };

        // Neither soloed, neither muted -> both audible
        assert!(t1.is_audible(false));
        assert!(t2.is_audible(false));

        // Mute t1
        t1.muted = true;
        assert!(!t1.is_audible(false));
        assert!(t2.is_audible(false));
        t1.muted = false;

        // Solo t2 -> any_solo is true, t1 not soloed -> t1 inaudible, t2 audible
        t2.solo = true;
        assert!(!t1.is_audible(true));
        assert!(t2.is_audible(true));

        // Both soloed -> both audible
        t1.solo = true;
        assert!(t1.is_audible(true));
        assert!(t2.is_audible(true));
    }
}
