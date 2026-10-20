//! Graphics Templates (.fcgt-like format)
//!
//! Provides graphics template system similar to FilmCraft's .fcgt format
//! for reusable motion graphics, lower thirds, transitions, etc.

use saturn_timeline::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use std::collections::HashMap;
use indexmap::IndexMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Graphics template format version
pub const GRAPHICS_TEMPLATE_VERSION: &str = "1.0";

/// Graphics template file extension
pub const GRAPHICS_TEMPLATE_EXTENSION: &str = "sgt";

/// Unique identifier for a graphics template
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateId(pub Uuid);

impl TemplateId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TemplateId {
    fn default() -> Self {
        Self::new()
    }
}

/// Graphics template categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateCategory {
    LowerThird,
    Title,
    Transition,
    Background,
    Overlay,
    Bug,
    FullScreen,
    Custom,
}

impl Default for TemplateCategory {
    fn default() -> Self {
        TemplateCategory::Custom
    }
}

/// Template parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub id: String,
    pub name: String,
    pub description: String,
    pub param_type: ParameterType,
    pub default_value: Value,
    pub min_value: Option<Value>,
    pub max_value: Option<Value>,
    pub options: Option<Vec<String>>, // For enum types
    pub animatable: bool,
    pub category: ParameterCategory,
}

/// Parameter types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Color,
    Font,
    Image,
    Video,
    Audio,
    Enum,
    Vector2,
    Vector3,
    Transform,
}

/// Parameter categories for UI organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterCategory {
    Text,
    Appearance,
    Animation,
    Layout,
    Advanced,
}

/// Template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub id: TemplateId,
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub thumbnail: Option<String>, // Base64 encoded thumbnail
    pub preview: Option<String>,   // Base64 encoded preview video
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub min_version: String,
    pub license: String,
    pub custom: Value,
}

impl Default for TemplateMetadata {
    fn default() -> Self {
        Self {
            id: TemplateId::new(),
            name: "Untitled Template".to_string(),
            description: String::new(),
            category: TemplateCategory::Custom,
            author: String::new(),
            version: "1.0".to_string(),
            tags: Vec::new(),
            thumbnail: None,
            preview: None,
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
            min_version: "1.0".to_string(),
            license: "Proprietary".to_string(),
            custom: Value::Object(Default::default()),
        }
    }
}

/// Template instance parameter values
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateInstance {
    pub template_id: TemplateId,
    pub parameters: HashMap<String, Value>,
    pub keyframes: HashMap<String, Vec<Keyframe>>,
}

/// Keyframe for parameter animation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: Ticks,
    pub value: Value,
    pub easing: Easing,
}

/// Easing functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    Custom,
}

impl Default for Easing {
    fn default() -> Self {
        Easing::Linear
    }
}

impl Easing {
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::EaseInQuart => t * t * t * t,
            Easing::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            Easing::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            Easing::EaseInQuint => t * t * t * t * t,
            Easing::EaseOutQuint => 1.0 - (1.0 - t).powi(5),
            Easing::EaseInOutQuint => {
                if t < 0.5 {
                    16.0 * t * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            }
            Easing::EaseInSine => 1.0 - (std::f64::consts::PI / 2.0 * (1.0 - t)).sin(),
            Easing::EaseOutSine => (std::f64::consts::PI / 2.0 * t).sin(),
            Easing::EaseInOutSine => -(std::f64::consts::PI * (1.0 - 2.0 * t)).cos() / 2.0 + 0.5,
            Easing::EaseInExpo => if t == 0.0 { 0.0 } else { 2.0_f64.powf(10.0 * (t - 1.0)) },
            Easing::EaseOutExpo => if t == 1.0 { 1.0 } else { 1.0 - 2.0_f64.powf(-10.0 * t) },
            Easing::EaseInOutExpo => {
                if t == 0.0 { 0.0 }
                else if t == 1.0 { 1.0 }
                else if t < 0.5 { 2.0_f64.powf(20.0 * t - 10.0) / 2.0 }
                else { (2.0 - 2.0_f64.powf(-20.0 * t + 10.0)) / 2.0 }
            }
            Easing::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            Easing::EaseOutCirc => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
            Easing::EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    (1.0 + (1.0 - (-2.0 * t + 2.0).powi(2)).sqrt()) / 2.0
                }
            }
            Easing::EaseInBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                c3 * t * t * t - c1 * t * t
            }
            Easing::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Easing::EaseInOutBack => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    (2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (2.0 * t - 2.0) + c2) + 2.0) / 2.0
                }
            }
            Easing::EaseInElastic => {
                if t == 0.0 { 0.0 }
                else if t == 1.0 { 1.0 }
                else {
                    -2.0_f64.powf(10.0 * t - 10.0) * (std::f64::consts::PI * (t * 10.0 - 10.75) / 3.0).sin()
                }
            }
            Easing::EaseOutElastic => {
                if t == 0.0 { 0.0 }
                else if t == 1.0 { 1.0 }
                else {
                    2.0_f64.powf(-10.0 * t) * (std::f64::consts::PI * (t * 10.0 - 0.75) / 3.0).sin() + 1.0
                }
            }
            Easing::EaseInOutElastic => {
                if t == 0.0 { 0.0 }
                else if t == 1.0 { 1.0 }
                else if t < 0.5 {
                    -(2.0_f64.powf(20.0 * t - 10.0) * (std::f64::consts::PI * (20.0 * t - 11.125) / 3.0).sin()) / 2.0
                } else {
                    (2.0_f64.powf(-20.0 * t + 10.0) * (std::f64::consts::PI * (20.0 * t - 11.125) / 3.0).sin()) / 2.0 + 1.0
                }
            }
            Easing::EaseInBounce => 1.0 - Easing::EaseOutBounce.apply(1.0 - t),
            Easing::EaseOutBounce => {
                if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    7.5625 * (t - 1.5 / 2.75).powi(2) + 0.75
                } else if t < 2.5 / 2.75 {
                    7.5625 * (t - 2.25 / 2.75).powi(2) + 0.9375
                } else {
                    7.5625 * (t - 2.625 / 2.75).powi(2) + 0.984375
                }
            }
            Easing::EaseInOutBounce => {
                if t < 0.5 {
                    Easing::EaseInBounce.apply(2.0 * t) / 2.0
                } else {
                    (1.0 + Easing::EaseOutBounce.apply(2.0 * t - 1.0)) / 2.0
                }
            }
            Easing::Custom => t,
        }
    }
}

/// Template engine for rendering and managing templates
pub struct TemplateEngine {
    templates: IndexMap<TemplateId, GraphicsTemplate>,
    instances: IndexMap<Uuid, TemplateInstance>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            templates: IndexMap::new(),
            instances: IndexMap::new(),
        }
    }

    /// Register a new template
    pub fn register_template(&mut self, template: GraphicsTemplate) -> Result<TemplateId, TemplateError> {
        let id = template.metadata.id;
        if self.templates.contains_key(&id) {
            return Err(TemplateError::TemplateAlreadyExists);
        }
        self.templates.insert(id, template);
        Ok(id)
    }

    /// Get template by ID
    pub fn get_template(&self, id: TemplateId) -> Option<&GraphicsTemplate> {
        self.templates.get(&id)
    }

    /// Get template mutably
    pub fn get_template_mut(&mut self, id: TemplateId) -> Option<&mut GraphicsTemplate> {
        self.templates.get_mut(&id)
    }

    /// List all templates
    pub fn list_templates(&self) -> Vec<&GraphicsTemplate> {
        self.templates.values().collect()
    }

    /// List templates by category
    pub fn list_by_category(&self, category: TemplateCategory) -> Vec<&GraphicsTemplate> {
        self.templates.values()
            .filter(|t| t.metadata.category == category)
            .collect()
    }

    /// Create a new instance from a template
    pub fn create_instance(&mut self, template_id: TemplateId, overrides: Option<HashMap<String, Value>>) -> Result<Uuid, TemplateError> {
        let template = self.templates.get(&template_id)
            .ok_or(TemplateError::TemplateNotFound)?;

        let instance_id = Uuid::new_v4();
        let mut parameters = HashMap::new();

        // Set default values
        for param in &template.parameters {
            parameters.insert(param.id.clone(), param.default_value.clone());
        }

        // Apply overrides
        if let Some(overrides) = overrides {
            for (key, value) in overrides {
                if template.parameters.iter().any(|p| p.id == key) {
                    parameters.insert(key, value);
                }
            }
        }

        let instance = TemplateInstance {
            template_id,
            parameters,
            keyframes: HashMap::new(),
        };

        self.instances.insert(instance_id, instance);
        Ok(instance_id)
    }

    /// Get an instance
    pub fn get_instance(&self, instance_id: Uuid) -> Option<&TemplateInstance> {
        self.instances.get(&instance_id)
    }

    /// Get instance mutably
    pub fn get_instance_mut(&mut self, instance_id: Uuid) -> Option<&mut TemplateInstance> {
        self.instances.get_mut(&instance_id)
    }

    /// Update instance parameters
    pub fn update_instance_parameters(&mut self, instance_id: Uuid, params: HashMap<String, Value>) -> Result<(), TemplateError> {
        let instance = self.instances.get_mut(&instance_id)
            .ok_or(TemplateError::InstanceNotFound)?;

        // Validate parameters against template
        let template = self.templates.get(&instance.template_id)
            .ok_or(TemplateError::TemplateNotFound)?;

        for (key, value) in params {
            if let Some(param) = template.parameters.iter().find(|p| p.id == key) {
                // Validate value type
                if Self::validate_parameter_value(&param.param_type, &value) {
                    instance.parameters.insert(key, value);
                } else {
                    return Err(TemplateError::InvalidParameterValue { parameter: key });
                }
            }
        }
        Ok(())
    }

    /// Validate parameter value against type
    fn validate_parameter_value(param_type: &ParameterType, value: &Value) -> bool {
        match param_type {
            ParameterType::String => value.is_string(),
            ParameterType::Integer => value.is_i64() || value.is_u64(),
            ParameterType::Float => value.is_f64() || value.is_i64(),
            ParameterType::Boolean => value.is_boolean(),
            ParameterType::Color => value.is_string(), // Expect hex color string
            ParameterType::Font => value.is_string(),
            ParameterType::Image => value.is_string(), // Path or base64
            ParameterType::Video => value.is_string(),
            ParameterType::Audio => value.is_string(),
            ParameterType::Enum => value.is_string(),
            ParameterType::Vector2 => value.is_array() && value.as_array().map_or(false, |a| a.len() == 2),
            ParameterType::Vector3 => value.is_array() && value.as_array().map_or(false, |a| a.len() == 3),
            ParameterType::Transform => value.is_object(),
        }
    }

    /// Evaluate template at a specific time
    pub fn evaluate_at_time(&self, instance_id: Uuid, time: Ticks, frame_rate: FrameRate) -> Result<HashMap<String, Value>, TemplateError> {
        let instance = self.instances.get(&instance_id)
            .ok_or(TemplateError::InstanceNotFound)?;

        let template = self.templates.get(&instance.template_id)
            .ok_or(TemplateError::TemplateNotFound)?;

        let mut results = HashMap::new();

        for param in &template.parameters {
            let mut value = instance.parameters.get(&param.id)
                .cloned()
                .unwrap_or(param.default_value.clone());

            // Check for keyframes
            if let Some(keyframes) = instance.keyframes.get(&param.id) {
                if !keyframes.is_empty() {
                    let time_seconds = time as f64 / TICKS_PER_SECOND as f64;
                    value = Self::interpolate_keyframes(keyframes, time_seconds);
                }
            }

            results.insert(param.id.clone(), value);
        }

        Ok(results)
    }

    /// Interpolate keyframes at a given time
    fn interpolate_keyframes(keyframes: &[Keyframe], time: f64) -> Value {
        if keyframes.is_empty() {
            return Value::Null;
        }

        // Find surrounding keyframes
        let mut prev = &keyframes[0];
        let mut next = None;

        for kf in keyframes.iter().skip(1) {
            let kf_time = kf.time as f64 / TICKS_PER_SECOND as f64;
            if kf_time > time {
                next = Some(kf);
                break;
            }
            prev = kf;
        }

        match next {
            Some(next_kf) => {
                let prev_time = prev.time as f64 / TICKS_PER_SECOND as f64;
                let next_time = next_kf.time as f64 / TICKS_PER_SECOND as f64;

                if next_time == prev_time {
                    return next_kf.value.clone();
                }

                let t = (time - prev_time) / (next_time - prev_time);
                let eased_t = prev.easing.apply(t.clamp(0.0, 1.0));

                Self::interpolate_values(&prev.value, &next_kf.value, eased_t)
            }
            None => prev.value.clone(),
        }
    }

    fn interpolate_values(a: &Value, b: &Value, t: f64) -> Value {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                let a_f = a.as_f64().unwrap_or(0.0);
                let b_f = b.as_f64().unwrap_or(0.0);
                json!(a_f + (b_f - a_f) * t)
            }
            (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
                let result: Vec<Value> = a.iter().zip(b.iter())
                    .map(|(a, b)| Self::interpolate_values(a, b, t))
                    .collect();
                json!(result)
            }
            (Value::Object(a), Value::Object(b)) => {
                let mut result = serde_json::Map::new();
                for (key, a_val) in a {
                    if let Some(b_val) = b.get(key) {
                        result.insert(key.clone(), Self::interpolate_values(a_val, b_val, t));
                    } else {
                        result.insert(key.clone(), a_val.clone());
                    }
                }
                json!(result)
            }
            _ => if t < 0.5 { a.clone() } else { b.clone() },
        }
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Graphics template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsTemplate {
    pub metadata: TemplateMetadata,
    pub parameters: Vec<TemplateParameter>,
    pub timeline: TemplateTimeline,
    pub layers: Vec<TemplateLayer>,
    pub expressions: HashMap<String, String>, // JavaScript-like expressions
    pub scripts: Vec<TemplateScript>,
}

impl GraphicsTemplate {
    pub fn new(metadata: TemplateMetadata) -> Self {
        Self {
            metadata,
            parameters: Vec::new(),
            timeline: TemplateTimeline::default(),
            layers: Vec::new(),
            expressions: HashMap::new(),
            scripts: Vec::new(),
        }
    }

    pub fn add_parameter(&mut self, param: TemplateParameter) {
        self.parameters.push(param);
    }

    pub fn add_layer(&mut self, layer: TemplateLayer) {
        self.layers.push(layer);
    }
}

/// Template timeline
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateTimeline {
    pub duration: Ticks,
    pub frame_rate: FrameRate,
    pub resolution: (u32, u32),
    pub background_color: String,
    pub markers: Vec<Marker>,
}

/// Template layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateLayer {
    pub id: Uuid,
    pub name: String,
    pub layer_type: LayerType,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub transform: Transform,
    pub effects: Vec<LayerEffect>,
    pub source: LayerSource,
    pub mask: Option<Mask>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    Text,
    Shape,
    Image,
    Video,
    Solid,
    Adjustment,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerSource {
    Text { content: String, font: FontProperties },
    Shape { shape_type: ShapeType, fill: FillProperties, stroke: Option<StrokeProperties> },
    Image { path: String },
    Video { path: String },
    Solid { color: String },
    Composition { composition_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontProperties {
    pub family: String,
    pub size: f32,
    pub weight: String,
    pub style: String,
    pub color: String,
    pub alignment: TextAlignment,
    pub tracking: f32,
    pub leading: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeType {
    Rectangle,
    Ellipse,
    Polygon,
    Star,
    CustomPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillProperties {
    pub color: String,
    pub opacity: f32,
    pub gradient: Option<Gradient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokeProperties {
    pub color: String,
    pub width: f32,
    pub dash_pattern: Option<Vec<f32>>,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gradient {
    pub gradient_type: GradientType,
    pub stops: Vec<GradientStop>,
    pub angle: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientType {
    Linear,
    Radial,
    Angular,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStop {
    pub position: f32,
    pub color: String,
    pub opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerEffect {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
    pub path: Vec<[f32; 2]>, // Points
    pub feather: f32,
    pub expansion: f32,
    pub invert: bool,
    pub mode: MaskMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskMode {
    Add,
    Subtract,
    Intersect,
    Difference,
}

/// Template script for advanced logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateScript {
    pub id: Uuid,
    pub name: String,
    pub trigger: ScriptTrigger,
    pub code: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptTrigger {
    OnLoad,
    OnTimeUpdate,
    OnParameterChange,
    OnClick,
    Custom,
}

/// Template instance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateInstance {
    pub template_id: TemplateId,
    pub parameters: HashMap<String, Value>,
    pub keyframes: HashMap<String, Vec<Keyframe>>,
}

/// Keyframe for parameter animation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: Ticks,
    pub value: Value,
    pub easing: Easing,
}

/// Template errors
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found")]
    TemplateNotFound,
    #[error("Template already exists")]
    TemplateAlreadyExists,
    #[error("Instance not found")]
    InstanceNotFound,
    #[error("Invalid parameter value for {parameter}")]
    InvalidParameterValue { parameter: String },
    #[error("Invalid template format")]
    InvalidFormat,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Template file format (similar to .fcgt)
pub mod format {
    use super::*;
    use std::io::{Read, Write};

    /// Save template to file
    pub fn save_template(template: &GraphicsTemplate, path: &std::path::Path) -> Result<(), TemplateError> {
        let data = serialize_template(template)?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(&data)?;
        Ok(())
    }

    /// Load template from file
    pub fn load_template(path: &std::path::Path) -> Result<GraphicsTemplate, TemplateError> {
        let mut file = std::fs::File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        deserialize_template(&data)
    }

    /// Serialize template to binary format
    fn serialize_template(template: &GraphicsTemplate) -> Result<Vec<u8>, TemplateError> {
        // Custom binary format with magic bytes
        let mut data = Vec::new();

        // Magic bytes: "SGTF" (Saturn Graphics Template Format)
        data.extend_from_slice(b"SGTF");

        // Version
        data.extend_from_slice(&[1, 0]);

        // JSON metadata
        let json = serde_json::to_vec(template)?;
        let len = json.len() as u32;
        data.extend_from_slice(&len.to_le_bytes());
        data.extend_from_slice(&json);

        Ok(data)
    }

    /// Deserialize template from binary format
    fn deserialize_template(data: &[u8]) -> Result<GraphicsTemplate, TemplateError> {
        if data.len() < 6 {
            return Err(TemplateError::InvalidFormat);
        }

        // Check magic bytes
        if &data[0..4] != b"SGTF" {
            return Err(TemplateError::InvalidFormat);
        }

        // Version
        let version = u16::from_le_bytes([data[4], data[5]]);
        if version > 1 {
            return Err(TemplateError::InvalidFormat);
        }

        // JSON length
        let len = u32::from_le_bytes([data[6], data[7], data[8], data[9]]) as usize;
        if data.len() < 10 + len {
            return Err(TemplateError::InvalidFormat);
        }

        let json = &data[10..10 + len];
        let template: GraphicsTemplate = serde_json::from_slice(json)?;
        Ok(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_functions() {
        assert_eq!(Easing::Linear.apply(0.0), 0.0);
        assert_eq!(Easing::Linear.apply(1.0), 1.0);
        assert_eq!(Easing::Linear.apply(0.5), 0.5);

        assert!(Easing::EaseIn.apply(0.5) < 0.5);
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
    }

    #[test]
    fn test_template_creation() {
        let metadata = TemplateMetadata::default();
        let template = GraphicsTemplate::new(metadata);
        assert_eq!(template.metadata.name, "Untitled Template");
    }

    #[test]
    fn test_keyframe_interpolation() {
        let keyframes = vec![
            Keyframe { time: 0, value: json!(0.0), easing: Easing::Linear },
            Keyframe { time: TICKS_PER_SECOND, value: json!(100.0), easing: Easing::Linear },
        ];

        let result = TemplateEngine::interpolate_keyframes(&keyframes, 0.5);
        assert_eq!(result.as_f64().unwrap(), 50.0);
    }

    #[test]
    fn test_easing_apply() {
        let easing = Easing::EaseInOut;
        assert_eq!(easing.apply(0.0), 0.0);
        assert_eq!(easing.apply(1.0), 1.0);
        assert!(easing.apply(0.25) < 0.5);
        assert!(easing.apply(0.75) > 0.5);
    }

    #[test]
    fn test_template_engine() {
        let mut engine = TemplateEngine::new();
        let metadata = TemplateMetadata::default();
        let template = GraphicsTemplate::new(metadata);
        let id = engine.register_template(template).unwrap();
        assert!(engine.get_template(id).is_some());
    }
}