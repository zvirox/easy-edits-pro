//! Command execution context and handler system

use crate::{
    CommandId, CommandSpec, RegistryError, ValidationError,
    CommandPriority, ExecutionMode, CommandCategory,
    params::ParamSchema,
    HandlerError,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Application context available to command handlers
pub struct AppContext {
    pub state: Arc<RwLock<AppState>>,
    pub registry: Arc<crate::CommandRegistry>,
    pub undo: Arc<crate::UndoManager>,
    pub shortcuts: Arc<crate::ShortcutManager>,
    pub events: Arc<EventBus>,
}

/// Application state
#[derive(Default)]
pub struct AppState {
    pub project: Option<ProjectHandle>,
    pub sequence: Option<SequenceHandle>,
    pub playhead: u64,
    pub selection: SelectionState,
    pub ui: UiState,
}

/// Project handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHandle(pub Uuid);

/// Sequence handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceHandle(pub Uuid);

/// Current selection state
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SelectionState {
    pub clips: Vec<ClipHandle>,
    pub tracks: Vec<TrackHandle>,
    pub transitions: Vec<TransitionHandle>,
    pub playhead: Option<u64>,
}

/// Clip handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipHandle(pub Uuid);

/// Track handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackHandle(pub Uuid);

/// Transition handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionHandle(pub Uuid);

/// UI state
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub active_workspace: String,
    pub timeline_zoom: f32,
    pub playhead_visible: bool,
}

/// Execution context passed to handlers
pub struct ExecutionContext<'a> {
    pub app: &'a AppContext,
    pub command: &'a CommandSpec,
    pub params: Value,
    pub execution_id: Uuid,
    pub start_time: std::time::Instant,
}

impl<'a> ExecutionContext<'a> {
    pub fn get<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<T, ValidationError> {
        let value = self.params.get(name).ok_or_else(|| ValidationError::MissingRequired(name.to_string()))?;
        serde_json::from_value(value.clone()).map_err(|e| ValidationError::Custom {
            param: name.to_string(),
            message: format!("failed to deserialize: {e}"),
        })
    }

    pub fn get_opt<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<Option<T>, ValidationError> {
        if let Some(value) = self.params.get(name) {
            if value.is_null() {
                return Ok(None);
            }
            Ok(Some(serde_json::from_value(value.clone()).map_err(|e| ValidationError::Custom {
                param: name.to_string(),
                message: format!("failed to deserialize: {e}"),
            })?))
        } else {
            Ok(None)
        }
    }

    pub fn get_raw(&self, name: &str) -> Option<&Value> {
        self.params.get(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.params.get(name).map(|v| !v.is_null()).unwrap_or(false)
    }

    pub fn emit_event(&self, event: AppEvent) {
        self.app.events.emit(event);
    }
}

/// Application events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    CommandStarted { command_id: CommandId, execution_id: Uuid },
    CommandCompleted { command_id: CommandId, execution_id: Uuid, success: bool },
    ProjectChanged { project_id: Uuid },
    SequenceChanged { sequence_id: Uuid },
    SelectionChanged { selection: SelectionState },
    PlayheadMoved { position: u64 },
    TrackAdded { track_id: Uuid },
    TrackRemoved { track_id: Uuid },
    ClipAdded { clip_id: Uuid, track_id: Uuid },
    ClipRemoved { clip_id: Uuid },
    ClipMoved { clip_id: Uuid, old_position: u64, new_position: u64 },
    UndoStackChanged { can_undo: bool, can_redo: bool },
}

/// Event bus
pub struct EventBus {
    subscribers: RwLock<Vec<Box<dyn Fn(AppEvent) + Send + Sync>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { subscribers: RwLock::new(Vec::new()) }
    }

    pub fn subscribe(&self, f: impl Fn(AppEvent) + Send + Sync + 'static) {
        self.subscribers.write().push(Box::new(f));
    }

    pub fn emit(&self, event: AppEvent) {
        for sub in self.subscribers.read().iter() {
            sub(event.clone());
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for command context
pub type CommandContext = AppContext;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_context_creation() {
        let ctx = AppContext {
            state: Arc::new(RwLock::new(AppState::default())),
            registry: Arc::new(crate::CommandRegistry::new()),
            undo: Arc::new(crate::UndoManager::new(100)),
            shortcuts: Arc::new(crate::ShortcutManager::new()),
            events: Arc::new(EventBus::new()),
        };

        assert!(ctx.state.read().project.is_none());
    }
}