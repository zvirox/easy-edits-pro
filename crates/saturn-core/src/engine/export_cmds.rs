//! Export machinery the Rust side truly owns: the supported-format table,
//! named export presets, and settings resolution. The GStreamer render
//! itself runs in the Tauri shell (queue commands are ui_only and route to
//! the shell's JobManager); these commands give the CLI and MCP bridge the
//! same preset/settings surface as the Deliver workspace.

use super::*;

/// The render pipeline's real capability table (saturn-render): two
/// container/codec pairs, hardware encoding available on the MP4 path.
const EXPORT_FORMATS: &[(&str, &str, &str, &str, bool)] = &[
    ("mp4", "H.264 · MP4", "h264", "aac", true),
    ("webm", "VP8 · WebM", "vp8", "vorbis", false),
];

/// Built-in presets ship with the product; user presets live in the
/// settings store next to them.
const FACTORY_PRESETS: &[(&str, &str)] = &[
    (
        "source",
        r#"{"format":"mp4","resolution":"source","frame_rate":"source","use_mark_range":false,"use_hardware":true}"#,
    ),
    (
        "youtube-4k",
        r#"{"format":"mp4","resolution":"3840x2160","frame_rate":"source","use_mark_range":false,"use_hardware":true}"#,
    ),
    (
        "social-vertical",
        r#"{"format":"mp4","resolution":"1080x1920","frame_rate":"source","use_mark_range":false,"use_hardware":true}"#,
    ),
    (
        "webm-archive",
        r#"{"format":"webm","resolution":"source","frame_rate":"source","use_mark_range":false,"use_hardware":false}"#,
    ),
];

fn factory_preset(name: &str) -> Option<Value> {
    FACTORY_PRESETS
        .iter()
        .find(|(factory, _)| *factory == name)
        .map(|(_, json)| serde_json::from_str(json).expect("factory preset is valid JSON"))
}

impl EditorEngine {
    pub(super) fn dispatch_export(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            // The shell's Deliver workspace and saturn-render agree on
            // this table; hardware accel follows the MP4 path only.
            "export.formats" => Some(Ok(json!({
                "formats": EXPORT_FORMATS.iter().map(|(id, label, video, audio, hw)| json!({
                    "id": id,
                    "label": label,
                    "video_codec": video,
                    "audio_codec": audio,
                    "hardware_accel": hw,
                })).collect::<Vec<_>>(),
            }))),
            "export.presets" => Some((|| {
                // The list mirrors presets.get's precedence: user presets,
                // with factory presets shadowing any same-named override.
                let mut presets = self.export_preset_map();
                for (name, _) in FACTORY_PRESETS {
                    if let Some(factory) = factory_preset(name) {
                        presets.insert((*name).to_string(), factory);
                    }
                }
                Ok(json!({ "presets": presets }))
            })()),
            "export.presets.save" => Some((|| {
                let params: ExportPresetSaveParams = parse_params(params)?;
                let name = params.name.trim();
                if name.is_empty() {
                    return Err(DispatchError::InvalidParameters(
                        "Preset name cannot be empty".into(),
                    ));
                }
                if !params
                    .settings
                    .get("format")
                    .and_then(|value| value.as_str())
                    .is_some_and(|format| {
                        EXPORT_FORMATS.iter().any(|(known, ..)| *known == format)
                    })
                {
                    return Err(DispatchError::InvalidParameters(
                        "Preset format must be one of: mp4, webm".into(),
                    ));
                }
                let mut presets = self.export_preset_map();
                presets.insert(name.to_string(), Value::Object(params.settings));
                self.store_export_presets(presets)?;
                Ok(json!({"saved": name}))
            })()),
            "export.presets.get" => Some((|| {
                let params: ExportPresetNameParams = parse_params(params)?;
                let name = params.name.trim();
                let preset = factory_preset(name)
                    .or_else(|| self.export_preset_map().get(name).cloned());
                Ok(match preset {
                    Some(settings) => json!({"name": name, "settings": settings}),
                    None => json!({"name": name, "settings": null}),
                })
            })()),
            "export.presets.delete" => Some((|| {
                let params: ExportPresetNameParams = parse_params(params)?;
                let name = params.name.trim();
                let mut presets = self.export_preset_map();
                if presets.remove(name).is_none() {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Unknown export preset: {name} (factory presets cannot be deleted)"
                    )));
                }
                self.store_export_presets(presets)?;
                Ok(json!({"deleted": name}))
            })()),
            "export.presets.favorite" => Some((|| {
                let params: ExportPresetFavoriteParams = parse_params(params)?;
                let name = params.name.trim();
                if factory_preset(name).is_none() && !self.export_preset_map().contains_key(name)
                {
                    return Err(DispatchError::InvalidParameters(format!(
                        "Unknown export preset: {name}"
                    )));
                }
                let mut favorites = self.favorite_export_presets();
                if params.favorite {
                    favorites.insert(name.to_string());
                } else {
                    favorites.remove(name);
                }
                let list: Vec<&String> = favorites.iter().collect();
                self.settings.store_object("export_preset_favorites", json!(list))?;
                Ok(json!({"name": name, "favorite": params.favorite, "favorites": list}))
            })()),
            "export.presets.import" => Some((|| {
                let params: ExportPresetsImportParams = parse_params(params)?;
                for (name, settings) in &params.presets {
                    if !settings.get("format").is_some_and(|format| {
                        format.as_str().is_some_and(|format| {
                            EXPORT_FORMATS.iter().any(|(known, ..)| *known == format)
                        })
                    }) {
                        return Err(DispatchError::InvalidParameters(format!(
                            "Preset {name} needs a format of mp4 or webm"
                        )));
                    }
                }
                let mut presets = self.export_preset_map();
                let imported = params.presets.len();
                for (name, settings) in params.presets {
                    presets.insert(name, settings);
                }
                self.store_export_presets(presets)?;
                Ok(json!({"imported": imported}))
            })()),
            "export.presets.export" => Some((|| {
                let params: ExportPresetsWriteParams = parse_params(params)?;
                let payload = serde_json::to_vec_pretty(&json!({
                    "presets": self.export_preset_map(),
                }))
                .map_err(|error| {
                    DispatchError::Operation(format!("Could not serialise presets: {error}"))
                })?;
                std::fs::write(&params.path, payload).map_err(|error| {
                    DispatchError::Operation(format!("Could not write presets file: {error}"))
                })?;
                Ok(json!({
                    "exported": self.export_preset_map().len(),
                    "path": params.path,
                }))
            })()),
            // Pure computation: turn a preset (or explicit overrides) into
            // the concrete render settings for THIS project, including the
            // marked range when requested. The shell feeds the result to
            // start_export; nothing here renders.
            "export.resolve" => Some((|| {
                let params: ExportResolveParams = parse_params(params)?;
                let settings = match &params.preset {
                    Some(name) => {
                        let name = name.trim();
                        factory_preset(name)
                            .or_else(|| self.export_preset_map().get(name).cloned())
                            .ok_or_else(|| {
                                DispatchError::InvalidParameters(format!(
                                    "Unknown export preset: {name}"
                                ))
                            })?
                    }
                    None => json!({}),
                };
                let pick = |key: &str, explicit: &Option<Value>| {
                    explicit.clone().or_else(|| settings.get(key).cloned())
                };
                let project = &self.document.project;
                let format = pick("format", &params.format)
                    .and_then(|value| value.as_str().map(str::to_string))
                    .unwrap_or_else(|| "mp4".to_string());
                if !EXPORT_FORMATS.iter().any(|(known, ..)| *known == format) {
                    return Err(DispatchError::InvalidParameters(
                        "Format must be one of: mp4, webm".into(),
                    ));
                }
                let (default_width, default_height) =
                    (project.settings.width, project.settings.height);
                let resolution = match pick("resolution", &params.resolution)
                    .and_then(|value| value.as_str().map(str::to_string))
                {
                    Some(value) if value == "source" || value.is_empty() => {
                        (default_width, default_height)
                    }
                    Some(value) => {
                        let mut parts = value.splitn(2, 'x');
                        let width = parts.next().and_then(|width| width.parse::<u32>().ok());
                        let height = parts.next().and_then(|height| height.parse::<u32>().ok());
                        match (width, height) {
                            (Some(width), Some(height)) if width > 0 && height > 0 => (width, height),
                            _ => {
                                return Err(DispatchError::InvalidParameters(
                                    "Resolution must be \"source\" or \"WIDTHxHEIGHT\"".into(),
                                ))
                            }
                        }
                    }
                    None => (default_width, default_height),
                };
                let frame_rate = match pick("frame_rate", &params.frame_rate)
                    .and_then(|value| value.as_str().map(str::to_string))
                {
                    Some(value) if value == "source" || value.is_empty() => {
                        let rate = &project.settings.frame_rate;
                        (rate.numerator, rate.denominator)
                    }
                    Some(value) => {
                        let mut parts = value.splitn(2, '/');
                        let numerator = parts.next().and_then(|n| n.parse::<u32>().ok());
                        let denominator = parts.next().and_then(|d| d.parse::<u32>().ok());
                        match (numerator, denominator) {
                            (Some(numerator), Some(denominator)) if denominator > 0 => {
                                (numerator, denominator)
                            }
                            _ => {
                                return Err(DispatchError::InvalidParameters(
                                    "Frame rate must be \"source\" or \"NUM/DEN\"".into(),
                                ))
                            }
                        }
                    }
                    None => {
                        let rate = &project.settings.frame_rate;
                        (rate.numerator, rate.denominator)
                    }
                };
                let use_mark_range = pick("use_mark_range", &params.use_mark_range)
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                let range = if use_mark_range {
                    match (project.sequence.mark_in, project.sequence.mark_out) {
                        (Some(start), Some(end)) => {
                            if end <= start {
                                return Err(DispatchError::InvalidParameters(
                                    "Mark Out must come after Mark In".into(),
                                ));
                            }
                            Some(json!({"start": start, "end": end}))
                        }
                        _ => {
                            return Err(DispatchError::InvalidParameters(
                                "Set both In and Out points before resolving a marked-range export"
                                    .into(),
                            ))
                        }
                    }
                } else {
                    None
                };
                let use_hardware = pick("use_hardware", &params.use_hardware)
                    .and_then(|value| value.as_bool())
                    .unwrap_or(format == "mp4");
                Ok(json!({
                    "format": format,
                    "width": resolution.0,
                    "height": resolution.1,
                    "frame_rate_numerator": frame_rate.0,
                    "frame_rate_denominator": frame_rate.1,
                    "use_mark_range": use_mark_range,
                    "range": range,
                    "use_hardware": use_hardware,
                }))
            })()),
            _ => None,
        }
    }

    fn export_preset_map(&self) -> serde_json::Map<String, Value> {
        self.settings
            .object("export_presets")
            .as_object()
            .cloned()
            .unwrap_or_default()
    }

    fn store_export_presets(
        &mut self,
        presets: serde_json::Map<String, Value>,
    ) -> Result<(), DispatchError> {
        self.settings.store_object("export_presets", Value::Object(presets))
    }

    fn favorite_export_presets(&self) -> std::collections::BTreeSet<String> {
        self.settings
            .object("export_preset_favorites")
            .as_array()
            .cloned()
            .map(|list| {
                list.into_iter()
                    .filter_map(|entry| entry.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Deserialize)]
struct ExportPresetSaveParams {
    name: String,
    settings: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct ExportPresetNameParams {
    name: String,
}

#[derive(Deserialize)]
struct ExportPresetFavoriteParams {
    name: String,
    #[serde(default = "default_true")]
    favorite: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct ExportPresetsImportParams {
    presets: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct ExportPresetsWriteParams {
    path: String,
}

#[derive(Deserialize)]
struct ExportResolveParams {
    #[serde(default)]
    preset: Option<String>,
    #[serde(default)]
    format: Option<Value>,
    #[serde(default)]
    resolution: Option<Value>,
    #[serde(default)]
    frame_rate: Option<Value>,
    #[serde(default)]
    use_mark_range: Option<Value>,
    #[serde(default)]
    use_hardware: Option<Value>,
}
