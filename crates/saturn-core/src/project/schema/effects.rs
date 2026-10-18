use super::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    ColorCorrection,
    GaussianBlur,
    Crop,
    Vignette,
    ChromaKey,
    Lut,
    CustomShader,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ColorWheelParam {
    #[serde(default)]
    pub red: f64,
    #[serde(default)]
    pub green: f64,
    #[serde(default)]
    pub blue: f64,
    #[serde(default)]
    pub luma: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ColorWheels {
    #[serde(default)]
    pub lift: ColorWheelParam,
    #[serde(default)]
    pub gamma: ColorWheelParam,
    #[serde(default)]
    pub gain: ColorWheelParam,
    #[serde(default)]
    pub offset: ColorWheelParam,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ColorCorrectionParams {
    #[serde(default = "default_exposure")]
    pub exposure: f64,
    #[serde(default = "default_contrast")]
    pub contrast: f64,
    #[serde(default = "default_saturation")]
    pub saturation: f64,
    #[serde(default)]
    pub hue: f64,
    #[serde(default)]
    pub wheels: ColorWheels,
}

fn default_exposure() -> f64 { 0.0 }
fn default_contrast() -> f64 { 1.0 }
fn default_saturation() -> f64 { 1.0 }

impl Default for ColorCorrectionParams {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
            wheels: ColorWheels::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GaussianBlurParams {
    #[serde(default = "default_blur_sigma")]
    pub sigma: f64,
}

fn default_blur_sigma() -> f64 { 2.0 }

impl Default for GaussianBlurParams {
    fn default() -> Self {
        Self { sigma: 2.0 }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CropParams {
    #[serde(default)]
    pub top: f64,
    #[serde(default)]
    pub bottom: f64,
    #[serde(default)]
    pub left: f64,
    #[serde(default)]
    pub right: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VignetteParams {
    #[serde(default = "default_vignette_intensity")]
    pub intensity: f64,
    #[serde(default = "default_vignette_radius")]
    pub radius: f64,
    #[serde(default = "default_vignette_softness")]
    pub softness: f64,
}

fn default_vignette_intensity() -> f64 { 0.5 }
fn default_vignette_radius() -> f64 { 0.8 }
fn default_vignette_softness() -> f64 { 0.5 }

impl Default for VignetteParams {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            radius: 0.8,
            softness: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChromaKeyParams {
    #[serde(default = "default_chroma_color")]
    pub color: String,
    #[serde(default = "default_chroma_tolerance")]
    pub tolerance: f64,
    #[serde(default = "default_chroma_softness")]
    pub softness: f64,
    #[serde(default = "default_chroma_spill")]
    pub spill_suppression: f64,
}

fn default_chroma_color() -> String { "#00ff00".into() }
fn default_chroma_tolerance() -> f64 { 25.0 }
fn default_chroma_softness() -> f64 { 10.0 }
fn default_chroma_spill() -> f64 { 0.5 }

impl Default for ChromaKeyParams {
    fn default() -> Self {
        Self {
            color: default_chroma_color(),
            tolerance: default_chroma_tolerance(),
            softness: default_chroma_softness(),
            spill_suppression: default_chroma_spill(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LutParams {
    #[serde(default = "default_lut_name")]
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "default_lut_intensity")]
    pub intensity: f64,
    #[serde(default)]
    pub preset_id: Option<String>,
}

fn default_lut_name() -> String { "Teal & Orange".into() }
fn default_lut_intensity() -> f64 { 1.0 }

impl Default for LutParams {
    fn default() -> Self {
        Self {
            name: default_lut_name(),
            path: None,
            intensity: default_lut_intensity(),
            preset_id: Some("teal_orange".into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CustomShaderParams {
    pub name: String,
    pub shader_code: String,
    #[serde(default)]
    pub preset_id: Option<String>,
    #[serde(default)]
    pub uniforms: std::collections::HashMap<String, f64>,
}

impl Default for CustomShaderParams {
    fn default() -> Self {
        Self {
            name: "Custom Shader".into(),
            shader_code: r#"
// Saturn Custom Fragment Shader (GLSL)
precision highp float;
uniform sampler2D u_texture;
uniform vec2 u_resolution;
uniform float u_time;
varying vec2 v_texCoord;

void main() {
    vec4 color = texture2D(u_texture, v_texCoord);
    gl_FragColor = color;
}
"#.trim().into(),
            preset_id: Some("passthrough".into()),
            uniforms: std::collections::HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "params", rename_all = "snake_case")]
pub enum EffectPayload {
    ColorCorrection(ColorCorrectionParams),
    GaussianBlur(GaussianBlurParams),
    Crop(CropParams),
    Vignette(VignetteParams),
    ChromaKey(ChromaKeyParams),
    Lut(LutParams),
    CustomShader(CustomShaderParams),
}

fn default_effect_enabled() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EffectInstance {
    pub id: u64,
    pub name: String,
    #[serde(default = "default_effect_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub payload: EffectPayload,
}

impl EffectInstance {
    pub fn kind(&self) -> EffectKind {
        match &self.payload {
            EffectPayload::ColorCorrection(_) => EffectKind::ColorCorrection,
            EffectPayload::GaussianBlur(_) => EffectKind::GaussianBlur,
            EffectPayload::Crop(_) => EffectKind::Crop,
            EffectPayload::Vignette(_) => EffectKind::Vignette,
            EffectPayload::ChromaKey(_) => EffectKind::ChromaKey,
            EffectPayload::Lut(_) => EffectKind::Lut,
            EffectPayload::CustomShader(_) => EffectKind::CustomShader,
        }
    }

    pub fn new_color_correction(id: u64, params: ColorCorrectionParams) -> Self {
        Self {
            id,
            name: "Color Correction".into(),
            enabled: true,
            payload: EffectPayload::ColorCorrection(params),
        }
    }

    pub fn new_gaussian_blur(id: u64, params: GaussianBlurParams) -> Self {
        Self {
            id,
            name: "Gaussian Blur".into(),
            enabled: true,
            payload: EffectPayload::GaussianBlur(params),
        }
    }

    pub fn new_crop(id: u64, params: CropParams) -> Self {
        Self {
            id,
            name: "Crop".into(),
            enabled: true,
            payload: EffectPayload::Crop(params),
        }
    }

    pub fn new_vignette(id: u64, params: VignetteParams) -> Self {
        Self {
            id,
            name: "Vignette".into(),
            enabled: true,
            payload: EffectPayload::Vignette(params),
        }
    }

    pub fn new_chroma_key(id: u64, params: ChromaKeyParams) -> Self {
        Self {
            id,
            name: "Chroma Key".into(),
            enabled: true,
            payload: EffectPayload::ChromaKey(params),
        }
    }

    pub fn new_lut(id: u64, params: LutParams) -> Self {
        Self {
            id,
            name: "3D LUT".into(),
            enabled: true,
            payload: EffectPayload::Lut(params),
        }
    }

    pub fn new_custom_shader(id: u64, params: CustomShaderParams) -> Self {
        Self {
            id,
            name: params.name.clone(),
            enabled: true,
            payload: EffectPayload::CustomShader(params),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CurvePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RgbCurves {
    #[serde(default = "default_curve_points")]
    pub master: Vec<CurvePoint>,
    #[serde(default = "default_curve_points")]
    pub red: Vec<CurvePoint>,
    #[serde(default = "default_curve_points")]
    pub green: Vec<CurvePoint>,
    #[serde(default = "default_curve_points")]
    pub blue: Vec<CurvePoint>,
}

pub fn default_curve_points() -> Vec<CurvePoint> {
    vec![
        CurvePoint { x: 0.0, y: 0.0 },
        CurvePoint { x: 1.0, y: 1.0 },
    ]
}

impl Default for RgbCurves {
    fn default() -> Self {
        Self {
            master: default_curve_points(),
            red: default_curve_points(),
            green: default_curve_points(),
            blue: default_curve_points(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HslQualifier {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_hue_center")]
    pub hue_center: f64,
    #[serde(default = "default_hue_width")]
    pub hue_width: f64,
    #[serde(default = "default_softness")]
    pub hue_softness: f64,
    #[serde(default = "default_sat_center")]
    pub sat_center: f64,
    #[serde(default = "default_sat_width")]
    pub sat_width: f64,
    #[serde(default = "default_softness")]
    pub sat_softness: f64,
    #[serde(default = "default_luma_center")]
    pub luma_center: f64,
    #[serde(default = "default_luma_width")]
    pub luma_width: f64,
    #[serde(default = "default_softness")]
    pub luma_softness: f64,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub blur_radius: f64,
    #[serde(default)]
    pub hue_shift: f64,
    #[serde(default = "default_one")]
    pub sat_boost: f64,
    #[serde(default)]
    pub exposure_shift: f64,
}

fn default_hue_center() -> f64 { 120.0 }
fn default_hue_width() -> f64 { 40.0 }
fn default_sat_center() -> f64 { 0.5 }
fn default_sat_width() -> f64 { 0.5 }
fn default_luma_center() -> f64 { 0.5 }
fn default_luma_width() -> f64 { 0.5 }
fn default_softness() -> f64 { 0.15 }
fn default_one() -> f64 { 1.0 }

impl Default for HslQualifier {
    fn default() -> Self {
        Self {
            enabled: false,
            hue_center: default_hue_center(),
            hue_width: default_hue_width(),
            hue_softness: default_softness(),
            sat_center: default_sat_center(),
            sat_width: default_sat_width(),
            sat_softness: default_softness(),
            luma_center: default_luma_center(),
            luma_width: default_luma_width(),
            luma_softness: default_softness(),
            invert: false,
            blur_radius: 0.0,
            hue_shift: 0.0,
            sat_boost: 1.0,
            exposure_shift: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ColorAdjustments {
    #[serde(default)]
    pub exposure: f64,
    #[serde(default = "default_contrast")]
    pub contrast: f64,
    #[serde(default = "default_saturation")]
    pub saturation: f64,
    #[serde(default)]
    pub hue: f64,
    #[serde(default)]
    pub wheels: ColorWheels,
    #[serde(default)]
    pub curves: Option<RgbCurves>,
    #[serde(default)]
    pub qualifier: Option<HslQualifier>,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
            wheels: ColorWheels::default(),
            curves: None,
            qualifier: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum KeyframeProperty {
    PositionX,
    PositionY,
    Scale,
    Rotation,
    Opacity,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterpolationKind {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bezier,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Keyframe {
    pub property: KeyframeProperty,
    /// Time relative to the clip's source in point.
    pub time: Tick,
    pub value: f64,
    #[serde(default)]
    pub interpolation: InterpolationKind,
    #[serde(default)]
    pub handle_in_x: Option<f64>,
    #[serde(default)]
    pub handle_in_y: Option<f64>,
    #[serde(default)]
    pub handle_out_x: Option<f64>,
    #[serde(default)]
    pub handle_out_y: Option<f64>,
}

impl Keyframe {
    pub fn new(property: KeyframeProperty, time: Tick, value: f64) -> Self {
        Self {
            property,
            time,
            value,
            interpolation: InterpolationKind::Linear,
            handle_in_x: None,
            handle_in_y: None,
            handle_out_x: None,
            handle_out_y: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProjectView {
    #[serde(default)]
    pub active_sequence: Option<String>,
    #[serde(default)]
    pub timeline_zoom: f64,
    #[serde(default)]
    pub timeline_scroll: f64,
}
