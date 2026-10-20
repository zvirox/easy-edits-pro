use saturn_core::{CommandRequest, EditorEngine, command_catalogue};
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
            for spec in command_catalogue() {
                println!(
                    "{}\t{}\t{}",
                    spec.id,
                    spec.label,
                    spec.shortcut.unwrap_or("")
                );
            }
        }
        "control" => run_control()?,
        "template" => run_template(args.collect())?,
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
fn run_control() -> Result<(), String> {
    let stdin = io::stdin();
    let mut engine = EditorEngine::default();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("could not read control input: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<CommandRequest>(&line) {
            Ok(request) => match engine.execute(request) {
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

            let template = GraphicsTemplate::new(metadata);
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
        "apply" => Err(
            "template apply is not supported: graphics templates cannot be applied to a sequence yet"
                .into(),
        ),
        _ => Err("usage: saturn-cli template create <path> --name <name> | inspect <path> | list [dir] | export <path> --output <path>".into()),
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
    Err("usage: saturn-cli commands | control | new <project.saturn> [name] | inspect <project.saturn> | list-media <project.saturn> | add-media <project.saturn> <media-path> | template create|inspect|list|export".into())
}
