// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
use crate::catalogue::{Category, CommandSpec, Tier, UndoGroup};

pub(crate) const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: "dsp.load_plugin",
        label: "Load Track Plugin",
        menu: &["Audio", "DSP Plugins"],
        shortcut: None,
        params: "{\"track_id\":int,\"format\":str,\"path\":str,\"plugin_id\":str,\"enabled\":bool?,\"params\":obj?}",
        category: Category::Sequence,
        mutating: true,
        undo_group: UndoGroup::Transaction,
        aliases: &[],
        since: "0.4.0",
        shipped: true,
        ui_only: false,
        tier: Tier::Edit, tiers: &[Tier::Edit, Tier::Filesystem],
        profiles: &["admin", "audio"],
    },
    CommandSpec {
        id: "dsp.set_plugin_param",
        label: "Set Track Plugin Parameter",
        menu: &["Audio", "DSP Plugins"],
        shortcut: None,
        params: "{\"track_id\":int,\"slot_index\":int,\"param_id\":int,\"value\":num}",
        category: Category::Sequence,
        mutating: true,
        undo_group: UndoGroup::Transaction,
        aliases: &[],
        since: "0.4.0",
        shipped: true,
        ui_only: false,
        tier: Tier::Edit, tiers: &[Tier::Edit, Tier::Filesystem],
        profiles: &["admin", "audio"],
    },
];
