//! Project-panel family: media-pool view state (mode/sort/columns), named
//! view presets, panel row listing, project notes, and source-monitor marks.
//!
//! View state lives in the shared "prefs" settings object (like the marker
//! preferences) so it survives restarts and is scriptable through
//! `prefs.get`. Notes and source marks are document data, so those arms go
//! through `checkpoint` and are undoable.

use super::*;

/// Preferences keys (nested under the "prefs" settings object).
pub(in crate::engine) const PANEL_VIEW_KEY: &str = "project_panel_view";
pub(in crate::engine) const PANEL_PRESETS_KEY: &str = "project_view_presets";
pub(in crate::engine) const PANEL_PRESET_CURRENT_KEY: &str = "project_view_preset_current";

pub(in crate::engine) const PANEL_DEFAULT_COLUMNS: [&str; 6] =
    ["name", "kind", "duration", "fps", "codec", "bin"];

fn default_panel_view() -> Value {
    json!({
        "view_mode": "grid",
        "sort": {"key": "name", "dir": "asc"},
        "columns": PANEL_DEFAULT_COLUMNS
            .iter()
            .map(|id| json!({"id": id, "visible": true}))
            .collect::<Vec<_>>(),
    })
}

impl EditorEngine {
    /// Read the stored panel view, filling defaults for missing sections.
    fn panel_view(&self) -> Value {
        let stored = self.settings.object("prefs").get(PANEL_VIEW_KEY).cloned();
        let mut merged = default_panel_view();
        if let (Some(target), Some(stored)) = (merged.as_object_mut(), stored.and_then(|v| v.as_object().cloned()))
        {
            for (key, value) in stored {
                target.insert(key, value);
            }
        }
        merged
    }

    fn store_panel_view(&mut self, view: Value) -> Result<(), DispatchError> {
        let mut prefs = self
            .settings
            .object("prefs")
            .as_object()
            .cloned()
            .unwrap_or_default();
        prefs.insert(PANEL_VIEW_KEY.into(), view);
        self.settings.store_object("prefs", Value::Object(prefs))
    }

    fn presets_map(&self) -> serde_json::Map<String, Value> {
        self.settings
            .object("prefs")
            .get(PANEL_PRESETS_KEY)
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default()
    }

    fn store_presets(&mut self, presets: serde_json::Map<String, Value>) -> Result<(), DispatchError> {
        let mut prefs = self
            .settings
            .object("prefs")
            .as_object()
            .cloned()
            .unwrap_or_default();
        prefs.insert(PANEL_PRESETS_KEY.into(), Value::Object(presets));
        self.settings.store_object("prefs", Value::Object(prefs))
    }

    fn store_current_preset(&mut self, name: &str) -> Result<(), DispatchError> {
        let mut prefs = self
            .settings
            .object("prefs")
            .as_object()
            .cloned()
            .unwrap_or_default();
        prefs.insert(PANEL_PRESET_CURRENT_KEY.into(), json!(name));
        self.settings.store_object("prefs", Value::Object(prefs))
    }

    pub(super) fn dispatch_project_panel(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "project.view.get" => Some(Ok(json!({
                "view": self.panel_view(),
                "preset": self
                    .settings
                    .object("prefs")
                    .get(PANEL_PRESET_CURRENT_KEY)
                    .cloned(),
            }))),
            "project.view.set" => Some((|| {
                let params: ProjectViewSetParams = parse_params(params)?;
                let mut view = self.panel_view();
                if let Some(mode) = &params.view_mode {
                    if mode != "grid" && mode != "list" {
                        return Err(DispatchError::InvalidParameters(
                            "View mode must be 'grid' or 'list'".into(),
                        ));
                    }
                    view["view_mode"] = json!(mode);
                }
                if let Some(sort) = &params.sort {
                    if sort.dir != "asc" && sort.dir != "desc" {
                        return Err(DispatchError::InvalidParameters(
                            "Sort direction must be 'asc' or 'desc'".into(),
                        ));
                    }
                    view["sort"] = json!({"key": sort.key, "dir": sort.dir});
                }
                if let Some(columns) = &params.columns {
                    let mut ids = Vec::new();
                    for column in columns {
                        if column.id.trim().is_empty() {
                            return Err(DispatchError::InvalidParameters(
                                "Column ids cannot be empty".into(),
                            ));
                        }
                        if ids.contains(&column.id) {
                            return Err(DispatchError::InvalidParameters(format!(
                                "Duplicate column '{}'",
                                column.id
                            )));
                        }
                        ids.push(column.id.clone());
                    }
                    view["columns"] = json!(columns);
                }
                self.store_panel_view(view.clone())?;
                Ok(json!({"view": view}))
            })()),
            "project.sort" => Some((|| {
                let params: ProjectSortParams = parse_params(params)?;
                let dir = params.dir.unwrap_or_else(default_sort_dir);
                if dir != "asc" && dir != "desc" {
                    return Err(DispatchError::InvalidParameters(
                        "Sort direction must be 'asc' or 'desc'".into(),
                    ));
                }
                let mut view = self.panel_view();
                view["sort"] = json!({"key": params.key, "dir": dir});
                self.store_panel_view(view.clone())?;
                Ok(json!({"sort": view["sort"]}))
            })()),
            "project.columns.list" => Some(Ok(json!({
                "columns": self.panel_view()["columns"],
                "defaults": PANEL_DEFAULT_COLUMNS,
            }))),
            "project.columns.set" => Some((|| {
                let params: ProjectViewSetParams = parse_params(params)?;
                let columns = params.columns.ok_or_else(|| {
                    DispatchError::InvalidParameters("Provide the full 'columns' list".into())
                })?;
                let mut view = self.panel_view();
                view["columns"] = json!(columns);
                self.store_panel_view(view.clone())?;
                Ok(json!({"columns": view["columns"]}))
            })()),
            "project.columns.resize" => Some((|| {
                let params: PanelColumnResizeParams = parse_params(params)?;
                if params.width <= 0.0 {
                    return Err(DispatchError::InvalidParameters(
                        "Column width must be positive".into(),
                    ));
                }
                let mut view = self.panel_view();
                let mut columns: Vec<Value> = view["columns"]
                    .as_array()
                    .cloned()
                    .ok_or_else(|| {
                        DispatchError::Operation("Panel columns are not configured".into())
                    })?;
                let index = columns
                    .iter()
                    .position(|column| column["id"] == json!(params.id))
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Unknown column '{}'",
                            params.id
                        ))
                    })?;
                columns[index]["width"] = json!(params.width);
                view["columns"] = Value::Array(columns.clone());
                self.store_panel_view(view)?;
                Ok(json!({"column": columns[index]}))
            })()),
            "project.items" => Some(Ok(json!({
                "media": self.document.project.media.iter().map(|asset| json!({
                    "id": asset.id,
                    "name": asset.name,
                    "kind": asset.kind,
                    "bin_id": asset.bin_id,
                    "color_label": asset.color_label,
                    "duration": asset.duration,
                })).collect::<Vec<_>>(),
                "bins": self.document.project.bins.iter().map(|bin| json!({
                    "id": bin.id,
                    "name": bin.name,
                    "parent_id": bin.parent_id,
                })).collect::<Vec<_>>(),
            }))),
            "project.notes" => Some(Ok(json!({"notes": self.document.project.notes}))),
            "project.setNotes" => Some((|| {
                let params: ProjectNotesParams = parse_params(params)?;
                self.checkpoint();
                self.document.project.notes = params.text;
                Ok(json!({"notes": self.document.project.notes}))
            })()),
            "project.setMarks" => Some((|| {
                let params: ProjectMarksParams = parse_params(params)?;
                if let (Some(source_in), Some(source_out)) = (params.source_in, params.source_out) {
                    if source_out.0 <= source_in.0 {
                        return Err(DispatchError::InvalidParameters(
                            "The source mark-out must land after the mark-in".into(),
                        ));
                    }
                }
                let index = self
                    .document
                    .project
                    .media
                    .iter()
                    .position(|asset| asset.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("Media asset does not exist".into())
                    })?;
                self.checkpoint();
                let asset = &mut self.document.project.media[index];
                if let Some(source_in) = params.source_in {
                    asset.source_mark_in = Some(source_in);
                }
                if let Some(source_out) = params.source_out {
                    asset.source_mark_out = Some(source_out);
                }
                Ok(json!({
                    "media_id": params.media_id,
                    "source_mark_in": asset.source_mark_in,
                    "source_mark_out": asset.source_mark_out,
                }))
            })()),
            "project.viewPreset.save" => Some((|| {
                let params: ProjectPresetSaveParams = parse_params(params)?;
                let name = match params.name {
                    Some(name) if !name.trim().is_empty() => name.trim().to_owned(),
                    _ => self
                        .settings
                        .object("prefs")
                        .get(PANEL_PRESET_CURRENT_KEY)
                        .and_then(|value| value.as_str())
                        .map(str::to_owned)
                        .ok_or_else(|| {
                            DispatchError::InvalidParameters(
                                "Name the preset (Save As) before saving over it".into(),
                            )
                        })?,
                };
                let view = self.panel_view();
                let mut presets = self.presets_map();
                presets.insert(name.clone(), view.clone());
                self.store_presets(presets)?;
                self.store_current_preset(&name)?;
                Ok(json!({"preset": name, "view": view}))
            })()),
            "project.viewPreset.saveAs" => Some((|| {
                let params: ProjectPresetSaveParams = parse_params(params)?;
                let name = params
                    .name
                    .map(|name| name.trim().to_owned())
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters("A preset name is required".into())
                    })?;
                let view = self.panel_view();
                let mut presets = self.presets_map();
                presets.insert(name.clone(), view.clone());
                self.store_presets(presets)?;
                self.store_current_preset(&name)?;
                Ok(json!({"preset": name, "view": view}))
            })()),
            "project.viewPreset.list" => Some(Ok(json!({
                "presets": self.presets_map().keys().collect::<Vec<_>>(),
                "current": self
                    .settings
                    .object("prefs")
                    .get(PANEL_PRESET_CURRENT_KEY)
                    .cloned(),
            }))),
            "project.viewPreset.restore" => Some((|| {
                let params: ProjectPresetRestoreParams = parse_params(params)?;
                let presets = self.presets_map();
                let view = presets.get(&params.name).cloned().ok_or_else(|| {
                    DispatchError::InvalidParameters(format!(
                        "No view preset named '{}'",
                        params.name
                    ))
                })?;
                self.store_panel_view(view.clone())?;
                self.store_current_preset(&params.name)?;
                Ok(json!({"preset": params.name, "view": view}))
            })()),
            "project.viewPreset.rename" => Some((|| {
                let params: ProjectPresetRenameParams = parse_params(params)?;
                let mut presets = self.presets_map();
                if !presets.contains_key(&params.from) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "No view preset named '{}'",
                        params.from
                    )));
                }
                let view = presets.remove(&params.from).unwrap();
                if presets.contains_key(&params.to) {
                    return Err(DispatchError::InvalidParameters(format!(
                        "A preset named '{}' already exists",
                        params.to
                    )));
                }
                presets.insert(params.to.clone(), view);
                self.store_presets(presets)?;
                self.store_current_preset(&params.to)?;
                Ok(json!({"renamed": {"from": params.from, "to": params.to}}))
            })()),
            "project.viewPreset.delete" => Some((|| {
                let params: ProjectPresetRestoreParams = parse_params(params)?;
                let mut presets = self.presets_map();
                if presets.remove(&params.name).is_none() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "No view preset named '{}'",
                        params.name
                    )));
                }
                self.store_presets(presets)?;
                Ok(json!({"deleted": params.name}))
            })()),
            _ => None,
        }
    }
}
