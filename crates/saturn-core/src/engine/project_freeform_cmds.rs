//! Project freeform family: reference-editor freeform-view layout for the media pool.
//!
//! Card positions and named arrangements persist in the shared "prefs"
//! settings object (like the panel view state), so they survive restarts
//! and are scriptable through `prefs.get`; they are engine settings, not
//! document data, so no arm checkpoints. Positions are keyed by media id
//! as strings and store `{x, y, scale}` in pixels.
//!
//! `project.freeform.layout` flips the stored panel view between "list"
//! and "freeform" (the panel view helpers live in project_panel_cmds).

use super::*;

const FREEFORM_POSITIONS_KEY: &str = "project_freeform_positions";
const FREEFORM_ARRANGEMENTS_KEY: &str = "project_freeform_arrangements";
const FREEFORM_GRID_KEY: &str = "project_freeform_grid";

/// Grid size in pixels; `alignToGrid` snaps to it and `unstack` spreads
/// cards on a `CELL = grid * 4` pitch (160px by default).
const FREEFORM_GRID_DEFAULT: f64 = 40.0;
const FREEFORM_CELL_PITCH: f64 = 4.0;
/// Cascade offset for stacked cards so the pile stays visually inspectable.
const FREEFORM_STACK_CASCADE: f64 = 12.0;

type Positions = serde_json::Map<String, Value>;

fn default_position() -> Value {
    json!({"x": 0.0, "y": 0.0, "scale": 1.0})
}

impl EditorEngine {
    fn freeform_grid(&self) -> f64 {
        self.settings
            .object("prefs")
            .get(FREEFORM_GRID_KEY)
            .and_then(|value| value.as_f64())
            .filter(|grid| *grid > 0.0)
            .unwrap_or(FREEFORM_GRID_DEFAULT)
    }

    fn freeform_positions(&self) -> Positions {
        self.settings
            .object("prefs")
            .get(FREEFORM_POSITIONS_KEY)
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default()
    }

    fn freeform_arrangements(&self) -> Positions {
        self.settings
            .object("prefs")
            .get(FREEFORM_ARRANGEMENTS_KEY)
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default()
    }

    fn store_freeform_positions(&mut self, positions: Positions) -> Result<(), DispatchError> {
        self.store_pref(FREEFORM_POSITIONS_KEY, Value::Object(positions))
    }

    /// Validate that every media id exists and return them sorted/deduped.
    fn validate_freeform_media(&self, ids: &[u64]) -> Result<Vec<u64>, DispatchError> {
        if ids.is_empty() {
            return Err(DispatchError::InvalidParameters(
                "Select one or more media items".into(),
            ));
        }
        let mut unique = ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        for id in &unique {
            if !self.document.project.media.iter().any(|asset| asset.id == *id) {
                return Err(DispatchError::InvalidParameters(format!(
                    "Media item {} does not exist",
                    id
                )));
            }
        }
        Ok(unique)
    }

    pub(super) fn dispatch_project_freeform(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "project.freeform.layout" => Some(self.freeform_layout(params)),
            "project.freeform.move" => Some(self.freeform_move(params)),
            "project.freeform.resize" => Some(self.freeform_resize(params)),
            "project.freeform.alignToGrid" => Some(self.freeform_align()),
            "project.freeform.reset" => Some(self.freeform_reset()),
            "project.freeform.stack" => Some(self.freeform_stack(params, true)),
            "project.freeform.unstack" => Some(self.freeform_stack(params, false)),
            "project.freeform.options" => Some(self.freeform_options(params)),
            "project.freeform.saveArrangement" => Some(self.freeform_save_arrangement(params)),
            "project.freeform.restoreArrangement" => Some(self.freeform_restore_arrangement(params)),
            "project.freeform.deleteArrangement" => Some(self.freeform_delete_arrangement(params)),
            "project.freeform.arrangements" => Some((|| {
                let arrangements = self.freeform_arrangements();
                let names: Vec<String> = arrangements.keys().cloned().collect();
                Ok(json!({"arrangements": names}))
            })()),
            _ => None,
        }
    }

    /// `project.freeform.layout` — flip or set the panel view mode between
    /// "list" and "freeform". `enabled` defaults to flipping the current mode.
    fn freeform_layout(&mut self, params: Value) -> Result<Value, DispatchError> {
        let params: FreeformLayoutParams = if params.is_null() {
            FreeformLayoutParams { enabled: None }
        } else {
            parse_params(params)?
        };
        let mut view = self.panel_view();
        let current = view
            .get("view_mode")
            .and_then(|mode| mode.as_str())
            .unwrap_or("grid")
            .to_string();
        let freeform = params.enabled.unwrap_or(current != "freeform");
        let mode = if freeform { "freeform" } else { "list" };
        view["view_mode"] = json!(mode);
        self.store_panel_view(view)?;
        Ok(json!({"view_mode": mode, "freeform": freeform}))
    }

    fn freeform_move(&mut self, params: Value) -> Result<Value, DispatchError> {
        let params: FreeformMoveParams = parse_params(params)?;
        let ids = self.validate_freeform_media(&params.media_ids)?;
        let mut positions = self.freeform_positions();
        for id in &ids {
            let key = id.to_string();
            let mut position = positions.get(&key).cloned().unwrap_or_else(default_position);
            position["x"] = json!(position.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) + params.dx);
            position["y"] = json!(position.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) + params.dy);
            positions.insert(key, position);
        }
        self.store_freeform_positions(positions)?;
        Ok(json!({"moved": ids.len(), "dx": params.dx, "dy": params.dy}))
    }

    fn freeform_resize(&mut self, params: Value) -> Result<Value, DispatchError> {
        let params: FreeformResizeParams = parse_params(params)?;
        if !(0.2..=5.0).contains(&params.scale) {
            return Err(DispatchError::InvalidParameters(
                "Clip size scale must be between 0.2 and 5".into(),
            ));
        }
        let ids = self.validate_freeform_media(&params.media_ids)?;
        let mut positions = self.freeform_positions();
        for id in &ids {
            let key = id.to_string();
            let mut position = positions.get(&key).cloned().unwrap_or_else(default_position);
            position["scale"] = json!(params.scale);
            positions.insert(key, position);
        }
        self.store_freeform_positions(positions)?;
        Ok(json!({"resized": ids.len(), "scale": params.scale}))
    }

    fn freeform_align(&mut self) -> Result<Value, DispatchError> {
        let grid = self.freeform_grid();
        let mut positions = self.freeform_positions();
        for (_key, position) in positions.iter_mut() {
            let x = position.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = position.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            position["x"] = json!((x / grid).round() * grid);
            position["y"] = json!((y / grid).round() * grid);
        }
        self.store_freeform_positions(positions)?;
        Ok(json!({"aligned_to_grid": grid}))
    }

    fn freeform_reset(&mut self) -> Result<Value, DispatchError> {
        let cleared = self.freeform_positions().len();
        self.store_freeform_positions(serde_json::Map::new())?;
        Ok(json!({"reset": true, "cleared": cleared}))
    }

    /// Stack piles the cards on one origin with a small cascade; unstack
    /// spreads them on a square grid using the cell pitch.
    fn freeform_stack(&mut self, params: Value, stack: bool) -> Result<Value, DispatchError> {
        let params: FreeformStackParams = parse_params(params)?;
        let ids = self.validate_freeform_media(&params.media_ids)?;
        let mut positions = self.freeform_positions();
        if stack {
            for (offset, id) in ids.iter().enumerate() {
                let cascade = offset as f64 * FREEFORM_STACK_CASCADE;
                let key = id.to_string();
                let mut position = positions.get(&key).cloned().unwrap_or_else(default_position);
                position["x"] = json!(cascade);
                position["y"] = json!(cascade);
                positions.insert(key, position);
            }
        } else {
            let columns = (ids.len() as f64).sqrt().ceil().max(1.0) as usize;
            let cell = self.freeform_grid() * FREEFORM_CELL_PITCH;
            for (index, id) in ids.iter().enumerate() {
                let key = id.to_string();
                let mut position = positions.get(&key).cloned().unwrap_or_else(default_position);
                position["x"] = json!((index % columns) as f64 * cell);
                position["y"] = json!((index / columns) as f64 * cell);
                positions.insert(key, position);
            }
        }
        self.store_freeform_positions(positions)?;
        Ok(json!({"updated": ids.len(), "stacked": stack}))
    }

    fn freeform_options(&mut self, params: Value) -> Result<Value, DispatchError> {
        let params: FreeformOptionsParams = parse_params(params)?;
        if let Some(grid) = params.grid {
            if grid <= 0.0 || grid > 500.0 {
                return Err(DispatchError::InvalidParameters(
                    "Grid size must be between 0 and 500 pixels".into(),
                ));
            }
            self.store_pref(FREEFORM_GRID_KEY, json!(grid))?;
            return Ok(json!({"grid": grid}));
        }
        Ok(json!({"grid": self.freeform_grid()}))
    }

    fn freeform_save_arrangement(&mut self, params: Value) -> Result<Value, DispatchError> {
        let params: FreeformArrangementParams = parse_params(params)?;
        let name = params.name.trim();
        if name.is_empty() || name.len() > 64 {
            return Err(DispatchError::InvalidParameters(
                "Arrangement name must be 1-64 characters".into(),
            ));
        }
        let mut arrangements = self.freeform_arrangements();
        let snapshot = json!({
            "positions": Value::Object(self.freeform_positions()),
            "grid": self.freeform_grid(),
            "items": self.freeform_positions().len(),
        });
        arrangements.insert(name.to_string(), snapshot);
        let names: Vec<String> = arrangements.keys().cloned().collect();
        self.store_pref(FREEFORM_ARRANGEMENTS_KEY, Value::Object(arrangements))?;
        Ok(json!({"saved": name, "arrangements": names}))
    }

    fn freeform_restore_arrangement(&mut self, params: Value) -> Result<Value, DispatchError> {
        let params: FreeformArrangementParams = parse_params(params)?;
        let name = params.name.trim();
        let arrangements = self.freeform_arrangements();
        let snapshot = arrangements.get(name).cloned().ok_or_else(|| {
            DispatchError::InvalidParameters(format!("No arrangement named '{name}'"))
        })?;
        let positions = snapshot
            .get("positions")
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();
        if let Some(grid) = snapshot.get("grid").and_then(|value| value.as_f64()) {
            self.store_pref(FREEFORM_GRID_KEY, json!(grid))?;
        }
        self.store_freeform_positions(positions)?;
        Ok(json!({"restored": name, "items": self.freeform_positions().len()}))
    }

    fn freeform_delete_arrangement(&mut self, params: Value) -> Result<Value, DispatchError> {
        let params: FreeformArrangementParams = parse_params(params)?;
        let name = params.name.trim();
        let mut arrangements = self.freeform_arrangements();
        if arrangements.remove(name).is_none() {
            return Err(DispatchError::InvalidParameters(format!(
                "No arrangement named '{name}'"
            )));
        }
        self.store_pref(FREEFORM_ARRANGEMENTS_KEY, Value::Object(arrangements))?;
        Ok(json!({"deleted": name}))
    }
}
