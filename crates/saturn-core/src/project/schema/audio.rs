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

/// How breakpoint-curve segments interpolate between automation points.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationInterpolation {
    /// Straight-line ramp between neighbouring points.
    #[default]
    Linear,
    /// Hold the previous point's value until the next point is reached.
    Constant,
    /// Half-cosine ease in/out between neighbouring points.
    Smooth,
}

/// Which fader lane an automation point belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationLane {
    Volume,
    Pan,
}

/// Volume automation is stored in dB, pan in the -1.0 (left) .. +1.0 (right) domain.
pub const AUTOMATION_VOLUME_MIN_DB: f64 = -60.0;
pub const AUTOMATION_VOLUME_MAX_DB: f64 = 12.0;
pub const AUTOMATION_PAN_RANGE: f64 = 1.0;

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
    pub interpolation: AutomationInterpolation,
    #[serde(default)]
    pub volume_points: Vec<AutomationPoint>,
    #[serde(default)]
    pub pan_points: Vec<AutomationPoint>,
}

impl TrackAutomation {
    /// True when playback should follow the recorded curve. `Off` never plays;
    /// every other mode plays the curve when it has points.
    pub fn plays_curve(&self) -> bool {
        !matches!(self.mode, AutomationMode::Off)
            && (!self.volume_points.is_empty() || !self.pan_points.is_empty())
    }

    /// Volume (dB) at `time`, or `None` when the volume lane is empty.
    pub fn evaluate_volume(&self, time: Tick) -> Option<f64> {
        evaluate_points(&self.volume_points, self.interpolation, time)
    }

    /// Pan (-1..+1) at `time`, or `None` when the pan lane is empty.
    pub fn evaluate_pan(&self, time: Tick) -> Option<f64> {
        evaluate_points(&self.pan_points, self.interpolation, time)
    }

    /// Records a Latch/Write-style point: the written value replaces any point
    /// within [`AUTOMATION_RECORD_GAP_TICKS`] and keeps the lane time-sorted.
    pub fn record_point(&mut self, lane: AutomationLane, time: Tick, value: f64) {
        let lane_points = match lane {
            AutomationLane::Volume => &mut self.volume_points,
            AutomationLane::Pan => &mut self.pan_points,
        };
        if let Some(last) = lane_points.last_mut() {
            if time.0 <= last.time.0 + AUTOMATION_RECORD_GAP_TICKS {
                last.time = time;
                last.value = value;
                return;
            }
        }
        lane_points.push(AutomationPoint { time, value });
    }

    /// Removes the recorded point that covers `time` (within the record gap),
    /// if any. Returns true when a point was removed.
    pub fn remove_point_at(&mut self, lane: AutomationLane, time: Tick) -> bool {
        let lane_points = match lane {
            AutomationLane::Volume => &mut self.volume_points,
            AutomationLane::Pan => &mut self.pan_points,
        };
        let index = lane_points.iter().position(|point| {
            let half = AUTOMATION_EDIT_SNAP_TICKS / 2;
            (point.time.0 - time.0).abs() <= half
        });
        if let Some(index) = index {
            lane_points.remove(index);
            true
        } else {
            false
        }
    }

    /// Inserts (or replaces) the exact point at `time` on `lane`.
    pub fn upsert_point(&mut self, lane: AutomationLane, time: Tick, value: f64) {
        let lane_points = match lane {
            AutomationLane::Volume => &mut self.volume_points,
            AutomationLane::Pan => &mut self.pan_points,
        };
        match lane_points.binary_search_by(|p| p.time.0.cmp(&time.0)) {
            Ok(index) => lane_points[index].value = value,
            Err(index) => lane_points.insert(index, AutomationPoint { time, value }),
        }
    }
}

/// Points written closer than this (ticks) collapse into one recorded point;
/// chosen so continuous fader writes along the playhead stay ≈4 ms apart.
pub const AUTOMATION_RECORD_GAP_TICKS: i64 = 1_000_000_000;
/// Snap tolerance (ticks) when clicking a lane to edit or delete a point,
/// a ±~4 ms window at the timeline's tick rate.
pub const AUTOMATION_EDIT_SNAP_TICKS: i64 = 2_000_000_000;

/// Evaluates a sorted breakpoint curve at `time`.
pub fn evaluate_points(
    points: &[AutomationPoint],
    interpolation: AutomationInterpolation,
    time: Tick,
) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    let first = points.first()?;
    let last = points.last()?;
    if time <= first.time {
        return Some(first.value);
    }
    if time >= last.time {
        return Some(last.value);
    }
    // The range checks above guarantee a following neighbour exists.
    let segment_index = points
        .windows(2)
        .position(|window| window[0].time <= time && time < window[1].time)?;
    let a = &points[segment_index];
    let b = &points[segment_index + 1];
    let span = (b.time.0 - a.time.0).max(1);
    let raw = (time.0 - a.time.0) as f64 / span as f64;
    let t = match interpolation {
        AutomationInterpolation::Linear => raw,
        AutomationInterpolation::Constant => return Some(a.value),
        AutomationInterpolation::Smooth => 0.5 * (1.0 - (raw * std::f64::consts::PI).cos()),
    };
    Some(a.value + (b.value - a.value) * t)
}

/// Clamps an automation value to its lane's legal domain.
pub fn clamp_automation_value(lane: AutomationLane, value: f64) -> f64 {
    match lane {
        AutomationLane::Volume => value.clamp(AUTOMATION_VOLUME_MIN_DB, AUTOMATION_VOLUME_MAX_DB),
        AutomationLane::Pan => value.clamp(-AUTOMATION_PAN_RANGE, AUTOMATION_PAN_RANGE),
    }
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
    pub eq_low_gain: f64,     // -15.0 to +15.0 dB (low shelf, 80 Hz)
    pub eq_mid_gain: f64,     // -15.0 to +15.0 dB (mid bell, 1 kHz)
    pub eq_highmid_gain: f64, // -15.0 to +15.0 dB (high-mid bell, 2.5 kHz)
    pub eq_high_gain: f64,    // -15.0 to +15.0 dB (high shelf, 8 kHz)
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
            eq_highmid_gain: 0.0,
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
