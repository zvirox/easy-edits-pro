//! Durable application settings: preferences, shortcut overrides, and custom
//! layout presets. The engine owns the data so the CLI and MCP bridge see the
//! same values as the desktop UI; the Tauri shell points it at one JSON file in
//! the app data directory.

use super::*;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub(super) struct SettingsStore {
    path: Option<PathBuf>,
    data: serde_json::Map<String, Value>,
    /// Shortcut-map snapshots for shortcuts.undo / shortcuts.redo. Settings
    /// live outside the project document, so they carry their own stack.
    shortcut_undo: Vec<serde_json::Map<String, Value>>,
    shortcut_redo: Vec<serde_json::Map<String, Value>>,
}

impl SettingsStore {
    fn load(path: PathBuf) -> Self {
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        Self {
            path: Some(path),
            data,
            shortcut_undo: Vec::new(),
            shortcut_redo: Vec::new(),
        }
    }

    fn object(&self, key: &str) -> Value {
        self.data.get(key).cloned().unwrap_or_else(|| json!({}))
    }

    fn shortcuts_map(&self) -> serde_json::Map<String, Value> {
        self.data
            .get("shortcuts")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    }

    fn presets_map(&self) -> serde_json::Map<String, Value> {
        self.data
            .get("shortcut_presets")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    }

    /// Snapshot the shortcut map before a mutation so shortcuts.undo can
    /// restore it. Settings changes do not enter the project history.
    fn push_shortcut_snapshot(&mut self) {
        self.shortcut_undo.push(self.shortcuts_map());
        if self.shortcut_undo.len() > 100 {
            self.shortcut_undo.remove(0);
        }
        self.shortcut_redo.clear();
    }

    /// Write a shortcut map, snapshotting first and persisting.
    fn write_shortcuts_map(
        &mut self,
        shortcuts: serde_json::Map<String, Value>,
        snapshot: bool,
    ) -> Result<(), DispatchError> {
        if snapshot {
            self.push_shortcut_snapshot();
        }
        self.data.insert("shortcuts".into(), Value::Object(shortcuts));
        self.persist()
    }

    fn persist(&self) -> Result<(), DispatchError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory).map_err(|error| {
                DispatchError::Operation(format!("Could not create settings folder: {error}"))
            })?;
        }
        let bytes = serde_json::to_vec_pretty(&Value::Object(self.data.clone()))
            .map_err(|error| DispatchError::Operation(error.to_string()))?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes).map_err(|error| {
            DispatchError::Operation(format!("Could not write settings: {error}"))
        })?;
        std::fs::rename(&temporary, path)
            .map_err(|error| DispatchError::Operation(format!("Could not save settings: {error}")))
    }
}

impl EditorEngine {
    /// Point the engine at the desktop settings file and hydrate it.
    pub fn load_settings(&mut self, path: impl AsRef<Path>) {
        self.settings = SettingsStore::load(path.as_ref().to_path_buf());
    }

    pub(super) fn dispatch_settings(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "prefs.get" => Some((|| {
                let params: PrefGetParams = if params.is_null() {
                    PrefGetParams { key: None }
                } else {
                    parse_params(params)?
                };
                let prefs = self.settings.object("prefs");
                Ok(match params.key {
                    Some(key) => json!({"key": key, "value": prefs.get(&key).cloned()}),
                    None => json!({"prefs": prefs}),
                })
            })()),
            "prefs.set" => Some((|| {
                let params: PrefSetParams = parse_params(params)?;
                let mut prefs = self.settings.object("prefs");
                let object = prefs.as_object_mut().ok_or_else(|| {
                    DispatchError::Operation("Preferences store is corrupt".into())
                })?;
                for (key, value) in params.values {
                    object.insert(key, value);
                }
                self.settings.data.insert("prefs".into(), prefs);
                self.settings.persist()?;
                Ok(json!({"prefs": self.settings.object("prefs")}))
            })()),
            "prefs.reset" => Some((|| {
                self.settings.data.remove("prefs");
                self.settings.persist()?;
                Ok(json!({"reset": true}))
            })()),
            // Query: the keys the product recognises, their shape, and the
            // default the UI applies when a key is absent.
            "prefs.schema" => Some(Ok(json!({
                "schema": [
                    {"key": "theme", "type": "string", "values": ["dark", "light"], "default": "dark"},
                    {"key": "glass", "type": "string", "values": ["full", "lite", "off"], "default": "full"},
                    {"key": "reducedMotion", "type": "bool|null", "values": [true, false, null], "default": null},
                    {"key": "snap", "type": "bool", "values": [true, false], "default": true},
                    {"key": "trackHeight", "type": "string", "values": ["compact", "normal", "expanded"], "default": "normal"},
                ],
            }))),
            "shortcuts.list" => Some(Ok(json!({"shortcuts": self.settings.object("shortcuts")}))),
            "shortcuts.set" => Some(self.write_shortcuts(params, false)),
            "shortcuts.import" => Some(self.write_shortcuts(params, true)),
            "shortcuts.reset" => Some((|| {
                self.settings.push_shortcut_snapshot();
                self.settings.data.remove("shortcuts");
                self.settings.persist()?;
                Ok(json!({"reset": true}))
            })()),
            "shortcuts.get" => Some((|| {
                let params: ShortcutGetParams = parse_params(params)?;
                let shortcut = self
                    .settings
                    .shortcuts_map()
                    .get(&params.command_id)
                    .cloned();
                Ok(json!({"command_id": params.command_id, "shortcut": shortcut}))
            })()),
            "shortcuts.for_key" => Some((|| {
                let params: ShortcutForKeyParams = parse_params(params)?;
                let mut commands: Vec<String> = self
                    .settings
                    .shortcuts_map()
                    .into_iter()
                    .filter(|(_, value)| value.as_str() == Some(params.key.as_str()))
                    .map(|(id, _)| id)
                    .collect();
                commands.sort();
                Ok(json!({"key": params.key, "commands": commands}))
            })()),
            "shortcuts.conflicts" => Some((|| {
                let mut by_key: std::collections::BTreeMap<String, Vec<String>> = Default::default();
                for (id, value) in self.settings.shortcuts_map() {
                    if let Some(key) = value.as_str() {
                        if !key.is_empty() {
                            by_key.entry(key.to_string()).or_default().push(id);
                        }
                    }
                }
                let conflicts: Vec<Value> = by_key
                    .into_iter()
                    .filter(|(_, commands)| commands.len() > 1)
                    .map(|(key, mut commands)| {
                        commands.sort();
                        json!({"key": key, "commands": commands})
                    })
                    .collect();
                Ok(json!({"conflicts": conflicts}))
            })()),
            // Read-only health report over the override table: how many
            // entries exist, how many are removals, and how many key clashes.
            "shortcuts.audit" => Some((|| {
                let map = self.settings.shortcuts_map();
                let mut by_key: std::collections::BTreeMap<&str, usize> = Default::default();
                let mut empty = 0usize;
                for value in map.values() {
                    match value.as_str() {
                        Some(key) if !key.is_empty() => {
                            *by_key.entry(key).or_default() += 1;
                        }
                        _ => empty += 1,
                    }
                }
                let conflicts = by_key.values().filter(|count| **count > 1).count();
                Ok(json!({
                    "total": map.len(),
                    "empty": empty,
                    "conflicts": conflicts,
                }))
            })()),
            "shortcuts.clear" => Some((|| {
                let params: ShortcutClearParams = parse_params(params)?;
                let mut map = self.settings.shortcuts_map();
                if map.remove(&params.command_id).is_none() {
                    return Ok(json!({"cleared": false, "command_id": params.command_id}));
                }
                self.settings.write_shortcuts_map(map, true)?;
                Ok(json!({"cleared": true, "command_id": params.command_id}))
            })()),
            // Resolve a conflict: the keeper command keeps the key, every
            // other binding of that key is removed.
            "shortcuts.resolve" => Some((|| {
                let params: ShortcutResolveParams = parse_params(params)?;
                let map = self.settings.shortcuts_map();
                if map.get(&params.keep).and_then(|value| value.as_str())
                    != Some(params.key.as_str())
                {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Command {} is not bound to {}",
                        params.keep, params.key
                    )));
                }
                let mut map = map;
                let losers: Vec<String> = map
                    .iter()
                    .filter(|(id, value)| {
                        id.as_str() != params.keep.as_str()
                            && value.as_str() == Some(params.key.as_str())
                    })
                    .map(|(id, _)| id.clone())
                    .collect();
                if losers.is_empty() {
                    return Ok(json!({"resolved": 0, "key": params.key}));
                }
                for id in &losers {
                    map.remove(id);
                }
                self.settings.write_shortcuts_map(map, true)?;
                Ok(json!({"resolved": losers.len(), "key": params.key}))
            })()),
            "shortcuts.export" => Some((|| {
                let params: ShortcutExportParams = parse_params(params)?;
                let map = self.settings.shortcuts_map();
                let payload =
                    serde_json::to_vec_pretty(&json!({"shortcuts": map})).map_err(|error| {
                        DispatchError::Operation(format!("Could not serialise shortcuts: {error}"))
                    })?;
                std::fs::write(&params.path, payload).map_err(|error| {
                    DispatchError::Operation(format!("Could not write shortcuts file: {error}"))
                })?;
                Ok(json!({
                    "exported": map.len(),
                    "path": params.path,
                }))
            })()),
            // Presets are named shortcut maps in the settings store; the
            // built-in "default" is the empty override set (factory keys).
            "shortcuts.presets" => Some((|| {
                let mut presets = self.settings.presets_map();
                presets.insert("default".into(), json!({}));
                Ok(json!({"presets": presets}))
            })()),
            "shortcuts.save_preset" => Some((|| {
                let params: ShortcutPresetParams = parse_params(params)?;
                let name = params.name.trim();
                if name.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Preset name cannot be empty".into(),
                    ));
                }
                let mut presets = self.settings.presets_map();
                presets.insert(name.to_string(), Value::Object(self.settings.shortcuts_map()));
                self.settings.data.insert("shortcut_presets".into(), Value::Object(presets));
                self.settings.persist()?;
                Ok(json!({"saved": name}))
            })()),
            "shortcuts.load_preset" => Some((|| {
                let params: ShortcutPresetParams = parse_params(params)?;
                let name = params.name.trim();
                if name == "default" {
                    self.settings.write_shortcuts_map(Default::default(), true)?;
                    return Ok(json!({"loaded": name, "shortcuts": Value::Object(Default::default())}));
                }
                let presets = self.settings.presets_map();
                let Some(preset) = presets.get(name).and_then(|value| value.as_object().cloned()) else {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Unknown shortcut preset: {name}"
                    )));
                };
                let loaded = preset.clone();
                self.settings.write_shortcuts_map(preset, true)?;
                Ok(json!({"loaded": name, "shortcuts": Value::Object(loaded)}))
            })()),
            "shortcuts.delete_preset" => Some((|| {
                let params: ShortcutPresetParams = parse_params(params)?;
                let name = params.name.trim();
                if name == "default" {
                    return Err(DispatchError::InvalidParameters(
                        "The default preset cannot be deleted".into(),
                    ));
                }
                let mut presets = self.settings.presets_map();
                if presets.remove(name).is_none() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Unknown shortcut preset: {name}"
                    )));
                }
                self.settings.data.insert("shortcut_presets".into(), Value::Object(presets));
                self.settings.persist()?;
                Ok(json!({"deleted": name}))
            })()),
            // Settings-scoped undo/redo over the shortcut map.
            "shortcuts.undo" => Some((|| {
                let Some(previous) = self.settings.shortcut_undo.pop() else {
                    return Err(DispatchError::Operation(
                        "No shortcut change to undo".into(),
                    ));
                };
                let current = self.settings.shortcuts_map();
                self.settings.shortcut_redo.push(current);
                self.settings
                    .data
                    .insert("shortcuts".into(), Value::Object(previous));
                self.settings.persist()?;
                Ok(json!({
                    "undone": true,
                    "shortcuts": self.settings.object("shortcuts"),
                }))
            })()),
            "shortcuts.redo" => Some((|| {
                let Some(next) = self.settings.shortcut_redo.pop() else {
                    return Err(DispatchError::Operation(
                        "No shortcut change to redo".into(),
                    ));
                };
                let current = self.settings.shortcuts_map();
                self.settings.shortcut_undo.push(current);
                self.settings
                    .data
                    .insert("shortcuts".into(), Value::Object(next));
                self.settings.persist()?;
                Ok(json!({
                    "redone": true,
                    "shortcuts": self.settings.object("shortcuts"),
                }))
            })()),
            _ => None,
        }
    }

    fn write_shortcuts(&mut self, params: Value, validate: bool) -> Result<Value, DispatchError> {
        let params: ShortcutsParams = parse_params(params)?;
        if validate {
            for id in params.shortcuts.keys() {
                if find_command(id).is_none() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Unknown command in shortcut import: {id}"
                    )));
                }
            }
        }
        self.settings.write_shortcuts_map(params.shortcuts, true)?;
        Ok(json!({"shortcuts": self.settings.object("shortcuts")}))
    }
}

#[derive(Deserialize)]
struct PrefGetParams {
    #[serde(default)]
    key: Option<String>,
}

#[derive(Deserialize)]
struct PrefSetParams {
    values: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct ShortcutsParams {
    shortcuts: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct ShortcutGetParams {
    command_id: String,
}

#[derive(Deserialize)]
struct ShortcutForKeyParams {
    key: String,
}

#[derive(Deserialize)]
struct ShortcutClearParams {
    command_id: String,
}

#[derive(Deserialize)]
struct ShortcutResolveParams {
    key: String,
    /// The command that keeps the key; every other binding is removed.
    keep: String,
}

#[derive(Deserialize)]
struct ShortcutExportParams {
    path: String,
}

#[derive(Deserialize)]
struct ShortcutPresetParams {
    name: String,
}
