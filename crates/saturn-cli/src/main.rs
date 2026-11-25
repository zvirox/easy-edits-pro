// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use saturn_core::{ActorContext, CommandRequest, EditorEngine, command_catalogue};
use saturn_graphics::format::{load_template, save_template};
use saturn_graphics::{GraphicsTemplate, TemplateCategory, TemplateMetadata};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("saturn-cli: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        return usage();
    };
    let mut engine = EditorEngine::default();
    match command.as_str() {
        "commands" => {
            let mut as_json = false;
            let mut filter = String::new();
            for arg in args {
                if arg == "--json" {
                    as_json = true;
                } else if filter.is_empty() {
                    filter = arg;
                }
            }
            let items: Vec<&saturn_core::CommandSpec> = if filter.is_empty() {
                command_catalogue().iter().collect()
            } else {
                saturn_core::search_commands(&filter)
            };
            if as_json {
                let value = Value::Array(
                    items
                        .iter()
                        .map(|spec| {
                            json!({
                                "id": spec.id,
                                "label": spec.label,
                                "menu": spec.menu,
                                "shortcut": spec.shortcut,
                                "params": spec.params,
                                "category": spec.category.label(),
                                "shipped": spec.shipped,
                                "uiOnly": spec.ui_only,
                            })
                        })
                        .collect(),
                );
                print_json(&value)?;
            } else {
                for spec in items {
                    println!(
                        "{}\t{}\t{}\t{}",
                        spec.id,
                        spec.label,
                        spec.menu_path(),
                        spec.shortcut.unwrap_or("")
                    );
                }
            }
        }
        "describe" => {
            let id = args.next().ok_or("missing command id")?;
            let spec = saturn_core::find_command(&id)
                .ok_or_else(|| format!("unknown command: {id}"))?;
            let status = engine.command_status(&id).map_err(|error| error.to_string())?;
            print_json(&json!({
                "id": spec.id,
                "label": spec.label,
                "menu": spec.menu,
                "defaultShortcut": spec.shortcut,
                "shortcut": spec.shortcut,
                "params": spec.params,
                "category": spec.category.label(),
                "mutating": spec.mutating,
                "undoGroup": spec.undo_group,
                "aliases": spec.aliases,
                "since": spec.since,
                "shipped": spec.shipped,
                "uiOnly": spec.ui_only,
                "enabled": status.enabled,
                "reason": status.reason,
            }))?;
        }
        "render" => {
            let project_path = required(&mut args, "project path")?;
            let output_path = required(&mut args, "output path")?;
            execute(&mut engine, "file.open", json!({ "path": project_path }))?;
            let document = engine.document().clone();
            let result = saturn_render::render_project(&document, &output_path, |progress| {
                eprintln!("render: {} {}", progress.phase, progress.fraction_percent);
            });
            result?;
            println!("Rendered {}", output_path.display());
        }
        "catalogue" => run_catalogue(args.collect())?,
        "control" => run_control(args.collect())?,
        "template" => run_template(args.collect())?,
        "cet-gen" => saturn_cli::cet_cmd::run_cet_gen_cli(args.collect())?,
        "cet-validate" => saturn_cli::cet_cmd::run_cet_validate(args.collect())?,
        "cet-propose" => saturn_cli::cet_cmd::run_cet_propose(args.collect())?,
        "cet-query" => saturn_cli::cet_cmd::run_cet_query(args.collect())?,
        "analyze-recipe" => saturn_cli::cet_cmd::run_analyze_recipe(args.collect())?,
        "new" => {
            let path = required(&mut args, "project path")?;
            let name = args.next().unwrap_or_else(|| "Untitled project".into());
            execute(&mut engine, "project.new", json!({"name": name}))?;
            execute(&mut engine, "file.save", json!({"path": path}))?;
            println!("Created {}", path.display());
        }
        "inspect" | "list-media" => {
            let path = required(&mut args, "project path")?;
            execute(&mut engine, "file.open", json!({"path": path}))?;
            let id = if command == "inspect" {
                "project.inspect"
            } else {
                "media.list"
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&execute(&mut engine, id, Value::Null)?).unwrap()
            );
        }
        "add-media" => {
            let project_path = required(&mut args, "project path")?;
            let media_path = required(&mut args, "media path")?;
            execute(&mut engine, "file.open", json!({"path": project_path}))?;
            let name = media_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let kind = media_kind(&media_path);
            execute(
                &mut engine,
                "project.add_media",
                json!({
                    "name": name,
                    "path": media_path,
                    "kind": kind
                }),
            )?;
            execute(&mut engine, "file.save", json!({"path": project_path}))?;
            println!("Added media to {}", project_path.display());
        }
        _ => return usage(),
    }
    Ok(())
}

/// JSON Lines control interface: one CommandRequest per input line, one result per output line.
/// `--project <path>` optionally opens a saved project first; `--actor <human|agent:id>` sets
/// the execution context (defaults to human).
fn run_control(tokens: Vec<String>) -> Result<(), String> {
    let mut engine = EditorEngine::default();
    let mut project_path: Option<String> = None;
    let mut actor_spec: Option<String> = None;
    let mut iter = tokens.into_iter();
    while let Some(tok) = iter.next() {
        match tok.as_str() {
            "--project" => {
                project_path = Some(
                    iter.next()
                        .ok_or("--project requires the path of a saved project")?,
                );
            }
            "--actor" => {
                actor_spec = Some(iter.next().ok_or("--actor requires `human` or `agent:<id>`")?);
            }
            other => return Err(format!("unknown control flag: {other}")),
        }
    }
    if let Some(path) = project_path {
        execute(&mut engine, "file.open", json!({ "path": path }))?;
    }
    let actor_ctx = match actor_spec.as_deref() {
        None | Some("human") => ActorContext::human(),
        Some(spec) if spec.starts_with("agent:") => {
            let agent_id = spec.trim_start_matches("agent:");
            if agent_id.is_empty() {
                return Err("--actor `agent:<id>` requires a non-empty agent identifier".into());
            }
            ActorContext::agent(agent_id, format!("cli-{agent_id}"))
        }
        Some(other) => {
            return Err(format!(
                "invalid --actor `{other}`; expected `human` or `agent:<id>`"
            ))
        }
    };
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("could not read control input: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<CommandRequest>(&line) {
            Ok(request) => match engine.execute_with_context(request, &actor_ctx) {
                Ok(result) => json!({"ok": true, "result": result}),
                Err(error) => json!({"ok": false, "error": error.to_string()}),
            },
            Err(error) => json!({
                "ok": false,
                "error": format!("invalid command request: {error}")
            }),
        };
        println!(
            "{}",
            serde_json::to_string(&response).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}

fn execute(engine: &mut EditorEngine, id: &str, params: Value) -> Result<Value, String> {
    engine
        .execute(CommandRequest::new(id, params))
        .map_err(|e| e.to_string())
}

fn required(args: &mut impl Iterator<Item = String>, label: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {label}"))
}

fn media_kind(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" => "image",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => "audio",
        "mp4" | "mkv" | "mov" | "webm" | "avi" | "m4v" => "video",
        _ => "unknown",
    }
}

/// Generate or verify the checked-in command reference.
///
/// ```text
/// saturn-cli catalogue docs <path> [--check]
/// saturn-cli catalogue json <path> [--check]
/// ```
fn run_catalogue(tokens: Vec<String>) -> Result<(), String> {
    let check = tokens.iter().any(|token| token == "--check");
    let tokens: Vec<String> = tokens.into_iter().filter(|token| token != "--check").collect();
    let (positional, _flags) = parse_flags(tokens);
    let format = positional.first().map(String::as_str).unwrap_or_default();
    let path = positional.get(1).map(PathBuf::from);

    let rendered = match format {
        "docs" | "markdown" => saturn_core::catalogue_markdown(),
        "json" => serde_json::to_string_pretty(&saturn_core::catalogue_json())
            .map_err(|error| error.to_string())?,
        _ => return Err("usage: saturn-cli catalogue <docs|json> <path> [--check]".into()),
    };

    let problems = saturn_core::validate_catalogue();
    if !problems.is_empty() {
        return Err(format!(
            "command catalogue is invalid:\n  {}",
            problems.join("\n  ")
        ));
    }

    let path = path.ok_or("missing catalogue output path")?;
    if check {
        let existing = std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if existing.trim_end() != rendered.trim_end() {
            return Err(format!(
                "{} is out of date; run `saturn-cli catalogue {} {}`",
                path.display(),
                format,
                path.display()
            ));
        }
        println!("{} is up to date", path.display());
        return Ok(());
    }

    std::fs::write(&path, rendered)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    println!("Wrote {}", path.display());
    Ok(())
}


/// Convert a `.sgt` template's visible layers into project graphics. Image and
/// video layers are skipped: their media is owned by the project, not the
/// template file, and importing it here would invent an asset reference.
fn template_graphics(template: &GraphicsTemplate, start_tick: i64) -> Result<Vec<Value>, String> {
    let duration = if template.timeline.duration > 0 {
        template.timeline.duration
    } else {
        5 * saturn_core::TICKS_PER_SECOND
    };
    let mut graphics = Vec::new();
    for (index, layer) in template.layers.iter().filter(|layer| layer.visible).enumerate() {
        let kind = match &layer.source {
            saturn_graphics::LayerSource::Shape { shape_type, .. } => match shape_type {
                saturn_graphics::ShapeType::Ellipse => "ellipse",
                saturn_graphics::ShapeType::Polygon | saturn_graphics::ShapeType::Star => "polygon",
                _ => "rectangle",
            },
            saturn_graphics::LayerSource::Solid { .. } => "rectangle",
            saturn_graphics::LayerSource::Image { .. } | saturn_graphics::LayerSource::Video { .. } => continue,
            _ => "text",
        };
        let fill = match &layer.source {
            saturn_graphics::LayerSource::Solid { color } => Some(json!({"color": color, "opacity": layer.opacity})),
            saturn_graphics::LayerSource::Shape { fill, .. } => Some(json!({"color": fill.color, "opacity": layer.opacity * fill.opacity})),
            saturn_graphics::LayerSource::Text { font, .. } => Some(json!({"color": font.color, "opacity": layer.opacity})),
            _ => None,
        };
        let text = match &layer.source {
            saturn_graphics::LayerSource::Text { content, font } => Some(json!({
                "content": content,
                "font_family": font.family,
                "font_size": font.size,
                "tracking": font.tracking,
                "leading": font.leading,
                "align": match font.alignment {
                    saturn_graphics::TextAlignment::Left => "left",
                    saturn_graphics::TextAlignment::Center => "center",
                    saturn_graphics::TextAlignment::Right => "right",
                    saturn_graphics::TextAlignment::Justify => "justify",
                },
            })),
            _ => None,
        };
        graphics.push(json!({
            "name": layer.name,
            "kind": kind,
            "start_ticks": start_tick,
            "duration_ticks": duration,
            "layer": (index as i32) + 1,
            "visible": true,
            "transform": {
                "x": layer.transform.position[0] as f64,
                "y": layer.transform.position[1] as f64,
                "width": 400.0 * layer.transform.scale[0] as f64,
                "height": 400.0 * layer.transform.scale[1] as f64,
                "rotation": layer.transform.rotation as f64,
                "anchor_x": layer.transform.anchor[0] as f64,
                "anchor_y": layer.transform.anchor[1] as f64,
                "opacity": layer.opacity as f64,
            },
            "fill": fill,
            "text": text,
        }));
    }
    if graphics.is_empty() {
        return Err("template has no supported visible layers to apply".into());
    }
    Ok(graphics)
}

/// Graphics template commands backed by the `.sgt` format in `saturn-graphics`.
fn run_template(tokens: Vec<String>) -> Result<(), String> {
    let (positional, flags) = parse_flags(tokens);
    let action = positional.first().map(String::as_str).unwrap_or_default();
    let path = positional.get(1).map(PathBuf::from);

    match action {
        "create" => {
            let path = path.ok_or("missing template output path")?;
            let name = flags
                .get("name")
                .cloned()
                .ok_or("--name is required to create a template")?;
            let category = flags
                .get("category")
                .map(|value| parse_category(value))
                .transpose()?
                .unwrap_or_default();

            let mut metadata = TemplateMetadata {
                name,
                category,
                description: flags.get("description").cloned().unwrap_or_default(),
                author: flags.get("author").cloned().unwrap_or_default(),
                license: flags
                    .get("license")
                    .cloned()
                    .unwrap_or_else(|| "Proprietary".into()),
                ..TemplateMetadata::default()
            };
            if let Some(tags) = flags.get("tags") {
                metadata.tags = tags
                    .split(',')
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .map(str::to_string)
                    .collect();
            }

            let mut template = GraphicsTemplate::new(metadata);
            // A template with no layers cannot be applied to anything. Seed one
            // editable text layer so create -> apply is a real round trip.
            let layer: saturn_graphics::TemplateLayer = serde_json::from_value(json!({
                "id": "00000000-0000-0000-0000-000000000001",
                "name": "Title",
                "layer_type": "Text",
                "visible": true,
                "locked": false,
                "opacity": 1.0,
                "blend_mode": "Normal",
                "transform": {
                    "position": [100.0, 100.0], "scale": [1.0, 1.0],
                    "rotation": 0.0, "anchor": [0.0, 0.0],
                    "flip_x": false, "flip_y": false
                },
                "effects": [],
                "source": {
                    "Text": {
                        "content": "Title",
                        "font": {
                            "family": "Inter", "size": 48.0, "weight": "normal",
                            "style": "normal", "color": "#ffffff",
                            "alignment": "Left", "tracking": 0.0, "leading": 1.0
                        }
                    }
                },
                "mask": null
            })).map_err(|error| format!("could not build default template layer: {error}"))?;
            template.add_layer(layer);
            save_template(&template, &path)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
            print_json(&json!({
                "created": true,
                "path": path,
                "template": template.metadata,
            }))
        }
        "inspect" => {
            let path = path.ok_or("missing template path")?;
            let template = load_template(&path)
                .map_err(|error| format!("could not read {}: {error}", path.display()))?;
            print_json(&serde_json::to_value(&template).map_err(|error| error.to_string())?)
        }
        "list" => {
            let directory = path.unwrap_or_else(|| PathBuf::from("."));
            let mut entries: Vec<PathBuf> = std::fs::read_dir(&directory)
                .map_err(|error| format!("could not read {}: {error}", directory.display()))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("sgt"))
                })
                .collect();
            entries.sort();

            let mut templates = Vec::new();
            for path in entries {
                match load_template(&path) {
                    Ok(template) => templates.push(json!({
                        "file": path,
                        "name": template.metadata.name,
                        "category": template.metadata.category,
                        "version": template.metadata.version,
                        "description": template.metadata.description,
                        "author": template.metadata.author,
                        "tags": template.metadata.tags,
                        "duration_ticks": template.timeline.duration,
                        "parameters": template.parameters.len(),
                        "layers": template.layers.len(),
                    })),
                    Err(error) => templates.push(json!({
                        "file": path,
                        "error": error.to_string(),
                    })),
                }
            }
            print_json(&Value::Array(templates))
        }
        "export" => {
            let path = path.ok_or("missing template path")?;
            let output = flags
                .get("output")
                .map(PathBuf::from)
                .ok_or("--output is required to export a template")?;
            let template = load_template(&path)
                .map_err(|error| format!("could not read {}: {error}", path.display()))?;
            save_template(&template, &output)
                .map_err(|error| format!("could not write {}: {error}", output.display()))?;
            print_json(&json!({
                "exported": true,
                "source": path,
                "output": output,
                "name": template.metadata.name,
            }))
        }
        "apply" => {
            let template_path = path.ok_or("missing template path")?;
            let project_path = flags
                .get("project")
                .map(PathBuf::from)
                .ok_or("--project is required to apply a template to a saved project")?;
            let start_tick: i64 = flags
                .get("tick")
                .map(|value| value.parse::<i64>())
                .transpose()
                .map_err(|error| format!("--tick must be an integer: {error}"))?
                .unwrap_or(0);
            let template = load_template(&template_path)
                .map_err(|error| format!("could not read {}: {error}", template_path.display()))?;
            let graphics = template_graphics(&template, start_tick)?;

            let mut engine = EditorEngine::default();
            execute(&mut engine, "file.open", json!({ "path": project_path }))?;
            let mut template_ids = Vec::new();
            for mut body in graphics {
                let template_id = engine.document().project.next_id;
                let name = body["name"].clone();
                body["id"] = json!(template_id);
                engine.document_mut().project.next_id += 1;
                engine.document_mut().project.graphic_templates.push(
                    serde_json::from_value(json!({
                        "id": template_id,
                        "name": name,
                        "body": body,
                    }))
                    .map_err(|error| format!("could not build template graphic: {error}"))?,
                );
                template_ids.push(template_id);
            }
            let mut applied = Vec::new();
            for template_id in template_ids {
                let result = execute(
                    &mut engine,
                    "graphic.template_apply",
                    json!({"template_id": template_id, "start_ticks": start_tick}),
                )?;
                applied.push(result["id"].clone());
            }
            execute(&mut engine, "file.save", json!({ "path": project_path }))?;
            print_json(&json!({
                "applied": true,
                "template": template.metadata.name,
                "project": project_path,
                "start_ticks": start_tick,
                "graphic_ids": applied,
            }))
        }
        _ => Err("usage: saturn-cli template create <path> --name <name> | inspect <path> | list [dir] | export <path> --output <path> | apply <path> --project <project.saturn> [--tick <ticks>]".into()),
    }
}

fn parse_flags(tokens: Vec<String>) -> (Vec<String>, HashMap<String, String>) {
    let mut positional = Vec::new();
    let mut flags = HashMap::new();
    let mut iter = tokens.into_iter();
    while let Some(token) = iter.next() {
        if let Some(name) = token.strip_prefix("--") {
            if let Some((key, value)) = name.split_once('=') {
                flags.insert(key.to_string(), value.to_string());
            } else if let Some(value) = iter.next() {
                flags.insert(name.to_string(), value);
            }
        } else {
            positional.push(token);
        }
    }
    (positional, flags)
}

fn parse_category(value: &str) -> Result<TemplateCategory, String> {
    match value.to_ascii_lowercase().as_str() {
        "lower-third" | "lowerthird" => Ok(TemplateCategory::LowerThird),
        "title" => Ok(TemplateCategory::Title),
        "transition" => Ok(TemplateCategory::Transition),
        "background" => Ok(TemplateCategory::Background),
        "overlay" => Ok(TemplateCategory::Overlay),
        "bug" => Ok(TemplateCategory::Bug),
        "full-screen" | "fullscreen" => Ok(TemplateCategory::FullScreen),
        "custom" => Ok(TemplateCategory::Custom),
        other => Err(format!("unknown template category: {other}")),
    }
}

fn print_json(value: &Value) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn usage<T>() -> Result<T, String> {
    Err("usage: saturn-cli commands [filter] [--json] | describe <id> | catalogue <docs|json> <path> [--check] | control [--project <project.saturn>] [--actor human|agent:<id>] | new <project.saturn> [name] | inspect <project.saturn> | list-media <project.saturn> | add-media <project.saturn> <media-path> | render <project.saturn> <output.mp4|output.webm> | template create|inspect|list|export | cet-gen ... | cet-validate <file> | cet-propose <file> | cet-query <file> | analyze-recipe <podcast|webinar> <input.json>".into())
}
