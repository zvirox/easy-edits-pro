//! Additional MCP Tools for AI-assisted editing workflows

use std::sync::Arc;
use rmcp::{
    ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, ListToolsResult, Tool,
        TextContent, ToolAnnotations,
    },
    Error as McpError,
};
use saturn_commands::{
    CommandRegistry, CommandSpec,
    context::{AppContext, ExecutionContext},
    undo::UndoManager,
    shortcuts::ShortcutManager,
    CommandId, CommandResult, CommandCategory, ValidationError, HandlerError,
};
use serde_json::{json, Value};
use uuid::Uuid;
use tracing::{debug, info};

/// Extended MCP tools for advanced AI-assisted editing
pub struct ExtendedMcpTools {
    app_context: Arc<AppContext>,
    registry: Arc<CommandRegistry>,
}

impl ExtendedMcpTools {
    pub fn new(app_context: Arc<AppContext>) -> Self {
        Self {
            app_context: app_context.clone(),
            registry: app_context.registry.clone(),
        }
    }

    /// Get all tools including extended editing tools
    pub fn get_all_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();

        // Base commands
        let specs = self.registry.all_specs();
        for spec in specs {
            tools.push(self.command_to_tool(spec));
        }

        // Extended AI workflow tools
        tools.push(self.analyze_timeline_tool());
        tools.push(self.suggest_edit_tool());
        tools.push(self.batch_operation_tool());
        tools.push(self.search_media_tool());
        tools.push(self.create_template_tool());

        tools
    }

    fn command_to_tool(&self, spec: &CommandSpec) -> Tool {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &spec.params {
            let schema = self.param_schema_to_json_schema(&param.schema);
            properties.insert(param.name.clone(), schema);
            if param.required {
                required.push(param.name.clone());
            }
        }

        Tool {
            name: spec.id.to_string(),
            description: spec.help.clone().unwrap_or_else(|| spec.label.clone()),
            input_schema: json!({
                "type": "object",
                "properties": properties,
                "required": required,
            }),
            annotations: Some(ToolAnnotations {
                title: Some(spec.label.clone()),
                read_only_hint: Some(!spec.mutating),
                destructive_hint: None,
                idempotent_hint: None,
                open_world_hint: None,
            }),
        }
    }

    fn param_schema_to_json_schema(&self, schema: &saturn_commands::ParamSchema) -> Value {
        match schema {
            saturn_commands::ParamSchema::String { min_length, max_length, pattern, format } => {
                let mut s = json!({ "type": "string" });
                if let Some(min) = min_length { s["minLength"] = json!(min); }
                if let Some(max) = max_length { s["maxLength"] = json!(max); }
                if let Some(pat) = pattern { s["pattern"] = json!(pat); }
                if let Some(fmt) = format { s["format"] = json!(format!("{:?}", fmt).to_lowercase()); }
                s
            }
            saturn_commands::ParamSchema::Integer { minimum, maximum, multiple_of } => {
                let mut s = json!({ "type": "integer" });
                if let Some(min) = minimum { s["minimum"] = json!(min); }
                if let Some(max) = maximum { s["maximum"] = json!(max); }
                if let Some(mult) = multiple_of { s["multipleOf"] = json!(mult); }
                s
            }
            saturn_commands::ParamSchema::Number { minimum, maximum, multiple_of } => {
                let mut s = json!({ "type": "number" });
                if let Some(min) = minimum { s["minimum"] = json!(min); }
                if let Some(max) = maximum { s["maximum"] = json!(max); }
                if let Some(mult) = multiple_of { s["multipleOf"] = json!(mult); }
                s
            }
            saturn_commands::ParamSchema::Boolean => json!({ "type": "boolean" }),
            saturn_commands::ParamSchema::Array { items, .. } => {
                json!({ "type": "array", "items": self.param_schema_to_json_schema(items) })
            }
            saturn_commands::ParamSchema::Object { properties, required, .. } => {
                let mut props = serde_json::Map::new();
                for (k, v) in properties {
                    props.insert(k.clone(), self.param_schema_to_json_schema(v));
                }
                let mut s = json!({ "type": "object", "properties": props });
                if !required.is_empty() {
                    s["required"] = json!(required);
                }
                s
            }
            saturn_commands::ParamSchema::Enum { values } => json!({ "enum": values }),
            saturn_commands::ParamSchema::Time => json!({ "type": "integer", "format": "ticks" }),
            saturn_commands::ParamSchema::Timecode => json!({ "type": "string", "format": "timecode" }),
            saturn_commands::ParamSchema::Frame => json!({ "type": "integer", "format": "frame" }),
            saturn_commands::ParamSchema::File { extensions, .. } => {
                let mut s = json!({ "type": "string", "format": "file-path" });
                if !extensions.is_empty() {
                    s["extensions"] = json!(extensions);
                }
                s
            }
            saturn_commands::ParamSchema::Directory => json!({ "type": "string", "format": "directory" }),
            saturn_commands::ParamSchema::Color => json!({ "type": "string", "format": "color" }),
            saturn_commands::ParamSchema::Track { kind } => {
                let mut s = json!({ "type": "integer", "format": "track-id" });
                if let Some(k) = kind { s["trackKind"] = json!(format!("{:?}", k).to_lowercase()); }
                s
            }
            saturn_commands::ParamSchema::Clip => json!({ "type": "integer", "format": "clip-id" }),
            saturn_commands::ParamSchema::Media => json!({ "type": "integer", "format": "media-id" }),
            saturn_commands::ParamSchema::Any => json!({}),
        }
    }

    /// Tool: Analyze timeline structure and suggest edits
    fn analyze_timeline_tool(&self) -> Tool {
        Tool {
            name: "saturn.analyzeTimeline".to_string(),
            description: "Analyze timeline structure, detect gaps, overlaps, and suggest optimizations".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "sequence_id": { "type": "string", "description": "Sequence to analyze" },
                    "analyze_gaps": { "type": "boolean", "default": true },
                    "analyze_overlaps": { "type": "boolean", "default": true },
                    "suggest_transitions": { "type": "boolean", "default": true },
                },
                "required": ["sequence_id"]
            }),
            annotations: Some(ToolAnnotations {
                title: Some("Analyze Timeline".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: None,
            }),
        }
    }

    /// Tool: Suggest edits based on context
    fn suggest_edit_tool(&self) -> Tool {
        Tool {
            name: "saturn.suggestEdit".to_string(),
            description: "Suggest edits based on current context, selected clips, and playhead position".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "context": { "type": "string", "description": "Natural language description of desired edit" },
                    "selected_clips": { "type": "array", "items": { "type": "string" }, "description": "Currently selected clip IDs" },
                    "playhead_position": { "type": "integer", "description": "Current playhead position in ticks" },
                },
                "required": ["context"]
            }),
            annotations: Some(ToolAnnotations {
                title: Some("Suggest Edit".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(false),
                open_world_hint: None,
            }),
        }
    }

    /// Tool: Execute batch operations atomically
    fn batch_operation_tool(&self) -> Tool {
        Tool {
            name: "saturn.batchOperation".to_string(),
            description: "Execute multiple commands atomically with single undo entry".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "operations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string" },
                                "params": { "type": "object" }
                            },
                            "required": ["command", "params"]
                        }
                    },
                    "description": { "type": "string", "description": "Description for undo entry" }
                },
                "required": ["operations"]
            }),
            annotations: Some(ToolAnnotations {
                title: Some("Batch Operation".to_string()),
                read_only_hint: Some(false),
                destructive_hint: Some(true),
                idempotent_hint: Some(false),
                open_world_hint: None,
            }),
        }
    }

    /// Tool: Search media with AI-friendly queries
    fn search_media_tool(&self) -> Tool {
        Tool {
            name: "saturn.searchMedia".to_string(),
            description: "Search media bins with natural language queries (e.g., 'close up shots of person talking')".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "filters": {
                        "type": "object",
                        "properties": {
                            "media_type": { "type": "string", "enum": ["video", "audio", "image", "subtitle"] },
                            "duration_min": { "type": "number" },
                            "duration_max": { "type": "number" },
                            "tags": { "type": "array", "items": { "type": "string" } },
                            "bin_id": { "type": "string" }
                        }
                    },
                    "limit": { "type": "integer", "default": 20 }
                },
                "required": ["query"]
            }),
            annotations: Some(ToolAnnotations {
                title: Some("Search Media".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: None,
            }),
        }
    }

    /// Tool: Create project/template from AI specifications
    fn create_template_tool(&self) -> Tool {
        Tool {
            name: "saturn.createTemplate".to_string(),
            description: "Create project template or graphics template from AI specification".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "template_type": { "type": "string", "enum": ["project", "graphics", "sequence", "bin"] },
                    "specification": { "type": "object", "description": "JSON specification of template structure" },
                    "name": { "type": "string", "description": "Template name" },
                    "description": { "type": "string", "description": "Template description" },
                    "tags": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["template_type", "specification", "name"]
            }),
            annotations: Some(ToolAnnotations {
                title: Some("Create Template".to_string()),
                read_only_hint: Some(false),
                destructive_hint: Some(false),
                idempotent_hint: Some(false),
                open_world_hint: None,
            }),
        }
    }
}