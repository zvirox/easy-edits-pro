// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { invoke } from "@tauri-apps/api/core";

/**
 * Security batch (planning.md §6f, audit priority item 5): the shell rejects
 * every engine- or wrapper-dispatched write target outside the allowed roots
 * (app data, app cache, active project directory) unless it carries a session
 * write grant — because an MCP agent or injected document text can otherwise
 * write any file as the user.
 *
 * The native Save/Export dialog pick is the consent act, so every dialog-
 * driven flow forwards its destination here BEFORE the driving command runs.
 * Grants are session-scoped, exact-path, and never creatable from the MCP
 * bridge; they clear on project switch. Headless harnesses bypass via
 * SATURN_ALLOW_ANY_WRITE_TARGETS instead of this command.
 */
export async function grantWrite(path: string): Promise<void> {
  try {
    await invoke("register_write_grant", { path });
  } catch (error) {
    // A failed grant is not fatal here: the driving command re-checks the
    // policy at the shell and fails loudly on its own with the named error.
    console.error("register_write_grant failed:", error);
  }
}
