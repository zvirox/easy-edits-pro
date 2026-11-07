// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use crate::catalogue::{Category, CommandSpec, UndoGroup};

pub(crate) const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: "ocio.set_pipeline",
        label: "Set Color Pipeline",
        menu: &["Color", "Pipeline"],
        shortcut: None,
        params: "{\"working_space\":str,\"display_odt\":str,\"default_idt\":str?}",
        category: Category::Color,
        mutating: true,
        undo_group: UndoGroup::Transaction,
        aliases: &[],
        since: "0.4.0",
        shipped: true,
        ui_only: false,
    },
    CommandSpec {
        id: "ocio.set_media_idt",
        label: "Set Media Input Transform",
        menu: &["Color", "Pipeline"],
        shortcut: None,
        params: "{\"media_id\":int,\"idt\":str}",
        category: Category::Color,
        mutating: true,
        undo_group: UndoGroup::Transaction,
        aliases: &[],
        since: "0.4.0",
        shipped: true,
        ui_only: false,
    },
];
