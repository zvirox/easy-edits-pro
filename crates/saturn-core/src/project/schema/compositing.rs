use super::*;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompositingNodeKind {
    #[default]
    Merge,
    Transform,
    Keyer,
    Mask,
    Particles,
    MediaSource,
    OutputSink,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositingNode {
    pub id: u64,
    pub kind: CompositingNodeKind,
    pub title: String,
    pub pos_x: f64,
    pub pos_y: f64,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default = "default_node_enabled")]
    pub enabled: bool,
}

fn default_node_enabled() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeConnection {
    pub from_node: u64,
    pub from_socket: String,
    pub to_node: u64,
    pub to_socket: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RotoSplinePoint {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub handle_in: (f64, f64),
    #[serde(default)]
    pub handle_out: (f64, f64),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RotoSplineMask {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_mask_closed")]
    pub closed: bool,
    #[serde(default)]
    pub inverted: bool,
    #[serde(default)]
    pub feather: f64,
    #[serde(default = "default_mask_opacity")]
    pub opacity: f64,
    #[serde(default)]
    pub points: Vec<RotoSplinePoint>,
}

fn default_mask_closed() -> bool { true }
fn default_mask_opacity() -> f64 { 1.0 }

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeGraph {
    #[serde(default)]
    pub nodes: Vec<CompositingNode>,
    #[serde(default)]
    pub connections: Vec<NodeConnection>,
    #[serde(default)]
    pub masks: Vec<RotoSplineMask>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpeedRampPoint {
    pub time: Tick,
    pub speed_factor: f64,
    #[serde(default)]
    pub bezier_tangent: (f64, f64),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SpeedRamp {
    #[serde(default)]
    pub points: Vec<SpeedRampPoint>,
    #[serde(default = "default_optical_flow")]
    pub optical_flow_interpolation: bool,
    #[serde(default)]
    pub frame_blend: bool,
}

fn default_optical_flow() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MotionBlurParams {
    #[serde(default = "default_motion_blur_enabled")]
    pub enabled: bool,
    #[serde(default = "default_shutter_angle")]
    pub shutter_angle_deg: f64,
    #[serde(default = "default_motion_samples")]
    pub samples: u32,
}

fn default_motion_blur_enabled() -> bool { true }
fn default_shutter_angle() -> f64 { 180.0 }
fn default_motion_samples() -> u32 { 8 }

impl Default for MotionBlurParams {
    fn default() -> Self {
        Self {
            enabled: true,
            shutter_angle_deg: 180.0,
            samples: 8,
        }
    }
}
