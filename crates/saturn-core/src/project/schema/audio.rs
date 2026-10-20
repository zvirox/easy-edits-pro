use super::*;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BusTarget {
    #[default]
    Master,
    Submix1,
    Submix2,
    AuxFx,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationMode {
    #[default]
    Off,
    Read,
    Touch,
    Latch,
    Write,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AutomationPoint {
    pub time: Tick,
    pub value: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TrackAutomation {
    #[serde(default)]
    pub mode: AutomationMode,
    #[serde(default)]
    pub volume_points: Vec<AutomationPoint>,
    #[serde(default)]
    pub pan_points: Vec<AutomationPoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioPluginInstance {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_plugin_format")]
    pub plugin_format: String,
    #[serde(default = "default_plugin_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub preset: String,
    #[serde(default)]
    pub params: std::collections::HashMap<String, f64>,
}

fn default_plugin_format() -> String {
    "builtin".into()
}
fn default_plugin_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioBus {
    pub target: BusTarget,
    pub name: String,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub processing: TrackAudioProcessing,
}

pub fn default_audio_buses() -> Vec<AudioBus> {
    vec![
        AudioBus {
            target: BusTarget::Master,
            name: "Master Out".into(),
            gain_db: 0.0,
            muted: false,
            solo: false,
            processing: TrackAudioProcessing::default(),
        },
        AudioBus {
            target: BusTarget::Submix1,
            name: "Submix 1 (Dialogue)".into(),
            gain_db: 0.0,
            muted: false,
            solo: false,
            processing: TrackAudioProcessing::default(),
        },
        AudioBus {
            target: BusTarget::Submix2,
            name: "Submix 2 (Music)".into(),
            gain_db: 0.0,
            muted: false,
            solo: false,
            processing: TrackAudioProcessing::default(),
        },
        AudioBus {
            target: BusTarget::AuxFx,
            name: "Aux FX (Reverb)".into(),
            gain_db: 0.0,
            muted: false,
            solo: false,
            processing: TrackAudioProcessing::default(),
        },
    ]
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TrackAudioProcessing {
    pub eq_enabled: bool,
    pub eq_low_gain: f64,  // -15.0 to +15.0 dB
    pub eq_mid_gain: f64,  // -15.0 to +15.0 dB
    pub eq_high_gain: f64, // -15.0 to +15.0 dB
    pub comp_enabled: bool,
    pub comp_threshold: f64, // -60.0 to 0.0 dB
    pub comp_ratio: f64,     // 1.0 to 20.0
    pub comp_attack: f64,    // 0.001 to 0.1 s
    pub comp_release: f64,   // 0.05 to 1.0 s
    pub denoise_enabled: bool,
    pub denoise_amount: f64, // 0.0 to 1.0
}

impl Default for TrackAudioProcessing {
    fn default() -> Self {
        Self {
            eq_enabled: false,
            eq_low_gain: 0.0,
            eq_mid_gain: 0.0,
            eq_high_gain: 0.0,
            comp_enabled: false,
            comp_threshold: -24.0,
            comp_ratio: 4.0,
            comp_attack: 0.005,
            comp_release: 0.1,
            denoise_enabled: false,
            denoise_amount: 0.5,
        }
    }
}
