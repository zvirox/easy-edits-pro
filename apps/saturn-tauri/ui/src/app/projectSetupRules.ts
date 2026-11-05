// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/** Pure welcome-project creation rules, extracted from useProjectOperations
 * to keep that hook under the 800-line source cap (same extraction precedent
 * as enqueueExport.ts). Node-testable: no Tauri imports. */

export function assertProjectSetup(
  name: string,
  sequenceName: string,
  width: number,
  height: number,
): void {
  if (
    !name ||
    !sequenceName ||
    !Number.isInteger(width) ||
    !Number.isInteger(height) ||
    width <= 0 ||
    height <= 0 ||
    width > 16_384 ||
    height > 16_384
  ) {
    throw new Error("Enter project and timeline names and dimensions between 1 and 16,384 pixels.");
  }
}
