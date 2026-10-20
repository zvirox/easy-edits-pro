//! Parameter schema system with JSON Schema validation

use crate::{ParamType, StringFormat, TrackKind, ValidationError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
        properties: HashMap<String, ParamSchema>,
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
            ParamSchema::String {
                min_length,
                max_length,
                pattern,
                format,
            } => {
                let s = value
                    .as_str()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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

            ParamSchema::Integer {
                minimum,
                maximum,
                multiple_of,
            } => {
                let n = value
                    .as_i64()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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

            ParamSchema::Number {
                minimum,
                maximum,
                multiple_of,
            } => {
                let n = value
                    .as_f64()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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

            ParamSchema::Array {
                min_items,
                max_items,
                unique_items,
                items,
            } => {
                let arr = value
                    .as_array()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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

            ParamSchema::Object {
                properties,
                required,
                additional_properties,
            } => {
                let obj = value
                    .as_object()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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
                    let extra: Vec<String> = obj
                        .keys()
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
                let s = value
                    .as_str()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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

            ParamSchema::File {
                extensions,
                must_exist,
            } => {
                let s = value
                    .as_str()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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
                            message: format!(
                                "file extension must be one of: {}",
                                extensions.join(", ")
                            ),
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
                let s = value
                    .as_str()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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
                let s = value
                    .as_str()
                    .ok_or_else(|| ValidationError::TypeMismatch {
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
            ParamSchema::String {
                min_length,
                max_length,
                pattern,
                format,
            } => {
                let mut schema = serde_json::json!({ "type": "string" });
                if let Some(min) = min_length {
                    schema["minLength"] = Value::from(*min);
                }
                if let Some(max) = max_length {
                    schema["maxLength"] = Value::from(*max);
                }
                if let Some(pat) = pattern {
                    schema["pattern"] = Value::from(pat.as_str());
                }
                if let Some(fmt) = format {
                    schema["format"] = Value::from(format!("{fmt:?}").to_lowercase());
                }
                schema
            }
            ParamSchema::Integer {
                minimum,
                maximum,
                multiple_of,
            } => {
                let mut schema = serde_json::json!({ "type": "integer" });
                if let Some(min) = minimum {
                    schema["minimum"] = Value::from(*min);
                }
                if let Some(max) = maximum {
                    schema["maximum"] = Value::from(*max);
                }
                if let Some(mult) = multiple_of {
                    schema["multipleOf"] = Value::from(*mult);
                }
                schema
            }
            ParamSchema::Number {
                minimum,
                maximum,
                multiple_of,
            } => {
                let mut schema = serde_json::json!({ "type": "number" });
                if let Some(min) = minimum {
                    schema["minimum"] = Value::from(*min);
                }
                if let Some(max) = maximum {
                    schema["maximum"] = Value::from(*max);
                }
                if let Some(mult) = multiple_of {
                    schema["multipleOf"] = Value::from(*mult);
                }
                schema
            }
            ParamSchema::Boolean => serde_json::json!({ "type": "boolean" }),
            ParamSchema::Array {
                min_items,
                max_items,
                unique_items,
                items,
            } => {
                let mut schema = serde_json::json!({ "type": "array" });
                if let Some(min) = min_items {
                    schema["minItems"] = Value::from(*min);
                }
                if let Some(max) = max_items {
                    schema["maxItems"] = Value::from(*max);
                }
                if *unique_items {
                    schema["uniqueItems"] = Value::from(true);
                }
                schema["items"] = items.to_json_schema();
                schema
            }
            ParamSchema::Object {
                properties,
                required,
                additional_properties,
            } => {
                let mut schema = serde_json::json!({ "type": "object" });
                let props: serde_json::Map<String, Value> = properties
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_json_schema()))
                    .collect();
                schema["properties"] = Value::Object(props);
                if !required.is_empty() {
                    schema["required"] =
                        Value::Array(required.iter().map(|s| Value::from(s.as_str())).collect());
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
            ParamSchema::File {
                extensions,
                must_exist,
            } => {
                let mut schema = serde_json::json!({ "type": "string", "format": "file-path" });
                if !extensions.is_empty() {
                    schema["extensions"] =
                        Value::Array(extensions.iter().map(|s| Value::from(s.as_str())).collect());
                }
                schema["mustExist"] = Value::Bool(*must_exist);
                schema
            }
            ParamSchema::Directory => {
                serde_json::json!({ "type": "string", "format": "directory" })
            }
            ParamSchema::Color => serde_json::json!({ "type": "string", "format": "color" }),
            ParamSchema::Track { kind } => {
                let mut schema = serde_json::json!({ "type": "integer", "format": "track-id" });
                if let Some(k) = kind {
                    schema["trackKind"] = Value::from(format!("{k:?}").to_lowercase());
                }
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
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
    .to_string()
}

fn validate_string_format(
    param: &str,
    value: &str,
    format: StringFormat,
) -> Result<(), ValidationError> {
    match format {
        StringFormat::Email => {
            if !value.contains('@') || !value.contains('.') {
                return Err(ValidationError::FormatError {
                    param: param.to_string(),
                    format,
                });
            }
        }
        StringFormat::Uri => {
            if url::Url::parse(value).is_err() {
                return Err(ValidationError::FormatError {
                    param: param.to_string(),
                    format,
                });
            }
        }
        StringFormat::Uuid => {
            if uuid::Uuid::parse_str(value).is_err() {
                return Err(ValidationError::FormatError {
                    param: param.to_string(),
                    format,
                });
            }
        }
        StringFormat::DateTime => {
            if chrono::DateTime::parse_from_rfc3339(value).is_err() {
                return Err(ValidationError::FormatError {
                    param: param.to_string(),
                    format,
                });
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
                return Err(ValidationError::FormatError {
                    param: param.to_string(),
                    format,
                });
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
                return Err(ValidationError::FormatError {
                    param: param.to_string(),
                    format,
                });
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_string_validation() {
        let schema = ParamSchema::String {
            min_length: Some(3),
            max_length: Some(10),
            pattern: Some(r"^[a-z]+$".to_string()),
            format: None,
        };

        assert!(schema.validate("test", &json!("hello")).is_ok());
        assert!(schema.validate("test", &json!("hi")).is_err());
        assert!(schema.validate("test", &json!("hello world")).is_err());
        assert!(schema.validate("test", &json!("Hello")).is_err());
    }

    #[test]
    fn test_integer_validation() {
        let schema = ParamSchema::Integer {
            minimum: Some(0),
            maximum: Some(100),
            multiple_of: Some(5),
        };

        assert!(schema.validate("test", &json!(50)).is_ok());
        assert!(schema.validate("test", &json!(-1)).is_err());
        assert!(schema.validate("test", &json!(101)).is_err());
        assert!(schema.validate("test", &json!(7)).is_err());
    }

    #[test]
    fn test_enum_validation() {
        let schema = ParamSchema::Enum {
            values: vec![json!("a"), json!("b"), json!("c")],
        };

        assert!(schema.validate("test", &json!("a")).is_ok());
        assert!(schema.validate("test", &json!("d")).is_err());
    }

    #[test]
    fn test_timecode_validation() {
        let schema = ParamSchema::Timecode;
        assert!(schema.validate("test", &json!("01:02:03:04")).is_ok());
        assert!(schema.validate("test", &json!("01:02:03;04")).is_ok());
        assert!(schema.validate("test", &json!("01:02:03")).is_err());
        assert!(schema.validate("test", &json!("invalid")).is_err());
    }

    #[test]
    fn test_color_validation() {
        let schema = ParamSchema::Color;
        assert!(schema.validate("test", &json!("#ff0000")).is_ok());
        assert!(schema.validate("test", &json!("#f00")).is_ok());
        assert!(schema.validate("test", &json!("#ff0000ff")).is_ok());
        assert!(schema.validate("test", &json!("red")).is_err());
    }
}
