//! Command registry - central command management

use crate::{
    CommandCategory, CommandId, CommandSpec, HandlerError, RegistryError, ValidationError,
    context::{AppContext, ExecutionContext},
    params::ParamSchema,
};
use indexmap::IndexMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct CommandRegistry {
    commands: RwLock<IndexMap<CommandId, Arc<dyn crate::CommandHandler>>>,
    specs: RwLock<IndexMap<CommandId, CommandSpec>>,
    by_category: RwLock<HashMap<CommandCategory, Vec<CommandId>>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: RwLock::new(IndexMap::new()),
            specs: RwLock::new(IndexMap::new()),
            by_category: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, handler: Box<dyn crate::CommandHandler>) -> Result<(), RegistryError> {
        let spec = handler.spec();
        let id = spec.id.clone();

        if self.commands.read().contains_key(&id) {
            return Err(RegistryError::AlreadyRegistered(id));
        }

        self.validate_spec(&spec)?;

        let handler_arc = Arc::from(handler);
        self.commands.write().insert(id.clone(), handler_arc);
        self.specs.write().insert(id.clone(), spec.clone());
        self.by_category
            .write()
            .entry(spec.category)
            .or_default()
            .push(id);

        Ok(())
    }

    pub fn unregister(&self, id: &CommandId) -> Result<(), RegistryError> {
        let spec = self
            .specs
            .write()
            .shift_remove(id)
            .ok_or(RegistryError::NotFound(id.clone()))?;

        self.commands.write().shift_remove(id);
        self.by_category
            .write()
            .entry(spec.category)
            .and_modify(|v| v.retain(|x| x != id));

        Ok(())
    }

    pub fn get(&self, id: &CommandId) -> Option<Arc<dyn crate::CommandHandler>> {
        self.commands.read().get(id).cloned()
    }

    pub fn get_spec(&self, id: &CommandId) -> Option<CommandSpec> {
        self.specs.read().get(id).cloned()
    }

    pub fn all_specs(&self) -> Vec<CommandSpec> {
        self.specs.read().values().cloned().collect()
    }

    pub fn by_category(&self, category: CommandCategory) -> Vec<CommandId> {
        self.by_category
            .read()
            .get(&category)
            .cloned()
            .unwrap_or_default()
    }

    pub fn menu_structure(&self) -> Vec<MenuCategory> {
        let specs = self.specs.read();
        let mut categories: HashMap<CommandCategory, Vec<MenuItem>> = HashMap::new();

        for spec in specs.values() {
            let item = MenuItem {
                id: spec.id.clone(),
                label: spec.label.clone(),
                shortcut: spec.shortcut.clone(),
                icon: spec.icon.clone(),
                enabled: true,
            };
            categories.entry(spec.category).or_default().push(item);
        }

        let mut result: Vec<MenuCategory> = categories
            .into_iter()
            .map(|(cat, items)| MenuCategory {
                category: cat,
                label: cat.to_string(),
                items,
            })
            .collect();

        result.sort_by_key(|c| menu_category_order(c.category));
        result
    }

    pub async fn execute(
        &self,
        id: &CommandId,
        params: Value,
        ctx: &AppContext,
    ) -> Result<Value, crate::HandlerError> {
        let handler = self.get(id).ok_or(crate::HandlerError::Execution(format!(
            "Command not found: {}",
            id
        )))?;

        let spec = self
            .get_spec(id)
            .ok_or(crate::HandlerError::Execution(format!(
                "Spec not found for command: {}",
                id
            )))?;
        validate_params(&spec, &params)?;

        if !handler.is_available(ctx).await {
            return Err(crate::HandlerError::NotAvailable(format!(
                "Command {} not available in current context",
                id
            )));
        }

        let execution_id = uuid::Uuid::new_v4();
        let exec_ctx = ExecutionContext {
            app: ctx,
            command: &spec,
            params,
            execution_id,
            start_time: std::time::Instant::now(),
        };

        ctx.events.emit(crate::context::AppEvent::CommandStarted {
            command_id: id.clone(),
            execution_id,
        });

        let result = handler.execute(exec_ctx).await;

        ctx.events.emit(crate::context::AppEvent::CommandCompleted {
            command_id: id.clone(),
            execution_id,
            success: result.is_ok(),
        });

        result
    }

    pub async fn is_available(&self, id: &CommandId, ctx: &AppContext) -> bool {
        if let Some(handler) = self.get(id) {
            handler.is_available(ctx).await
        } else {
            false
        }
    }

    pub async fn get_state(&self, id: &CommandId, ctx: &AppContext) -> Option<Value> {
        if let Some(handler) = self.get(id) {
            handler.get_state(ctx).await
        } else {
            None
        }
    }

    fn validate_spec(&self, spec: &CommandSpec) -> Result<(), RegistryError> {
        if spec.id.as_str().is_empty() {
            return Err(RegistryError::InvalidSpec("empty command ID".to_string()));
        }
        if spec.label.is_empty() {
            return Err(RegistryError::InvalidSpec("empty label".to_string()));
        }
        Ok(())
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

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

fn menu_category_order(cat: CommandCategory) -> u32 {
    match cat {
        CommandCategory::File => 0,
        CommandCategory::Edit => 1,
        CommandCategory::Sequence => 2,
        CommandCategory::Clip => 3,
        CommandCategory::Timeline => 4,
        CommandCategory::Graphics => 5,
        CommandCategory::Color => 6,
        CommandCategory::Audio => 7,
        CommandCategory::Effects => 8,
        CommandCategory::Transitions => 9,
        CommandCategory::MultiCam => 10,
        CommandCategory::Transcription => 11,
        CommandCategory::Export => 12,
        CommandCategory::Workspace => 13,
        CommandCategory::View => 14,
        CommandCategory::Window => 15,
        CommandCategory::Help => 16,
        CommandCategory::Other => 100,
    }
}

fn validate_params(spec: &CommandSpec, params: &Value) -> Result<(), crate::HandlerError> {
    let obj = params.as_object().ok_or_else(|| {
        crate::HandlerError::Validation(ValidationError::Custom {
            param: "params".to_string(),
            message: "parameters must be an object".to_string(),
        })
    })?;

    for param in &spec.params {
        if param.required {
            if !obj.contains_key(&param.name) || obj[&param.name].is_null() {
                if param.default.is_none() {
                    return Err(crate::HandlerError::Validation(
                        ValidationError::MissingRequired(param.name.clone()),
                    ));
                }
            }
        }

        if let Some(value) = obj.get(&param.name) {
            if !value.is_null() {
                param.schema.validate(&param.name, value)?;
            }
        }
    }

    let known: std::collections::HashSet<_> = spec.params.iter().map(|p| p.name.as_str()).collect();
    for key in obj.keys() {
        if !known.contains(key.as_str()) {
            log::warn!("Unknown parameter '{}' for command", key);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandHandler;
    use crate::params::ParamSchema;
    use async_trait::async_trait;
    use serde_json::json;

    struct TestCommand;
    #[async_trait]
    impl CommandHandler for TestCommand {
        fn spec(&self) -> CommandSpec {
            CommandSpec::new("test.cmd", "Test Command")
                .with_category(CommandCategory::Edit)
                .with_params(vec![crate::ParamSpec::new(
                    "value",
                    "Value",
                    crate::ParamSchema::Integer {
                        minimum: Some(0),
                        maximum: Some(100),
                        multiple_of: None,
                    },
                )])
        }

        async fn execute(&self, ctx: ExecutionContext<'_>) -> Result<Value, crate::HandlerError> {
            let value: i64 = ctx.get("value")?;
            Ok(json!({ "doubled": value * 2 }))
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_execute() {
        let registry = Arc::new(CommandRegistry::new());
        registry.register(Box::new(TestCommand)).unwrap();

        let ctx = crate::context::AppContext {
            state: Arc::new(RwLock::new(crate::context::AppState::default())),
            registry: registry.clone(),
            undo: Arc::new(crate::UndoManager::new(100)),
            shortcuts: Arc::new(crate::ShortcutManager::new()),
            events: Arc::new(crate::context::EventBus::new()),
        };

        let result = registry
            .execute(&CommandId::new("test.cmd"), json!({ "value": 21 }), &ctx)
            .await
            .unwrap();
        assert_eq!(result["doubled"], 42);
    }
}
