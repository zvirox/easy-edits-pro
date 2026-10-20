//! Command handler implementations and built-in commands

use crate::{
    CommandCategory, CommandSpec,
    context::{AppContext, ExecutionContext},
};
use async_trait::async_trait;
use serde_json::Value;

pub struct EchoCommand;

#[async_trait]
impl crate::CommandHandler for EchoCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("test.echo", "Echo")
            .with_category(CommandCategory::Other)
            .with_params(vec![
                crate::ParamSpec::new(
                    "message",
                    "Message",
                    crate::ParamSchema::String {
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        format: None,
                    },
                )
                .with_description("Message to echo back"),
            ])
            .non_mutating()
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let message: String = ctx.get("message")?;
        Ok(serde_json::json!({ "echo": message }))
    }
}

pub struct ProjectStatusCommand;

#[async_trait]
impl crate::CommandHandler for ProjectStatusCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("project.status", "Project Status")
            .with_category(CommandCategory::File)
            .non_mutating()
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let state = ctx.app.state.read();
        Ok(serde_json::json!({
            "has_project": state.project.is_some(),
            "playhead": state.playhead,
            "selection": state.selection,
        }))
    }
}

pub struct NewProjectCommand;

#[async_trait]
impl crate::CommandHandler for NewProjectCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("file.new", "New Project")
            .with_category(CommandCategory::File)
            .with_shortcut("Ctrl+N")
            .with_params(vec![
                crate::ParamSpec::new(
                    "name",
                    "Project Name",
                    crate::ParamSchema::String {
                        min_length: Some(1),
                        max_length: Some(100),
                        pattern: None,
                        format: None,
                    },
                ),
                crate::ParamSpec::new(
                    "width",
                    "Width",
                    crate::ParamSchema::Integer {
                        minimum: Some(640),
                        maximum: Some(8192),
                        multiple_of: None,
                    },
                )
                .with_default(serde_json::json!(1920)),
                crate::ParamSpec::new(
                    "height",
                    "Height",
                    crate::ParamSchema::Integer {
                        minimum: Some(360),
                        maximum: Some(8192),
                        multiple_of: None,
                    },
                )
                .with_default(serde_json::json!(1080)),
                crate::ParamSpec::new(
                    "frame_rate",
                    "Frame Rate",
                    crate::ParamSchema::Number {
                        minimum: Some(1.0),
                        maximum: Some(120.0),
                        multiple_of: None,
                    },
                )
                .with_default(serde_json::json!(30.0)),
            ])
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let name: String = ctx.get("name")?;
        let _width: i64 = ctx.get("width")?;
        let _height: i64 = ctx.get("height")?;
        let _frame_rate: f64 = ctx.get("frame_rate")?;

        let project_id = uuid::Uuid::new_v4();

        let mut state = ctx.app.state.write();
        state.project = Some(crate::context::ProjectHandle(project_id));
        state.playhead = 0;

        ctx.emit_event(crate::context::AppEvent::ProjectChanged { project_id });

        Ok(serde_json::json!({
            "project_id": project_id.to_string(),
            "name": name,
        }))
    }
}

pub struct AddClipCommand;

#[async_trait]
impl crate::CommandHandler for AddClipCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("timeline.addClip", "Add Clip to Timeline")
            .with_category(CommandCategory::Timeline)
            .with_params(vec![
                crate::ParamSpec::new(
                    "track_id",
                    "Track ID",
                    crate::ParamSchema::Track { kind: None },
                ),
                crate::ParamSpec::new("media_id", "Media ID", crate::ParamSchema::Media),
                crate::ParamSpec::new("timeline_start", "Start Time", crate::ParamSchema::Time)
                    .optional()
                    .with_default(serde_json::json!(0)),
                crate::ParamSpec::new("source_in", "Source In", crate::ParamSchema::Time)
                    .optional()
                    .with_default(serde_json::json!(0)),
            ])
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let track_id: u64 = ctx.get("track_id")?;
        let _media_id: u64 = ctx.get("media_id")?;
        let timeline_start: u64 = ctx.get_opt("timeline_start")?.unwrap_or(0);
        let _source_in: u64 = ctx.get_opt("source_in")?.unwrap_or(0);

        let clip_id = uuid::Uuid::new_v4();

        let undo_entry = crate::undo::UndoEntry::new(
            "timeline.addClip".to_string(),
            serde_json::json!({
                "clip_id": clip_id,
                "track_id": track_id.to_string(),
            }),
            serde_json::json!({
                "clip_id": clip_id,
            }),
        );
        ctx.app.undo.push(undo_entry);

        ctx.emit_event(crate::context::AppEvent::ClipAdded {
            clip_id: uuid::Uuid::new_v4(),
            track_id: uuid::Uuid::new_v4(),
        });

        Ok(serde_json::json!({
            "clip_id": clip_id.to_string(),
            "track_id": track_id,
            "timeline_start": timeline_start,
        }))
    }
}

pub struct RazorCommand;

#[async_trait]
impl crate::CommandHandler for RazorCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("timeline.razor", "Split Clip at Playhead")
            .with_category(CommandCategory::Timeline)
            .with_shortcut("Ctrl+K")
            .with_params(vec![
                crate::ParamSpec::new(
                    "track_id",
                    "Track ID",
                    crate::ParamSchema::Track { kind: None },
                ),
                crate::ParamSpec::new(
                    "clip_index",
                    "Clip Index",
                    crate::ParamSchema::Integer {
                        minimum: Some(0),
                        maximum: None,
                        multiple_of: None,
                    },
                ),
                crate::ParamSpec::new("at", "Split Time", crate::ParamSchema::Time).optional(),
            ])
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let track_id: u64 = ctx.get("track_id")?;
        let clip_index: usize = ctx.get("clip_index")?;
        let at: u64 = ctx
            .get_opt("at")?
            .unwrap_or_else(|| ctx.app.state.read().playhead);

        let new_clip_id = uuid::Uuid::new_v4();

        let undo_entry = crate::undo::UndoEntry::new(
            "timeline.razor".to_string(),
            serde_json::json!({
                "track_id": track_id,
                "clip_index": clip_index,
                "at": at,
            }),
            serde_json::json!({
                "new_clip_id": new_clip_id.to_string(),
            }),
        );
        ctx.app.undo.push(undo_entry);

        Ok(serde_json::json!({
            "split": true,
            "new_clip_id": new_clip_id.to_string(),
            "at": at,
        }))
    }
}

pub struct AddTransitionCommand;

#[async_trait]
impl crate::CommandHandler for AddTransitionCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("timeline.addTransition", "Add Transition")
            .with_category(CommandCategory::Transitions)
            .with_shortcut("Ctrl+D")
            .with_params(vec![
                crate::ParamSpec::new(
                    "track_id",
                    "Track ID",
                    crate::ParamSchema::Track {
                        kind: Some(crate::TrackKind::Video),
                    },
                ),
                crate::ParamSpec::new("cut_at", "Cut Time", crate::ParamSchema::Time),
                crate::ParamSpec::new("duration", "Duration", crate::ParamSchema::Time)
                    .with_default(serde_json::json!(1_000_000_000)),
                crate::ParamSpec::new(
                    "type",
                    "Transition Type",
                    crate::ParamSchema::Enum {
                        values: vec![
                            serde_json::json!("cross_dissolve"),
                            serde_json::json!("dip_to_black"),
                            serde_json::json!("dip_to_white"),
                            serde_json::json!("wipe_left"),
                            serde_json::json!("wipe_right"),
                        ],
                    },
                )
                .with_default(serde_json::json!("cross_dissolve")),
            ])
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let track_id: u64 = ctx.get("track_id")?;
        let cut_at: u64 = ctx.get("cut_at")?;
        let duration: u64 = ctx.get("duration")?;
        let trans_type: String = ctx.get("type")?;

        let transition_id = uuid::Uuid::new_v4();

        let undo_entry = crate::undo::UndoEntry::new(
            "timeline.addTransition".to_string(),
            serde_json::json!({
                "transition_id": transition_id,
                "track_id": track_id,
            }),
            serde_json::json!({
                "transition_id": transition_id.to_string(),
            }),
        );
        ctx.app.undo.push(undo_entry);

        Ok(serde_json::json!({
            "transition_id": transition_id.to_string(),
            "track_id": track_id,
            "cut_at": cut_at,
            "duration": duration,
            "type": trans_type,
        }))
    }
}

pub struct UndoCommand;

#[async_trait]
impl crate::CommandHandler for UndoCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("edit.undo", "Undo")
            .with_category(CommandCategory::Edit)
            .with_shortcut("Ctrl+Z")
            .non_mutating()
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let undone = ctx.app.undo.undo();
        if let Some(entry) = undone {
            ctx.emit_event(crate::context::AppEvent::UndoStackChanged {
                can_undo: ctx.app.undo.can_undo(),
                can_redo: ctx.app.undo.can_redo(),
            });
            Ok(serde_json::json!({ "undone": entry.description }))
        } else {
            Err(crate::HandlerError::Execution(
                "Nothing to undo".to_string(),
            ))
        }
    }

    async fn is_available(&self, ctx: &AppContext) -> bool {
        ctx.undo.can_undo()
    }
}

pub struct RedoCommand;

#[async_trait]
impl crate::CommandHandler for RedoCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("edit.redo", "Redo")
            .with_category(CommandCategory::Edit)
            .with_shortcut("Ctrl+Shift+Z")
            .non_mutating()
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let redone = ctx.app.undo.redo();
        if let Some(entry) = redone {
            ctx.emit_event(crate::context::AppEvent::UndoStackChanged {
                can_undo: ctx.app.undo.can_undo(),
                can_redo: ctx.app.undo.can_redo(),
            });
            Ok(serde_json::json!({ "redone": entry.description }))
        } else {
            Err(crate::HandlerError::Execution(
                "Nothing to redo".to_string(),
            ))
        }
    }

    async fn is_available(&self, ctx: &AppContext) -> bool {
        ctx.undo.can_redo()
    }
}

pub struct SetPlayheadCommand;

#[async_trait]
impl crate::CommandHandler for SetPlayheadCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("playhead.set", "Set Playhead Position")
            .with_category(CommandCategory::Timeline)
            .with_params(vec![
                crate::ParamSpec::new("position", "Position", crate::ParamSchema::Time),
                crate::ParamSpec::new("timecode", "Timecode", crate::ParamSchema::Timecode)
                    .optional(),
                crate::ParamSpec::new("frame", "Frame", crate::ParamSchema::Frame).optional(),
                crate::ParamSpec::new(
                    "seconds",
                    "Seconds",
                    crate::ParamSchema::Number {
                        minimum: Some(0.0),
                        maximum: None,
                        multiple_of: None,
                    },
                )
                .optional(),
            ])
            .non_mutating()
    }

    async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
        let position = if ctx.has("position") {
            ctx.get::<u64>("position")?
        } else if ctx.has("timecode") {
            let tc: String = ctx.get("timecode")?;
            parse_timecode(&tc)?
        } else if ctx.has("frame") {
            let frame: u64 = ctx.get("frame")?;
            frame * (254_016_000_000 / 30)
        } else if ctx.has("seconds") {
            let secs: f64 = ctx.get("seconds")?;
            (secs * 254_016_000_000.0) as u64
        } else {
            return Err(crate::HandlerError::Execution(
                "No position specified".to_string(),
            ));
        };

        let mut state = ctx.app.state.write();
        state.playhead = position;

        ctx.emit_event(crate::context::AppEvent::PlayheadMoved { position });

        Ok(serde_json::json!({ "position": position }))
    }
}

fn parse_timecode(tc: &str) -> Result<u64, crate::HandlerError> {
    let parts: Vec<&str> = tc.split(|c| c == ':' || c == ';').collect();
    if parts.len() != 4 {
        return Err(crate::HandlerError::Execution(
            "Invalid timecode format".to_string(),
        ));
    }
    let hours: u64 = parts[0]
        .parse()
        .map_err(|_| crate::HandlerError::Execution("Invalid hours".to_string()))?;
    let minutes: u64 = parts[1]
        .parse()
        .map_err(|_| crate::HandlerError::Execution("Invalid minutes".to_string()))?;
    let seconds: u64 = parts[2]
        .parse()
        .map_err(|_| crate::HandlerError::Execution("Invalid seconds".to_string()))?;
    let frames: u64 = parts[3]
        .parse()
        .map_err(|_| crate::HandlerError::Execution("Invalid frames".to_string()))?;

    let frame_rate = 30;
    let total_frames =
        hours * 3600 * frame_rate + minutes * 60 * frame_rate + seconds * frame_rate + frames;
    Ok(total_frames * (254_016_000_000 / frame_rate))
}

pub fn register_builtin_commands(
    registry: &crate::CommandRegistry,
) -> Result<(), crate::RegistryError> {
    registry.register(Box::new(EchoCommand))?;
    registry.register(Box::new(ProjectStatusCommand))?;
    registry.register(Box::new(NewProjectCommand))?;
    registry.register(Box::new(AddClipCommand))?;
    registry.register(Box::new(RazorCommand))?;
    registry.register(Box::new(AddTransitionCommand))?;
    registry.register(Box::new(UndoCommand))?;
    registry.register(Box::new(RedoCommand))?;
    registry.register(Box::new(SetPlayheadCommand))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::AppContext;

    use crate::CommandId;
    use parking_lot::RwLock;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_builtin_commands() {
        let registry = Arc::new(crate::CommandRegistry::new());
        register_builtin_commands(&registry).unwrap();

        let ctx = AppContext {
            state: Arc::new(RwLock::new(crate::context::AppState::default())),
            registry: registry.clone(),
            undo: Arc::new(crate::UndoManager::new(100)),
            shortcuts: Arc::new(crate::ShortcutManager::new()),
            events: Arc::new(crate::context::EventBus::new()),
        };

        let result = registry
            .execute(
                &CommandId::new("test.echo"),
                serde_json::json!({ "message": "hello" }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(result["echo"], "hello");

        let result = registry
            .execute(
                &CommandId::new("project.status"),
                serde_json::json!({}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(result["has_project"], false);
    }
}
