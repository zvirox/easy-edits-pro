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
        }
    }

    fn object(&self, key: &str) -> Value {
        self.data.get(key).cloned().unwrap_or_else(|| json!({}))
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
            "shortcuts.list" => Some(Ok(json!({"shortcuts": self.settings.object("shortcuts")}))),
            "shortcuts.set" => Some(self.write_shortcuts(params, false)),
            "shortcuts.import" => Some(self.write_shortcuts(params, true)),
            "shortcuts.reset" => Some((|| {
                self.settings.data.remove("shortcuts");
                self.settings.persist()?;
                Ok(json!({"reset": true}))
            })()),
            "workspace.list" => Some(Ok(json!({"presets": self.workspace_presets()}))),
            "workspace.save" | "workspace.save_as" => Some((|| {
                let params: WorkspacePresetParams = parse_params(params)?;
                let preset = params.preset;
                let id = preset
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("Layout preset needs an id".into())
                    })?
                    .to_owned();
                let mut presets = self.workspace_presets();
                if let Some(existing) = presets
                    .iter_mut()
                    .find(|entry| entry.get("id").and_then(Value::as_str) == Some(&id))
                {
                    *existing = preset;
                } else {
                    presets.push(preset);
                }
                self.settings
                    .data
                    .insert("workspaces".into(), Value::Array(presets.clone()));
                self.settings.persist()?;
                Ok(json!({"presets": presets}))
            })()),
            "workspace.rename" => Some((|| {
                let params: WorkspaceRenameParams = parse_params(params)?;
                let mut presets = self.workspace_presets();
                let preset = presets
                    .iter_mut()
                    .find(|entry| entry.get("id").and_then(Value::as_str) == Some(&params.id))
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("Layout preset does not exist".into())
                    })?;
                let object = preset
                    .as_object_mut()
                    .ok_or_else(|| DispatchError::Operation("Layout preset is corrupt".into()))?;
                object.insert("label".into(), Value::String(params.label));
                self.settings
                    .data
                    .insert("workspaces".into(), Value::Array(presets.clone()));
                self.settings.persist()?;
                Ok(json!({"presets": presets}))
            })()),
            "workspace.delete" => Some((|| {
                let params: WorkspaceIdParams = parse_params(params)?;
                let mut presets = self.workspace_presets();
                presets.retain(|entry| entry.get("id").and_then(Value::as_str) != Some(&params.id));
                self.settings
                    .data
                    .insert("workspaces".into(), Value::Array(presets.clone()));
                self.settings.persist()?;
                Ok(json!({"presets": presets, "deleted": true}))
            })()),
            "workspace.reset" => Some((|| {
                self.settings.data.remove("workspaces");
                self.settings.persist()?;
                Ok(json!({"reset": true}))
            })()),
            _ => None,
        }
    }

    fn workspace_presets(&self) -> Vec<Value> {
        self.settings
            .data
            .get("workspaces")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
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
        self.settings
            .data
            .insert("shortcuts".into(), Value::Object(params.shortcuts.clone()));
        self.settings.persist()?;
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
struct WorkspacePresetParams {
    preset: Value,
}

#[derive(Deserialize)]
struct WorkspaceRenameParams {
    id: String,
    label: String,
}

#[derive(Deserialize)]
struct WorkspaceIdParams {
    id: String,
}
