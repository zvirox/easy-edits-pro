// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Command catalogue specifications for delivery packaging & integrity checks (TS5).

use crate::catalogue::Tier;
use crate::command::{Category, CommandSpec, UndoGroup};

pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: "delivery.package",
        label: "Package Delivery Bundle…",
        menu: &["File", "Delivery"],
        shortcut: None,
        params: "{\"path\":str,\"exports\":[str]?,\"interchange\":[str]?,\"mark_delivered\":bool?,\"require_approved\":bool?,\"require_rendered_exports\":bool?,\"verify\":{\"preset\":str?}?}",
        category: Category::File,
        mutating: true,
        undo_group: UndoGroup::Transaction,
        aliases: &["package delivery", "delivery bundle", "handoff package"],
        since: "0.5.0",
        shipped: true,
        ui_only: false,
        tier: Tier::Filesystem,
        tiers: &[Tier::Edit, Tier::Filesystem],
        profiles: &["admin", "tasksheet", "export"],
    },
    CommandSpec {
        id: "delivery.verify",
        label: "Verify Delivery Package…",
        menu: &["File", "Delivery"],
        shortcut: None,
        params: "{\"path\":str}",
        category: Category::File,
        mutating: false,
        undo_group: UndoGroup::None,
        aliases: &["verify delivery", "check delivery checksums"],
        since: "0.5.0",
        shipped: true,
        ui_only: false,
        tier: Tier::Filesystem,
        tiers: &[Tier::Read, Tier::Filesystem],
        profiles: &["admin", "tasksheet", "export", "review"],
    },
];
