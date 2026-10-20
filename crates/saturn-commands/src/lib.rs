//! Unified command system for Easy Edits Pro
//!
//! This crate provides the central command registry, execution pipeline,
//! undo/redo management, and introspection capabilities used by the Tauri
//! frontend, CLI, MCP server, and control server.

pub mod context;
pub mod handler;
pub mod params;
pub mod registry;
pub mod shortcuts;
pub mod undo;

// Re-export everything from submodules - only items that are NOT defined in lib.rs
pub use context::{
    AppContext, AppState, AppEvent, ExecutionContext, CommandContext,
    ProjectHandle, SequenceHandle, SelectionState, ClipHandle, TrackHandle,
    TransitionHandle, UiState, UiState as UIState,
    EventBus,
};
pub use handler::{
    register_builtin_commands, EchoCommand, ProjectStatusCommand, NewProjectCommand,
    AddClipCommand, RazorCommand, AddTransitionCommand, UndoCommand, RedoCommand, SetPlayheadCommand,
};
pub use registry::CommandRegistry;
pub use shortcuts::{ShortcutManager, ShortcutSpec, ShortcutError, ShortcutConflict, Platform, Modifier, KeyBinding};
pub use undo::{UndoManager, UndoEntry, Transaction};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Unique identifier for a command
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(pub String);

impl CommandId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for CommandId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for CommandId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Command execution priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Command execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Synchronous,
    AsyncJob,
    RenderThread,
    AudioThread,
}

/// Command categories for menu organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandCategory {
    File,
    Edit,
    Sequence,
    Clip,
    Timeline,
    Graphics,
    Color,
    Audio,
    Effects,
    Transitions,
    MultiCam,
    Transcription,
    Export,
    Workspace,
    View,
    Window,
    Help,
    Other,
}

impl std::fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "File"),
            Self::Edit => write!(f, "Edit"),
            Self::Sequence => write!(f, "Sequence"),
            Self::Clip => write!(f, "Clip"),
            Self::Timeline => write!(f, "Timeline"),
            Self::Graphics => write!(f, "Graphics"),
            Self::Color => write!(f, "Color"),
            Self::Audio => write!(f, "Audio"),
            Self::Effects => write!(f, "Effects"),
            Self::Transitions => write!(f, "Transitions"),
            Self::MultiCam => write!(f, "MultiCam"),
            Self::Transcription => write!(f, "Transcription"),
            Self::Export => write!(f, "Export"),
            Self::Workspace => write!(f, "Workspace"),
            Self::View => write!(f, "View"),
            Self::Window => write!(f, "Window"),
            Self::Help => write!(f, "Help"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// Command metadata for UI presentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    pub id: CommandId,
    pub label: String,
    pub menu: Vec<String>,
    pub shortcut: Option<String>,
    pub params: Vec<ParamSpec>,
    pub mutating: bool,
    pub priority: CommandPriority,
    pub mode: ExecutionMode,
    pub category: CommandCategory,
    pub condition: Option<String>,
    pub help: Option<String>,
    pub icon: Option<String>,
}

impl CommandSpec {
    pub fn new(id: impl Into<CommandId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            menu: Vec::new(),
            shortcut: None,
            params: Vec::new(),
            mutating: true,
            priority: CommandPriority::Normal,
            mode: ExecutionMode::Synchronous,
            category: CommandCategory::Other,
            condition: None,
            help: None,
            icon: None,
        }
    }

    pub fn with_menu(mut self, menu: Vec<impl Into<String>>) -> Self {
        self.menu = menu.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn with_params(mut self, params: Vec<ParamSpec>) -> Self {
        self.params = params;
        self
    }

    pub fn with_category(mut self, category: CommandCategory) -> Self {
        self.category = category;
        self
    }

    pub fn non_mutating(mut self) -> Self {
        self.mutating = false;
        self
    }

    pub fn with_priority(mut self, priority: CommandPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub execution_id: Uuid,
    pub command_id: CommandId,
    pub success: bool,
    pub value: Option<Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub undo_id: Option<Uuid>,
}

impl CommandResult {
    pub fn success(value: impl Serialize) -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            command_id: CommandId::new(""),
            success: true,
            value: Some(serde_json::to_value(value).unwrap_or_default()),
            error: None,
            duration_ms: 0,
            undo_id: None,
        }
    }

    pub fn error(command_id: CommandId, error: impl Into<String>) -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            command_id,
            success: false,
            value: None,
            error: Some(error.into()),
            duration_ms: 0,
            undo_id: None,
        }
    }
}

/// Registry errors
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Command not found: {0}")]
    NotFound(CommandId),
    #[error("Command already registered: {0}")]
    AlreadyRegistered(CommandId),
    #[error("Invalid command spec: {0}")]
    InvalidSpec(String),
    #[error("Parameter validation failed: {0}")]
    ValidationError(#[from] ValidationError),
    #[error("Handler error: {0}")]
    HandlerError(#[from] HandlerError),
    #[error("Circular dependency detected")]
    CircularDependency,
    #[error("Shortcut conflict: {0}")]
    ShortcutConflict(String),
}

/// Menu structure for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuCategory {
    pub category: CommandCategory,
    pub label: String,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: CommandId,
    pub label: String,
    pub shortcut: Option<String>,
    pub icon: Option<String>,
    pub enabled: bool,
}

/// Validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Required parameter missing: {0}")]
    MissingRequired(String),
    #[error("Parameter '{param}' type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { param: String, expected: String, actual: String },
    #[error("Parameter '{param}' value out of range: {message}")]
    OutOfRange { param: String, message: String },
    #[error("Parameter '{param}' string length invalid: {message}")]
    StringLength { param: String, message: String },
    #[error("Parameter '{param}' pattern mismatch: {pattern}")]
    PatternMismatch { param: String, pattern: String },
    #[error("Parameter '{param}' enum value not allowed: {value}")]
    EnumMismatch { param: String, value: Value },
    #[error("Parameter '{param}' array length invalid: {message}")]
    ArrayLength { param: String, message: String },
    #[error("Parameter '{param}' object missing required fields: {fields:?}")]
    MissingFields { param: String, fields: Vec<String> },
    #[error("Parameter '{param}' additional properties not allowed: {extra:?}")]
    AdditionalProperties { param: String, extra: Vec<String> },
    #[error("Parameter '{param}' format validation failed: {format:?}")]
    FormatError { param: String, format: StringFormat },
    #[error("Parameter '{param}' custom validation failed: {message}")]
    Custom { param: String, message: String },
}

/// Handler errors
#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Parameter validation failed: {0}")]
    Validation(#[from] ValidationError),
    #[error("Command not available: {0}")]
    NotAvailable(String),
    #[error("Execution failed: {0}")]
    Execution(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Cancelled")]
    Cancelled,
}

impl From<RegistryError> for HandlerError {
    fn from(e: RegistryError) -> Self {
        HandlerError::Execution(e.to_string())
    }
}

/// String format validations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StringFormat {
    Email,
    Uri,
    Uuid,
    DateTime,
    Timecode,
    FilePath,
    HexColor,
    Base64,
}

/// Track kind for track references
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
    Subtitle,
    Submix,
    Any,
}

/// JSON Schema-like parameter specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParamSchema {
    String {
        #[serde(default)]
        min_length: Option<usize>,
        #[serde(default)]
        max_length: Option<usize>,
        #[serde(default)]
        pattern: Option<String>,
        #[serde(default)]
        format: Option<StringFormat>,
    },
    Integer {
        #[serde(default)]
        minimum: Option<i64>,
        #[serde(default)]
        maximum: Option<i64>,
        #[serde(default)]
        multiple_of: Option<i64>,
    },
    Number {
        #[serde(default)]
        minimum: Option<f64>,
        #[serde(default)]
        maximum: Option<f64>,
        #[serde(default)]
        multiple_of: Option<f64>,
    },
    Boolean,
    Array {
        #[serde(default)]
        min_items: Option<usize>,
        #[serde(default)]
        max_items: Option<usize>,
        #[serde(default)]
        unique_items: bool,
        items: Box<ParamSchema>,
    },
    Object {
        #[serde(default)]
        properties: std::collections::HashMap<String, ParamSchema>,
        #[serde(default)]
        required: Vec<String>,
        #[serde(default)]
        additional_properties: bool,
    },
    Enum {
        values: Vec<Value>,
    },
    Time,
    Timecode,
    Frame,
    File {
        #[serde(default)]
        extensions: Vec<String>,
        #[serde(default)]
        must_exist: bool,
    },
    Directory,
    Color,
    Track {
        #[serde(default)]
        kind: Option<TrackKind>,
    },
    Clip,
    Media,
    Any,
}

impl ParamSchema {
    /// Validate a value against this schema
    pub fn validate(&self, param_name: &str, value: &Value) -> Result<(), ValidationError> {
        match self {
            ParamSchema::String { min_length, max_length, pattern, format } => {
                let s = value.as_str().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "string".to_string(),
                    actual: value_type_name(value),
                })?;

                if let Some(min) = min_length {
                    if s.len() < *min {
                        return Err(ValidationError::StringLength {
                            param: param_name.to_string(),
                            message: format!("minimum length is {min}"),
                        });
                    }
                }
                if let Some(max) = max_length {
                    if s.len() > *max {
                        return Err(ValidationError::StringLength {
                            param: param_name.to_string(),
                            message: format!("maximum length is {max}"),
                        });
                    }
                }
                if let Some(pat) = pattern {
                    let regex = regex::Regex::new(pat).map_err(|_| ValidationError::Custom {
                        param: param_name.to_string(),
                        message: "invalid regex pattern".to_string(),
                    })?;
                    if !regex.is_match(s) {
                        return Err(ValidationError::PatternMismatch {
                            param: param_name.to_string(),
                            pattern: pat.clone(),
                        });
                    }
                }
                if let Some(fmt) = format {
                    validate_string_format(param_name, s, *fmt)?;
                }
                Ok(())
            }

            ParamSchema::Integer { minimum, maximum, multiple_of } => {
                let n = value.as_i64().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "integer".to_string(),
                    actual: value_type_name(value),
                })?;

                if let Some(min) = minimum {
                    if n < *min {
                        return Err(ValidationError::OutOfRange {
                            param: param_name.to_string(),
                            message: format!("minimum value is {min}"),
                        });
                    }
                }
                if let Some(max) = maximum {
                    if n > *max {
                        return Err(ValidationError::OutOfRange {
                            param: param_name.to_string(),
                            message: format!("maximum value is {max}"),
                        });
                    }
                }
                if let Some(mult) = multiple_of {
                    if n % *mult != 0 {
                        return Err(ValidationError::OutOfRange {
                            param: param_name.to_string(),
                            message: format!("must be multiple of {mult}"),
                        });
                    }
                }
                Ok(())
            }

            ParamSchema::Number { minimum, maximum, multiple_of } => {
                let n = value.as_f64().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "number".to_string(),
                    actual: value_type_name(value),
                })?;

                if let Some(min) = minimum {
                    if n < *min {
                        return Err(ValidationError::OutOfRange {
                            param: param_name.to_string(),
                            message: format!("minimum value is {min}"),
                        });
                    }
                }
                if let Some(max) = maximum {
                    if n > *max {
                        return Err(ValidationError::OutOfRange {
                            param: param_name.to_string(),
                            message: format!("maximum value is {max}"),
                        });
                    }
                }
                if let Some(mult) = multiple_of {
                    if (n / *mult).fract() != 0.0 {
                        return Err(ValidationError::OutOfRange {
                            param: param_name.to_string(),
                            message: format!("must be multiple of {mult}"),
                        });
                    }
                }
                Ok(())
            }

            ParamSchema::Boolean => {
                if !value.is_boolean() {
                    return Err(ValidationError::TypeMismatch {
                        param: param_name.to_string(),
                        expected: "boolean".to_string(),
                        actual: value_type_name(value),
                    });
                }
                Ok(())
            }

            ParamSchema::Array { min_items, max_items, unique_items, items } => {
                let arr = value.as_array().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "array".to_string(),
                    actual: value_type_name(value),
                })?;

                if let Some(min) = min_items {
                    if arr.len() < *min {
                        return Err(ValidationError::ArrayLength {
                            param: param_name.to_string(),
                            message: format!("minimum {min} items"),
                        });
                    }
                }
                if let Some(max) = max_items {
                    if arr.len() > *max {
                        return Err(ValidationError::ArrayLength {
                            param: param_name.to_string(),
                            message: format!("maximum {max} items"),
                        });
                    }
                }
                if *unique_items {
                    let mut seen = std::collections::HashSet::new();
                    for (i, item) in arr.iter().enumerate() {
                        let key = serde_json::to_string(item).unwrap_or_default();
                        if !seen.insert(key) {
                            return Err(ValidationError::ArrayLength {
                                param: param_name.to_string(),
                                message: format!("duplicate item at index {i}"),
                            });
                        }
                    }
                }
                for (i, item) in arr.iter().enumerate() {
                    items.validate(&format!("{param_name}[{i}]"), item)?;
                }
                Ok(())
            }

            ParamSchema::Object { properties, required, additional_properties } => {
                let obj = value.as_object().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "object".to_string(),
                    actual: value_type_name(value),
                })?;

                for req in required {
                    if !obj.contains_key(req) {
                        return Err(ValidationError::MissingFields {
                            param: param_name.to_string(),
                            fields: vec![req.clone()],
                        });
                    }
                }

                if !*additional_properties {
                    let allowed: std::collections::HashSet<_> = properties.keys().collect();
                    let extra: Vec<String> = obj.keys()
                        .filter(|k| !allowed.contains(*k))
                        .cloned()
                        .collect();
                    if !extra.is_empty() {
                        return Err(ValidationError::AdditionalProperties {
                            param: param_name.to_string(),
                            extra,
                        });
                    }
                }

                for (key, val) in obj {
                    if let Some(prop_schema) = properties.get(key) {
                        prop_schema.validate(&format!("{param_name}.{key}"), val)?;
                    }
                }
                Ok(())
            }

            ParamSchema::Enum { values } => {
                if !values.contains(value) {
                    return Err(ValidationError::EnumMismatch {
                        param: param_name.to_string(),
                        value: value.clone(),
                    });
                }
                Ok(())
            }

            ParamSchema::Time => {
                if !value.is_i64() && !value.is_u64() {
                    return Err(ValidationError::TypeMismatch {
                        param: param_name.to_string(),
                        expected: "integer (ticks)".to_string(),
                        actual: value_type_name(value),
                    });
                }
                Ok(())
            }

            ParamSchema::Timecode => {
                let s = value.as_str().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "timecode string".to_string(),
                    actual: value_type_name(value),
                })?;
                validate_timecode(s).map_err(|_| ValidationError::FormatError {
                    param: param_name.to_string(),
                    format: StringFormat::Timecode,
                })?;
                Ok(())
            }

            ParamSchema::Frame => {
                if !value.is_i64() && !value.is_u64() {
                    return Err(ValidationError::TypeMismatch {
                        param: param_name.to_string(),
                        expected: "frame number".to_string(),
                        actual: value_type_name(value),
                    });
                }
                Ok(())
            }

            ParamSchema::File { extensions, must_exist } => {
                let s = value.as_str().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "file path string".to_string(),
                    actual: value_type_name(value),
                })?;
                if !extensions.is_empty() {
                    let ext = std::path::Path::new(s)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if !extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                        return Err(ValidationError::Custom {
                            param: param_name.to_string(),
                            message: format!("file extension must be one of: {}", extensions.join(", ")),
                        });
                    }
                }
                if *must_exist && !std::path::Path::new(s).exists() {
                    return Err(ValidationError::Custom {
                        param: param_name.to_string(),
                        message: "file does not exist".to_string(),
                    });
                }
                Ok(())
            }

            ParamSchema::Directory => {
                let s = value.as_str().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "directory path string".to_string(),
                    actual: value_type_name(value),
                })?;
                if !std::path::Path::new(s).is_dir() {
                    return Err(ValidationError::Custom {
                        param: param_name.to_string(),
                        message: "directory does not exist".to_string(),
                    });
                }
                Ok(())
            }

            ParamSchema::Color => {
                let s = value.as_str().ok_or_else(|| ValidationError::TypeMismatch {
                    param: param_name.to_string(),
                    expected: "color string".to_string(),
                    actual: value_type_name(value),
                })?;
                validate_color(s).map_err(|_| ValidationError::FormatError {
                    param: param_name.to_string(),
                    format: StringFormat::HexColor,
                })?;
                Ok(())
            }

            ParamSchema::Track { kind } => {
                if !value.is_u64() && !value.is_i64() {
                    return Err(ValidationError::TypeMismatch {
                        param: param_name.to_string(),
                        expected: "track ID".to_string(),
                        actual: value_type_name(value),
                    });
                }
                Ok(())
            }

            ParamSchema::Clip => {
                if !value.is_u64() && !value.is_i64() {
                    return Err(ValidationError::TypeMismatch {
                        param: param_name.to_string(),
                        expected: "clip ID".to_string(),
                        actual: value_type_name(value),
                    });
                }
                Ok(())
            }

            ParamSchema::Media => {
                if !value.is_u64() && !value.is_i64() {
                    return Err(ValidationError::TypeMismatch {
                        param: param_name.to_string(),
                        expected: "media ID".to_string(),
                        actual: value_type_name(value),
                    });
                }
                Ok(())
            }

            ParamSchema::Any => Ok(()),
        }
    }

    /// Get JSON Schema representation for documentation
    pub fn to_json_schema(&self) -> Value {
        match self {
            ParamSchema::String { min_length, max_length, pattern, format } => {
                let mut schema = serde_json::json!({ "type": "string" });
                if let Some(min) = min_length { schema["minLength"] = Value::from(*min); }
                if let Some(max) = max_length { schema["maxLength"] = Value::from(*max); }
                if let Some(pat) = pattern { schema["pattern"] = Value::from(pat.as_str()); }
                if let Some(fmt) = format { schema["format"] = Value::from(format!("{fmt:?}").to_lowercase()); }
                schema
            }
            ParamSchema::Integer { minimum, maximum, multiple_of } => {
                let mut schema = serde_json::json!({ "type": "integer" });
                if let Some(min) = minimum { schema["minimum"] = Value::from(*min); }
                if let Some(max) = maximum { schema["maximum"] = Value::from(*max); }
                if let Some(mult) = multiple_of { schema["multipleOf"] = Value::from(*mult); }
                schema
            }
            ParamSchema::Number { minimum, maximum, multiple_of } => {
                let mut schema = serde_json::json!({ "type": "number" });
                if let Some(min) = minimum { schema["minimum"] = Value::from(*min); }
                if let Some(max) = maximum { schema["maximum"] = Value::from(*max); }
                if let Some(mult) = multiple_of { schema["multipleOf"] = Value::from(*mult); }
                schema
            }
            ParamSchema::Boolean => serde_json::json!({ "type": "boolean" }),
            ParamSchema::Array { min_items, max_items, unique_items, items } => {
                let mut schema = serde_json::json!({ "type": "array" });
                if let Some(min) = min_items { schema["minItems"] = Value::from(*min); }
                if let Some(max) = max_items { schema["maxItems"] = Value::from(*max); }
                if *unique_items { schema["uniqueItems"] = Value::from(true); }
                schema["items"] = items.to_json_schema();
                schema
            }
            ParamSchema::Object { properties, required, additional_properties } => {
                let mut schema = serde_json::json!({ "type": "object" });
                let props: serde_json::Map<String, Value> = properties.iter()
                    .map(|(k, v)| (k.clone(), v.to_json_schema()))
                    .collect();
                schema["properties"] = Value::Object(props);
                if !required.is_empty() {
                    schema["required"] = Value::Array(required.iter().map(|s| Value::from(s.as_str())).collect());
                }
                schema["additionalProperties"] = Value::Bool(*additional_properties);
                schema
            }
            ParamSchema::Enum { values } => {
                serde_json::json!({ "enum": values })
            }
            ParamSchema::Time => serde_json::json!({ "type": "integer", "format": "ticks" }),
            ParamSchema::Timecode => serde_json::json!({ "type": "string", "format": "timecode" }),
            ParamSchema::Frame => serde_json::json!({ "type": "integer", "format": "frame" }),
            ParamSchema::File { extensions, must_exist } => {
                let mut schema = serde_json::json!({ "type": "string", "format": "file-path" });
                if !extensions.is_empty() {
                    schema["extensions"] = Value::Array(extensions.iter().map(|s| Value::from(s.as_str())).collect());
                }
                schema["mustExist"] = Value::Bool(*must_exist);
                schema
            }
            ParamSchema::Directory => serde_json::json!({ "type": "string", "format": "directory" }),
            ParamSchema::Color => serde_json::json!({ "type": "string", "format": "color" }),
            ParamSchema::Track { kind } => {
                let mut schema = serde_json::json!({ "type": "integer", "format": "track-id" });
                if let Some(k) = kind { schema["trackKind"] = Value::from(format!("{k:?}").to_lowercase()); }
                schema
            }
            ParamSchema::Clip => serde_json::json!({ "type": "integer", "format": "clip-id" }),
            ParamSchema::Media => serde_json::json!({ "type": "integer", "format": "media-id" }),
            ParamSchema::Any => serde_json::json!({}),
        }
    }
}

fn value_type_name(value: &Value) -> String {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() { "integer" } else { "number" }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }.to_string()
}

fn validate_string_format(param: &str, value: &str, format: StringFormat) -> Result<(), ValidationError> {
    match format {
        StringFormat::Email => {
            if !value.contains('@') || !value.contains('.') {
                return Err(ValidationError::FormatError { param: param.to_string(), format });
            }
        }
        StringFormat::Uri => {
            if url::Url::parse(value).is_err() {
                return Err(ValidationError::FormatError { param: param.to_string(), format });
            }
        }
        StringFormat::Uuid => {
            if uuid::Uuid::parse_str(value).is_err() {
                return Err(ValidationError::FormatError { param: param.to_string(), format });
            }
        }
        StringFormat::DateTime => {
            if chrono::DateTime::parse_from_rfc3339(value).is_err() {
                return Err(ValidationError::FormatError { param: param.to_string(), format });
            }
        }
        StringFormat::Timecode => {
            validate_timecode(value).map_err(|_| ValidationError::FormatError {
                param: param.to_string(),
                format,
            })?;
        }
        StringFormat::FilePath => {
            if std::path::Path::new(value).components().count() == 0 {
                return Err(ValidationError::FormatError { param: param.to_string(), format });
            }
        }
        StringFormat::HexColor => {
            validate_color(value).map_err(|_| ValidationError::FormatError {
                param: param.to_string(),
                format,
            })?;
        }
        StringFormat::Base64 => {
            if base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value).is_err() {
                return Err(ValidationError::FormatError { param: param.to_string(), format });
            }
        }
    }
    Ok(())
}

fn validate_timecode(tc: &str) -> Result<(), String> {
    let parts: Vec<&str> = tc.split(|c| c == ':' || c == ';').collect();
    if parts.len() != 4 {
        return Err("timecode must have 4 components".to_string());
    }
    for (i, part) in parts.iter().enumerate() {
        if part.parse::<u32>().is_err() {
            return Err(format!("timecode component {i} is not a number"));
        }
    }
    Ok(())
}

fn validate_color(color: &str) -> Result<(), String> {
    let c = color.trim().trim_start_matches('#');
    match c.len() {
        3 => {
            if !c.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err("invalid 3-digit hex color".to_string());
            }
        }
        6 => {
            if !c.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err("invalid 6-digit hex color".to_string());
            }
        }
        8 => {
            if !c.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err("invalid 8-digit hex color (with alpha)".to_string());
            }
        }
        _ => {
            return Err("unsupported color format".to_string());
        }
    }
    Ok(())
}

/// Parameter metadata for UI presentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSpec {
    pub name: String,
    pub label: String,
    pub schema: ParamSchema,
    pub required: bool,
    pub default: Option<Value>,
    pub description: Option<String>,
    pub enum_values: Option<Vec<Value>>,
    pub properties: Option<std::collections::HashMap<String, ParamSpec>>,
    pub items: Option<Box<ParamSpec>>,
}

impl ParamSpec {
    pub fn new(name: impl Into<String>, label: impl Into<String>, schema: ParamSchema) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            schema,
            required: true,
            default: None,
            description: None,
            enum_values: None,
            properties: None,
            items: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self.required = false;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_enum(mut self, values: Vec<Value>) -> Self {
        self.enum_values = Some(values);
        self
    }
}

/// Parameter type for command parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
    Time,
    Timecode,
    Frame,
    File,
    Directory,
    Color,
    Track,
    Clip,
    Media,
    Any,
}

/// Command handler trait
#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    fn spec(&self) -> CommandSpec;
    async fn execute(&self, ctx: crate::context::ExecutionContext<'_>) -> Result<Value, crate::HandlerError>;
    async fn is_available(&self, _ctx: &crate::context::AppContext) -> bool { true }
    async fn get_state(&self, _ctx: &crate::context::AppContext) -> Option<Value> { None }
}

#[cfg(test)]
mod tests {
    use super::*;
use serde_json::json;

    #[test]
    fn test_command_spec_builder() {
        let spec = crate::CommandSpec::new("test.command", "Test Command")
            .with_menu(vec!["Test", "Commands"])
            .with_shortcut("Ctrl+T")
            .with_category(CommandCategory::Edit)
            .with_help("This is a test command")
            .non_mutating();

        assert_eq!(spec.id.as_str(), "test.command");
        assert_eq!(spec.label, "Test Command");
        assert_eq!(spec.menu, vec!["Test", "Commands"]);
        assert_eq!(spec.shortcut, Some("Ctrl+T".to_string()));
        assert_eq!(spec.category, CommandCategory::Edit);
        assert_eq!(spec.help, Some("This is a test command".to_string()));
        assert!(!spec.mutating);
    }

    #[test]
    fn test_command_id() {
        let id: CommandId = "test.id".into();
        assert_eq!(id.as_str(), "test.id");
        assert_eq!(id.to_string(), "test.id");
    }

    #[test]
    fn test_command_result() {
        let result = crate::CommandResult::success(json!({ "value": 42 }));
        assert!(result.success);
        assert_eq!(result.value, Some(json!({ "value": 42 })));

        let err = crate::CommandResult::error(CommandId::new("test"), "failed");
        assert!(!err.success);
        assert_eq!(err.error, Some("failed".to_string()));
    }
}