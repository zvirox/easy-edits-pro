use super::*;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    #[default]
    Crossfade,
    DipToBlack,
    DipToWhite,
    WipeLeft,
    WipeRight,
    SlideLeft,
    SlideRight,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transition {
    pub id: u64,
    pub kind: TransitionKind,
    pub cut_at: Tick,
    pub duration: Tick,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubtitleItem {
    pub id: u64,
    pub start_ticks: Tick,
    pub end_ticks: Tick,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiCamSyncMode {
    #[default]
    Timecode,
    AudioWaveform,
    Marker,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MultiCamAngle {
    pub id: u64,
    pub name: String,
    pub media_id: u64,
    #[serde(default)]
    pub sync_offset_ticks: Tick,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MultiCamClip {
    pub id: u64,
    pub name: String,
    pub angles: Vec<MultiCamAngle>,
    pub active_angle_id: u64,
    #[serde(default)]
    pub sync_mode: MultiCamSyncMode,
}
