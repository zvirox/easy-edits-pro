//! Static command catalogue.
//!
//! This is the single source of truth for menus, shortcuts, the command
//! palette, `saturn-cli describe`, the MCP tool list, and the generated
//! reference in `docs/COMMANDS.md`. Command metadata lives next to the engine
//! that implements it, and is split per menu to keep files reviewable.

use serde::Serialize;
use serde_json::Value;

/// Top-level menus, in bar order. `CommandSpec::menu[0]` must be one of these.
pub const MENU_BAR: &[&str] = &[
    "File", "Edit", "Clip", "Sequence", "Markers", "Graphics", "Audio", "Effects", "View",
    "Window", "Help",
];

/// Coarse grouping used by the command reference, palette badges, and docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Category {
    File,
    Edit,
    Clip,
    Sequence,
    Markers,
    Graphics,
    Audio,
    View,
    Window,
    Help,
    Color,
    Effects,
    Animation,
    MultiCam,
    Transcription,
    Other,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Edit => "Edit",
            Self::Clip => "Clip",
            Self::Sequence => "Sequence",
            Self::Markers => "Markers",
            Self::Graphics => "Graphics",
            Self::Audio => "Audio",
            Self::View => "View",
            Self::Window => "Window",
            Self::Help => "Help",
            Self::Color => "Color",
            Self::Effects => "Effects",
            Self::Animation => "Animation",
            Self::MultiCam => "Multi-Cam",
            Self::Transcription => "Transcription",
            Self::Other => "Other",
        }
    }
}

/// How a mutating command participates in undo history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UndoGroup {
    /// The command does not change the project.
    None,
    /// One history entry per execution.
    Transaction,
    /// Coalesced while a gesture is open; one history entry per gesture.
    Gesture,
}

/// Static metadata for one engine command.
///
/// `params` is a compact documentation schema, not a validator: keys map to
/// `str`, `int`, `num`, `bool`, `ticks`, `path`, `obj`, or `[T]`, and a trailing
/// `?` means the field is optional (serde default or `Option<T>`).
#[derive(Clone, Copy, Debug, Serialize)]
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub menu: &'static [&'static str],
    pub shortcut: Option<&'static str>,
    pub params: &'static str,
    pub category: Category,
    pub mutating: bool,
    pub undo_group: UndoGroup,
    pub aliases: &'static [&'static str],
    pub since: &'static str,
    /// Wired to a working UI affordance, so menus, the palette, and shortcuts
    /// may surface it. Planned commands stay in the catalogue (CLI, MCP, docs)
    /// but are hidden until their phase lands.
    pub shipped: bool,
    /// Frontend-only command. The engine rejects it with `DispatchError::UiOnly`.
    pub ui_only: bool,
}

impl CommandSpec {
    /// Full menu path joined with `" > "`, or an empty string when unlisted.
    pub fn menu_path(&self) -> String {
        self.menu.join(" > ")
    }

    /// Case-insensitive match against id, label, aliases, and menu path.
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        self.id.to_lowercase().contains(&query)
            || self.label.to_lowercase().contains(&query)
            || self.menu_path().to_lowercase().contains(&query)
            || self
                .aliases
                .iter()
                .any(|alias| alias.to_lowercase().contains(&query))
    }
}

mod commands;

static COMMANDS: std::sync::LazyLock<Vec<CommandSpec>> = std::sync::LazyLock::new(|| {
    let mut all = Vec::new();
    all.extend_from_slice(commands::audio::COMMANDS);
    all.extend_from_slice(commands::clip::COMMANDS);
    all.extend_from_slice(commands::edit::COMMANDS);
    all.extend_from_slice(commands::effects::COMMANDS);
    all.extend_from_slice(commands::engine::COMMANDS);
    all.extend_from_slice(commands::file::COMMANDS);
    all.extend_from_slice(commands::file_flows::COMMANDS);
    all.extend_from_slice(commands::graphics::COMMANDS);
    all.extend_from_slice(commands::help::COMMANDS);
    all.extend_from_slice(commands::markers::COMMANDS);
    all.extend_from_slice(commands::playhead::COMMANDS);
    all.extend_from_slice(commands::project_freeform::COMMANDS);
    all.extend_from_slice(commands::project_panel::COMMANDS);
    all.extend_from_slice(commands::sequence::COMMANDS);
    all.extend_from_slice(commands::settings::COMMANDS);
    all.extend_from_slice(commands::timeline::COMMANDS);
    all.extend_from_slice(commands::trim::COMMANDS);
    all.extend_from_slice(commands::view::COMMANDS);
    all.extend_from_slice(commands::window::COMMANDS);
    all
});

pub fn command_catalogue() -> &'static [CommandSpec] {
    COMMANDS.as_slice()
}

pub fn find_command(id: &str) -> Option<&'static CommandSpec> {
    command_catalogue().iter().find(|command| command.id == id)
}

/// Commands listed under a top-level menu, in catalogue order.
pub fn menu_commands(menu: &str) -> Vec<&'static CommandSpec> {
    command_catalogue()
        .iter()
        .filter(|command| command.menu.first().copied() == Some(menu))
        .collect()
}

/// Fuzzy-ish search over id, label, aliases, and menu path.
pub fn search_commands(query: &str) -> Vec<&'static CommandSpec> {
    let query = query.trim();
    let mut matches: Vec<&'static CommandSpec> = command_catalogue()
        .iter()
        .filter(|command| command.matches(query))
        .collect();
    // Prefer label-prefix matches, then shorter labels.
    matches.sort_by_key(|command| {
        let label = command.label.to_lowercase();
        let q = query.to_lowercase();
        (!label.starts_with(&q), label.len(), command.id.len())
    });
    matches
}

/// Structural checks that must hold before the catalogue ships.
pub fn validate_catalogue() -> Vec<String> {
    let mut errors = Vec::new();
    let mut ids = std::collections::HashSet::new();
    let mut shortcuts = std::collections::HashMap::<String, &'static str>::new();
    for command in command_catalogue() {
        if !ids.insert(command.id) {
            errors.push(format!("duplicate command id: {}", command.id));
        }
        if command.label.trim().is_empty() {
            errors.push(format!("empty label: {}", command.id));
        }
        if let Some(menu) = command.menu.first() {
            if !MENU_BAR.contains(menu) {
                errors.push(format!(
                    "command {} uses unknown top-level menu {:?}",
                    command.id, menu
                ));
            }
        }
        if command.shipped && command.menu.is_empty() {
            errors.push(format!(
                "shipped command {} is unreachable: it has no menu",
                command.id
            ));
        }
        if command.ui_only && command.menu.is_empty() {
            errors.push(format!(
                "UI-only command {} must declare a menu",
                command.id
            ));
        }
        if let Some(shortcut) = command.shortcut {
            let key = shortcut.replace("Cmd+", "Ctrl+").to_lowercase();
            if let Some(previous) = shortcuts.insert(key, command.id) {
                errors.push(format!(
                    "shortcut {} is used by both {} and {}",
                    shortcut, previous, command.id
                ));
            }
        }
    }
    errors
}

/// Machine-readable catalogue for `saturn-cli commands --json`.
pub fn catalogue_json() -> Value {
    Value::Array(
        command_catalogue()
            .iter()
            .map(|command| {
                serde_json::json!({
                    "id": command.id,
                    "label": command.label,
                    "menu": command.menu,
                    "shortcut": command.shortcut,
                    "params": command.params,
                    "category": command.category.label(),
                    "mutating": command.mutating,
                    "undoGroup": command.undo_group,
                    "aliases": command.aliases,
                    "since": command.since,
                    "shipped": command.shipped,
                    "uiOnly": command.ui_only,
                })
            })
            .collect(),
    )
}

/// Markdown reference grouped by top-level menu.
pub fn catalogue_markdown() -> String {
    let mut out = String::from(
        "# Command reference\n\nGenerated from the command catalogue. Do not edit by hand.\n\n",
    );
    out.push_str(&format!("{} commands.\n", command_catalogue().len()));
    for menu in MENU_BAR {
        let items = menu_commands(menu);
        if items.is_empty() {
            continue;
        }
        out.push_str(&format!("\n## {menu}\n\n"));
        out.push_str("| Command | ID | Status | Shortcut | Params |\n|---|---|---|---|---|\n");
        for command in items {
            let shortcut = command.shortcut.unwrap_or("");
            let params = command.params.replace('|', "\\|");
            let status = if command.shipped {
                "shipped"
            } else {
                "planned"
            };
            out.push_str(&format!(
                "| {} | `{}` | {} | {} | `{}` |\n",
                command.label, command.id, status, shortcut, params
            ));
        }
    }
    out
}
