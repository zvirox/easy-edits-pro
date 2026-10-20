//! MCP Server Handler Implementation

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
    CommandRegistry, CommandHandler, CommandSpec,
    context::{AppContext, AppState, AppEvent, ExecutionContext},
    undo::UndoManager,
    shortcuts::ShortcutManager,
    CommandId, CommandResult, CommandSpec, CommandCategory, CommandPriority, ExecutionMode,
    ValidationError, HandlerError, RegistryError,
};
use serde_json::{json, Value};
use uuid::Uuid;
use tracing::{debug, info, warn};

/// MCP Server Handler that exposes command registry to AI assistants
pub struct McpServerHandler {
    app_context: Arc<AppContext>,
    registry: Arc<CommandRegistry>,
}

impl McpServerHandler {
    pub fn new(app_context: Arc<AppContext>) -> Self {
        let registry = app_context.registry.clone();
        Self {
            app_context,
            registry,
        }
    }

    /// Convert command spec to MCP tool
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
}

#[rmcp::async_trait]
impl ServerHandler for McpServerHandler {
    async fn list_tools(&self, _request: Option<()>) -> Result<ListToolsResult, McpError> {
        let specs = self.registry.all_specs();
        let tools: Vec<Tool> = specs.iter()
            .map(|spec| self.command_to_tool(spec))
            .collect();

        Ok(ListToolsResult { tools })
    }

    async fn call_tool(&self, request: CallToolRequestParam) -> Result<CallToolResult, McpError> {
        let command_id = CommandId::new(request.name.clone());
        let params = request.arguments.unwrap_or_default();

        debug!("Executing command: {} with params: {}", command_id, params);

        // Check if command exists
        let handler = self.registry.get(&command_id)
            .ok_or_else(|| McpError::invalid_params(format!("Command not found: {}", command_id), None))?;

        // Execute command
        let execution_context = ExecutionContext {
            app: &self.app_context,
            command: self.registry.get_spec(&command_id)
                .ok_or_else(|| McpError::internal_error("Command spec not found", None))?,
            params: params.clone(),
            execution_id: Uuid::new_v4(),
            start_time: std::time::Instant::now(),
        };

        let result = handler.execute(execution_context).await
            .map_err(|e| McpError::internal_error(format!("Command execution failed: {}", e), None))?;

        Ok(CallToolResult {
            content: vec![TextContent {
                type: "text".to_string(),
                text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| json!({ "result": result }).to_string()),
                annotations: None,
            }],
            is_error: None,
        })
    }
}

/// Additional MCP tools for AI-assisted editing workflows
pub struct EditingTools;

impl EditingTools {
    /// Get available command categories for AI to understand command organization
    pub fn get_categories() -> Vec<&'static str> {
        vec![
            "File", "Edit", "Sequence", "Clip", "Timeline",
            "Graphics", "Color", "Audio", "Effects", "Transitions",
            "MultiCam", "Transcription", "Export", "Workspace",
            "View", "Window", "Help", "Other"
        ]
    }

    /// Suggest commands based on natural language query
    pub fn suggest_commands(query: &str, registry: &CommandRegistry) -> Vec<CommandSpec> {
        let specs = registry.all_specs();
        let query_lower = query.to_lowercase();

        specs.into_iter()
            .filter(|spec| {
                spec.label.to_lowercase().contains(&query_lower) ||
                spec.help.as_ref().map_or(false, |h| h.to_lowercase().contains(&query_lower)) ||
                spec.id.to_string().to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Get command suggestions for common editing workflows
    pub fn get_workflow_suggestions(workflow: &str, registry: &CommandRegistry) -> Vec<CommandSpec> {
        let workflow_commands = match workflow {
            "cut" => vec!["timeline.razor", "edit.cut", "edit.copy", "edit.paste"],
            "transition" => vec!["timeline.addTransition", "timeline.removeTransition"],
            "color" => vec!["color.correct", "color.grade", "color.lut"],
            "audio" => vec!["audio.mix", "audio.effects", "audio.sync"],
            "export" => vec!["export.render", "export.queue"],
            _ => vec![],
        };

        workflow_commands.iter()
            .filter_map(|id| registry.get_spec(&CommandId::new(*id)))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saturn_commands::*;

    #[tokio::test]
    async fn test_mcp_handler_creation() {
        let registry = Arc::new(CommandRegistry::new());
        register_builtin_commands(&registry).unwrap();

        let app_context = Arc::new(AppContext {
            state: Arc::new(tokio::sync::RwLock::new(AppState::default())),
            registry: registry.clone(),
            undo: Arc::new(UndoManager::new(100)),
            shortcuts: Arc::new(ShortcutManager::new()),
            events: Arc::new(EventBus::new()),
        });

        let handler = McpServerHandler::new(app_context);
        let tools = handler.list_tools(None).await.unwrap();

        assert!(!tools.tools.is_empty());
        assert!(tools.tools.iter().any(|t| t.name == "test.echo"));
    }
}