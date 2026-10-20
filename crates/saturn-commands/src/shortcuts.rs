//! Keyboard shortcut management system

use crate::CommandId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Platform-specific key representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Mac,
    Windows,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        return Platform::Mac;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        return Platform::Linux;
    }

    pub fn modifier_name(&self, modifier: Modifier) -> &'static str {
        match (self, modifier) {
            (Platform::Mac, Modifier::Ctrl) => "Cmd",
            (Platform::Mac, Modifier::Alt) => "Opt",
            (Platform::Mac, Modifier::Meta) => "Cmd",
            (Platform::Mac, Modifier::Shift) => "Shift",
            (_, Modifier::Ctrl) => "Ctrl",
            (_, Modifier::Alt) => "Alt",
            (_, Modifier::Meta) => "Meta",
            (_, Modifier::Shift) => "Shift",
        }
    }
}

/// Keyboard modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

/// Key binding representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    pub modifiers: Vec<Modifier>,
}

impl KeyBinding {
    pub fn new(key: impl Into<String>, modifiers: Vec<Modifier>) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').collect();
        if parts.is_empty() {
            return None;
        }

        let mut modifiers = Vec::new();
        let mut key = String::new();

        for part in parts {
            let trimmed = part.trim();
            let lower = trimmed.to_lowercase();
            match lower.as_str() {
                "ctrl" | "control" | "cmd" | "command" | "meta" => modifiers.push(Modifier::Ctrl),
                "alt" | "option" => modifiers.push(Modifier::Alt),
                "shift" => modifiers.push(Modifier::Shift),
                "meta" | "super" | "win" => modifiers.push(Modifier::Meta),
                _ => key = trimmed.to_string(),
            }
        }

        if key.is_empty() {
            return None;
        }

        Some(Self { key, modifiers })
    }

    pub fn format(&self, platform: Platform) -> String {
        let mut parts = Vec::new();
        for &modif in &self.modifiers {
            parts.push(platform.modifier_name(modif).to_string());
        }
        parts.push(self.key.clone());
        parts.join("+")
    }

    pub fn matches(&self, other: &KeyBinding) -> bool {
        self.key.eq_ignore_ascii_case(&other.key) && self.modifiers == other.modifiers
    }
}

/// Shortcut specification for a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutSpec {
    pub command_id: CommandId,
    pub binding: KeyBinding,
    pub panel: Option<String>,
    pub platform: Option<Platform>,
    pub description: Option<String>,
}

impl ShortcutSpec {
    pub fn new(command_id: CommandId, binding: KeyBinding) -> Self {
        Self {
            command_id,
            binding,
            panel: None,
            platform: None,
            description: None,
        }
    }

    pub fn with_panel(mut self, panel: impl Into<String>) -> Self {
        self.panel = Some(panel.into());
        self
    }

    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }
}

/// Shortcut manager
pub struct ShortcutManager {
    shortcuts: RwLock<HashMap<CommandId, Vec<ShortcutSpec>>>,
    bindings: RwLock<HashMap<(Platform, Option<String>, KeyBinding), CommandId>>,
    platform: Platform,
}

impl ShortcutManager {
    pub fn new() -> Self {
        Self {
            shortcuts: RwLock::new(HashMap::new()),
            bindings: RwLock::new(HashMap::new()),
            platform: Platform::current(),
        }
    }

    pub fn with_platform(platform: Platform) -> Self {
        Self {
            shortcuts: RwLock::new(HashMap::new()),
            bindings: RwLock::new(HashMap::new()),
            platform,
        }
    }

    pub fn register(&self, spec: ShortcutSpec) -> Result<(), ShortcutError> {
        let binding = spec.binding.clone();
        let command_id = spec.command_id.clone();
        let panel = spec.panel.clone();
        let platform = spec.platform.unwrap_or(self.platform);

        let key = (platform, panel.clone(), binding.clone());
        if let Some(existing) = self.bindings.read().get(&key) {
            if existing != &command_id {
                return Err(ShortcutError::Conflict {
                    binding: binding.format(platform),
                    existing: existing.clone(),
                    new: command_id,
                });
            }
        }

        self.shortcuts
            .write()
            .entry(command_id.clone())
            .or_default()
            .push(spec.clone());
        self.bindings.write().insert(key, command_id);

        Ok(())
    }

    pub fn unregister(&self, command_id: &CommandId) {
        if let Some(specs) = self.shortcuts.write().remove(command_id) {
            for spec in specs {
                let platform = spec.platform.unwrap_or(self.platform);
                let key = (platform, spec.panel.clone(), spec.binding);
                self.bindings.write().remove(&key);
            }
        }
    }

    pub fn set(
        &self,
        command_id: &CommandId,
        binding: KeyBinding,
        panel: Option<String>,
    ) -> Result<(), ShortcutError> {
        self.unregister(command_id);
        self.register(
            ShortcutSpec::new(command_id.clone(), binding).with_panel(panel.unwrap_or_default()),
        )
    }

    pub fn get(&self, command_id: &CommandId) -> Vec<ShortcutSpec> {
        self.shortcuts
            .read()
            .get(command_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn all(&self) -> Vec<ShortcutSpec> {
        self.shortcuts.read().values().flatten().cloned().collect()
    }

    pub fn find(&self, binding: &KeyBinding, panel: Option<&str>) -> Option<CommandId> {
        let platform = self.platform;
        let key = (platform, panel.map(|s| s.to_string()), binding.clone());
        self.bindings.read().get(&key).cloned()
    }

    pub fn find_by_string(&self, s: &str, panel: Option<&str>) -> Option<CommandId> {
        if let Some(binding) = KeyBinding::parse(s) {
            self.find(&binding, panel)
        } else {
            None
        }
    }

    pub fn for_panel(&self, panel: &str) -> Vec<ShortcutSpec> {
        self.shortcuts
            .read()
            .values()
            .flatten()
            .filter(|s| s.panel.as_deref() == Some(panel) || s.panel.is_none())
            .cloned()
            .collect()
    }

    pub fn format(&self, command_id: &CommandId) -> Option<String> {
        let specs = self.get(command_id);
        specs.first().map(|s| s.binding.format(self.platform))
    }

    pub fn conflicts(&self) -> Vec<ShortcutConflict> {
        let mut conflicts = Vec::new();
        let bindings = self.bindings.read();

        let mut groups: HashMap<(Platform, Option<String>, KeyBinding), Vec<&CommandId>> =
            HashMap::new();
        for (key, cmd_id) in bindings.iter() {
            groups.entry(key.clone()).or_default().push(cmd_id);
        }

        for (key, commands) in groups {
            if commands.len() > 1 {
                conflicts.push(ShortcutConflict {
                    binding: key.2.format(key.0),
                    panel: key.1,
                    commands: commands.into_iter().cloned().collect(),
                });
            }
        }

        conflicts
    }

    pub fn resolve_conflicts(&self) {
        let conflicts = self.conflicts();
        for conflict in conflicts {
            let keep = conflict.commands[0].clone();
            for cmd in &conflict.commands[1..] {
                self.unregister(cmd);
            }
            if let Some(specs) = self.shortcuts.read().get(&keep) {
                for spec in specs {
                    let _ = self.register(spec.clone());
                }
            }
        }
    }

    pub fn export(&self) -> Value {
        let shortcuts = self.all();
        serde_json::to_value(shortcuts).unwrap_or_default()
    }

    pub fn import(&self, value: Value, activate: bool) -> Result<(), ShortcutError> {
        let shortcuts: Vec<ShortcutSpec> = serde_json::from_value(value)?;

        if activate {
            self.shortcuts.write().clear();
            self.bindings.write().clear();
        }

        for spec in shortcuts {
            self.register(spec)?;
        }

        Ok(())
    }

    pub fn load_preset(&self, name: &str) -> Result<(), ShortcutError> {
        match name {
            "default" => self.load_default_preset(),
            "premiere" => self.load_premiere_preset(),
            "davinci" => self.load_davinci_preset(),
            "final_cut" => self.load_final_cut_preset(),
            _ => Err(ShortcutError::NotFound(name.to_string())),
        }
    }

    fn load_default_preset(&self) -> Result<(), ShortcutError> {
        self.register(ShortcutSpec::new(
            CommandId::new("edit.undo"),
            KeyBinding::new("Z", vec![Modifier::Ctrl]),
        ))?;
        self.register(ShortcutSpec::new(
            CommandId::new("edit.redo"),
            KeyBinding::new("Z", vec![Modifier::Ctrl, Modifier::Shift]),
        ))?;
        self.register(ShortcutSpec::new(
            CommandId::new("timeline.razor"),
            KeyBinding::new("K", vec![Modifier::Ctrl]),
        ))?;
        self.register(ShortcutSpec::new(
            CommandId::new("timeline.addTransition"),
            KeyBinding::new("D", vec![Modifier::Ctrl]),
        ))?;
        Ok(())
    }

    fn load_premiere_preset(&self) -> Result<(), ShortcutError> {
        self.load_default_preset()?;
        self.register(ShortcutSpec::new(
            CommandId::new("timeline.razor"),
            KeyBinding::new("C", vec![]),
        ))?;
        Ok(())
    }

    fn load_davinci_preset(&self) -> Result<(), ShortcutError> {
        self.load_default_preset()?;
        self.register(ShortcutSpec::new(
            CommandId::new("timeline.razor"),
            KeyBinding::new("B", vec![]),
        ))?;
        Ok(())
    }

    fn load_final_cut_preset(&self) -> Result<(), ShortcutError> {
        self.load_default_preset()?;
        self.register(ShortcutSpec::new(
            CommandId::new("timeline.razor"),
            KeyBinding::new("B", vec![]),
        ))?;
        Ok(())
    }

    pub fn presets(&self) -> Vec<String> {
        vec![
            "default".to_string(),
            "premiere".to_string(),
            "davinci".to_string(),
            "final_cut".to_string(),
        ]
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Shortcut errors
#[derive(Debug, thiserror::Error)]
pub enum ShortcutError {
    #[error(
        "Shortcut conflict: {binding} already assigned to {existing:?}, trying to assign to {new:?}"
    )]
    Conflict {
        binding: String,
        existing: CommandId,
        new: CommandId,
    },

    #[error("Shortcut not found: {0}")]
    NotFound(String),

    #[error("Invalid shortcut format: {0}")]
    InvalidFormat(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConflict {
    pub binding: String,
    pub panel: Option<String>,
    pub commands: Vec<CommandId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandId;

    #[test]
    fn test_keybinding_parse() {
        let binding = KeyBinding::parse("Ctrl+K").unwrap();
        assert_eq!(binding.key, "K");
        assert_eq!(binding.modifiers, vec![Modifier::Ctrl]);

        let binding = KeyBinding::parse("Ctrl+Shift+Z").unwrap();
        assert_eq!(binding.key, "Z");
        assert_eq!(binding.modifiers, vec![Modifier::Ctrl, Modifier::Shift]);

        let binding = KeyBinding::parse("ArrowLeft").unwrap();
        assert_eq!(binding.key, "ArrowLeft");
        assert!(binding.modifiers.is_empty());
    }

    #[test]
    fn test_keybinding_format() {
        let binding = KeyBinding::new("K", vec![Modifier::Ctrl]);
        assert_eq!(binding.format(Platform::Mac), "Cmd+K");
        assert_eq!(binding.format(Platform::Windows), "Ctrl+K");
        assert_eq!(binding.format(Platform::Linux), "Ctrl+K");
    }

    #[test]
    fn test_shortcut_manager() {
        let manager = ShortcutManager::new();

        manager
            .register(ShortcutSpec::new(
                CommandId::new("test.cmd"),
                KeyBinding::new("K", vec![Modifier::Ctrl]),
            ))
            .unwrap();

        assert_eq!(
            manager.format(&CommandId::new("test.cmd")),
            Some("Ctrl+K".to_string())
        );
        assert_eq!(
            manager.find_by_string("Ctrl+K", None),
            Some(CommandId::new("test.cmd"))
        );
    }

    #[test]
    fn test_shortcut_conflict() {
        let manager = ShortcutManager::new();

        manager
            .register(ShortcutSpec::new(
                CommandId::new("cmd1"),
                KeyBinding::new("K", vec![Modifier::Ctrl]),
            ))
            .unwrap();

        let result = manager.register(ShortcutSpec::new(
            CommandId::new("cmd2"),
            KeyBinding::new("K", vec![Modifier::Ctrl]),
        ));

        assert!(result.is_err());
        assert!(matches!(result, Err(ShortcutError::Conflict { .. })));
    }

    #[test]
    fn test_presets() {
        let manager = ShortcutManager::new();
        manager.load_preset("default").unwrap();
        assert!(manager.find_by_string("Ctrl+K", None).is_some());
        assert!(manager.find_by_string("Ctrl+Z", None).is_some());
    }

    #[test]
    fn test_export_import() {
        let manager = ShortcutManager::new();
        manager.load_preset("default").unwrap();

        let exported = manager.export();
        assert!(exported.is_array());

        let manager2 = ShortcutManager::new();
        manager2.import(exported, true).unwrap();
        assert_eq!(
            manager2.format(&CommandId::new("edit.undo")),
            Some("Ctrl+Z".to_string())
        );
    }
}
