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
pub(in crate::engine) const SEARCH_BINS_KEY: &str = "project_search_bins";

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
    pub(in crate::engine) fn panel_view(&self) -> Value {
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

    pub(in crate::engine) fn store_panel_view(&mut self, view: Value) -> Result<(), DispatchError> {
        self.store_pref(PANEL_VIEW_KEY, view)
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

    /// Saved search bins: `{name: query}` in the shared prefs object.
    fn search_bins(&self) -> serde_json::Map<String, Value> {
        self.settings
            .object("prefs")
            .get(SEARCH_BINS_KEY)
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default()
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
                    if mode != "grid" && mode != "list" && mode != "freeform" {
                        return Err(DispatchError::InvalidParameters(
                            "View mode must be 'grid', 'list', or 'freeform'".into(),
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
            "project.remove" => Some((|| {
                let params: ProjectRemoveParams = parse_params(params)?;
                let ids = {
                    let mut unique = params.media_ids;
                    unique.sort_unstable();
                    unique.dedup();
                    if unique.is_empty() {
                        return Err(DispatchError::InvalidParameters(
                            "Select one or more media items to clear".into(),
                        ));
                    }
                    for id in &unique {
                        if !self.document.project.media.iter().any(|asset| asset.id == *id) {
                            return Err(DispatchError::InvalidParameters(format!(
                                "Media item {} does not exist",
                                id
                            )));
                        }
                    }
                    unique
                };
                // Media referenced by timeline clips stays put; FC's Clear
                // warns the same way.
                let in_use: Vec<u64> = ids
                    .iter()
                    .filter(|id| {
                        self.document
                            .project
                            .sequence
                            .tracks
                            .iter()
                            .any(|track| track.clips.iter().any(|clip| clip.media_id == **id))
                    })
                    .copied()
                    .collect();
                if !in_use.is_empty() {
                    return Err(DispatchError::Operation(format!(
                        "{} media item(s) are used by timeline clips; remove or ripple-delete those clips first",
                        in_use.len()
                    )));
                }
                self.checkpoint();
                self.document
                    .project
                    .media
                    .retain(|asset| !ids.contains(&asset.id));
                Ok(json!({"removed": ids.len(), "media_ids": ids}))
            })()),
            "project.searchBin.set" => Some((|| {
                let params: SearchBinSetParams = parse_params(params)?;
                let name = params.name.trim();
                if name.is_empty() || name.len() > 64 {
                    return Err(DispatchError::InvalidParameters(
                        "Search bin name must be 1-64 characters".into(),
                    ));
                }
                let mut bins = self.search_bins();
                let query = params.query.trim().to_string();
                if query.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Search bin query must not be empty".into(),
                    ));
                }
                bins.insert(name.to_string(), json!(query));
                self.store_pref(SEARCH_BINS_KEY, Value::Object(bins))?;
                Ok(json!({"saved": name, "query": query}))
            })()),
            "project.searchBin.delete" => Some((|| {
                let params: SearchBinNameParams = parse_params(params)?;
                let name = params.name.trim();
                let mut bins = self.search_bins();
                if bins.remove(name).is_none() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "No search bin named '{name}'"
                    )));
                }
                self.store_pref(SEARCH_BINS_KEY, Value::Object(bins))?;
                Ok(json!({"deleted": name}))
            })()),
            "project.searchBin.items" => Some((|| {
                let params: SearchBinNameParams = parse_params(params)?;
                let name = params.name.trim();
                let bins = self.search_bins();
                let query = bins
                    .get(name)
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "No search bin named '{name}'"
                        ))
                    })?;
                let needle = query.to_lowercase();
                let matches: Vec<Value> = self
                    .document
                    .project
                    .media
                    .iter()
                    .filter(|asset| {
                        format!("{} {:?}", asset.name, asset.kind).to_lowercase().contains(&needle)
                    })
                    .map(|asset| json!({"id": asset.id, "name": asset.name}))
                    .collect();
                Ok(json!({
                    "bin": name,
                    "query": query,
                    "matches": matches,
                    "count": matches.len(),
                }))
            })()),
            _ => None,
        }
    }
}
