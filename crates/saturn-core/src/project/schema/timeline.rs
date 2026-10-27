use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackGroup {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_track_group_color")]
    pub color: String,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub locked: bool,
}

fn default_true() -> bool {
    true
}

fn default_track_group_color() -> String {
    "#a855f7".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sequence {
    #[serde(default = "default_sequence_id")]
    pub id: u64,
    pub name: String,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub track_groups: Vec<TrackGroup>,
    #[serde(default)]
    pub mark_in: Option<Tick>,
    #[serde(default)]
    pub mark_out: Option<Tick>,
    #[serde(default)]
    pub caption_tracks: Vec<CaptionTrack>,
    #[serde(default)]
    pub graphics: Vec<Graphic>,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub multicam_clips: Vec<MultiCamClip>,
    #[serde(default = "default_audio_buses")]
    pub buses: Vec<AudioBus>,
    #[serde(default)]
    pub node_graph: Option<NodeGraph>,
}

fn default_sequence_id() -> u64 {
    1
}

impl Sequence {
    pub fn duration(&self) -> Tick {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| Tick(c.timeline_start.0.saturating_add(c.duration.0)))
            .max()
            .unwrap_or(Tick(0))
    }

    pub fn active_clips_at(&self, tick: Tick) -> Vec<(&Track, &Clip)> {
        let mut results = Vec::new();
        for track in &self.tracks {
            for clip in &track.clips {
                let start = clip.timeline_start.0;
                let end = start.saturating_add(clip.duration.0);
                if tick.0 >= start && tick.0 < end {
                    results.push((track, clip));
                }
            }
        }
        results
    }

    pub fn transition_at(&self, tick: Tick) -> Option<(&Track, &Transition)> {
        for track in &self.tracks {
            for transition in &track.transitions {
                let half_dur = transition.duration.0 / 2;
                let start = transition.cut_at.0.saturating_sub(half_dur);
                let end = transition.cut_at.0.saturating_add(half_dur);
                if tick.0 >= start && tick.0 <= end {
                    return Some((track, transition));
                }
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackingMode {
    #[default]
    Transform,
    Stabilize,
    Pin,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrackPoint {
    pub time: Tick,
    pub x: f64,
    pub y: f64,
    #[serde(default = "default_track_confidence")]
    pub confidence: f64,
}

fn default_track_confidence() -> f64 {
    1.0
}
fn default_smoothness() -> f64 {
    0.5
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MotionTrackingData {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub mode: TrackingMode,
    #[serde(default)]
    pub points: Vec<TrackPoint>,
    #[serde(default)]
    pub pinned_clip_id: Option<u64>,
    #[serde(default = "default_smoothness")]
    pub smoothness: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub kind: TrackKind,
    pub name: String,
    #[serde(default)]
    pub group_id: Option<u64>,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub pan: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub audio_processing: TrackAudioProcessing,
    #[serde(default)]
    pub bus: BusTarget,
    #[serde(default)]
    pub automation: TrackAutomation,
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

impl Track {
    pub fn is_audible(&self, any_solo: bool) -> bool {
        !self.muted && (!any_solo || self.solo)
    }

    pub fn clip_at(&self, tick: Tick) -> Option<&Clip> {
        self.clips.iter().find(|clip| {
            let start = clip.timeline_start.0;
            let end = start.saturating_add(clip.duration.0);
            tick.0 >= start && tick.0 < end
        })
    }

    pub fn next_clip_after(&self, tick: Tick) -> Option<&Clip> {
        self.clips
            .iter()
            .find(|clip| clip.timeline_start.0 > tick.0)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    #[default]
    Video,
    Audio,
}

fn default_speed() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub media_id: u64,
    pub timeline_start: Tick,
    pub source_in: Tick,
    pub duration: Tick,
    #[serde(default = "default_speed")]
    pub speed: f64,
    #[serde(default)]
    pub group_id: Option<u64>,
    #[serde(default)]
    pub color: ColorAdjustments,
    #[serde(default)]
    pub effects: Vec<EffectInstance>,
    #[serde(default)]
    pub keyframes: Vec<Keyframe>,
    #[serde(default)]
    pub multicam_id: Option<u64>,
    #[serde(default)]
    pub multicam_angle_id: Option<u64>,
    #[serde(default)]
    pub tracking: Option<MotionTrackingData>,
    #[serde(default)]
    pub node_graph: Option<NodeGraph>,
    #[serde(default)]
    pub speed_ramp: Option<SpeedRamp>,
    #[serde(default)]
    pub motion_blur: Option<MotionBlurParams>,
    #[serde(default)]
    pub compound_sequence_id: Option<u64>,
    /// FC-style label colour name ("blue", "mango", ...); empty = unlabelled.
    #[serde(default)]
    pub label: String,
    /// Per-clip audio trim in dB, applied on top of the track gain.
    #[serde(default)]
    pub volume_db: f64,
    /// FC Enable: disabled clips stay on the timeline but render nothing.
    #[serde(default = "default_true")]
    pub enabled: bool,
}
