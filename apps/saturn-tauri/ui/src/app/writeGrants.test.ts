// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

// E.4 seam, security batch (planning.md §6f): outside-roots write targets are
// rejected at the shell unless they carry a session grant, and the grant is
// minted exactly where the user picks the destination in a native dialog.
// The flows are orchestration over Tauri APIs (not node-importable logic), so
// like the other wiring guards this watches the wiring while the e2e/native
// suites watch behavior.

const read = (file: string) => readFileSync(new URL(file, import.meta.url), "utf8");

/** (source file, grantWrite call, driven command literal) pairs. */
const wirings: Array<[string, string, string]> = [
  ["./useFileProjectFlows.ts", 'await grantWrite(path);', '"file.save"'],
  ["./useFileProjectFlows.ts", "await grantWrite(target);", 'invoke("export_frame"'],
  ["./useFileProjectFlows.ts", "await grantWrite(target);", "{ path: target }, `Exported ${format.label}`"],
  ["./useProjectOperations.ts", "await grantWrite(path);", '"file.save", { path }, "Project saved"'],
  ["./useProjectOperations.ts", "await grantWrite(path);", 'invoke("start_export"'],
  ["../features/preview/PreviewSection.tsx", "await grantWrite(target);", 'invoke("export_frame"'],
  ["./useCaptions.ts", "await grantWrite(selected);", 'invoke("export_subtitles_file"'],
  ["./useMarkers.ts", "await grantWrite(path);", 'invoke("write_markers_file"'],
  ["./useInspectorOperations.ts", "await grantWrite(selected);", 'invoke("export_subtitles_file"'],
  ["./shortcutMachineCommands.ts", "await grantWrite(path);", "exportShortcuts(path)"],
  [
    "../features/shortcuts/ShortcutsDialog.tsx",
    "grantWrite(path)",
    "engineOps.exportShortcuts(path)",
  ],
  [
    "../features/deliver/DeliverPresetBank.tsx",
    "grantWrite(path)",
    '"export.presets.export"',
  ],
];

test("every dialog-driven write flow grants its destination before the driving command", () => {
  const byFile = new Map<string, string>();
  for (const [file] of wirings) {
    assert.ok(byFile.has(file) || file.includes("/"), file);
    if (!byFile.has(file)) byFile.set(file, read(file));
  }
  for (const [file, grant, command] of wirings) {
    const source = byFile.get(file);
    assert.ok(source, file);
    assert.ok(
      source.includes('from "./writeGrants.ts"') ||
        source.includes('from "../../app/writeGrants.ts"'),
      `${file} must import grantWrite from writeGrants`,
    );    assert.ok(source.includes(grant), `${file}: grant call ${grant}`);
    const grantIndex = source.indexOf(grant);
    const commandIndex = source.indexOf(command);
    assert.ok(commandIndex >= 0, `${file}: driven command ${command}`);
    assert.ok(
      grantIndex < commandIndex,
      `${file}: grantWrite must run before the driving command (${grant} vs ${command})`,
    );
  }
});

test("the welcome first-run save grants after the project lifecycle command", () => {
  // v0.1.2-alpha.1 regression: the welcome new-project flow minted its
  // dialog grant BEFORE `project.new`, which clears session write grants
  // (project switch). The grant was gone by `file.save`, so the very first
  // save on a fresh install was rejected as outside the allowed roots.
  const source = read("./useProjectOperations.ts");
  const create = source.slice(
    source.indexOf("const createProjectFromWelcome"),
    source.indexOf("const recoverProjectFromWelcome"),
  );
  assert.ok(create.length > 0, "createProjectFromWelcome slice");
  const grantIndex = create.indexOf("await grantWrite(path);");
  const newCommand = create.indexOf('runCommand("project.new"');
  const saveCommand = create.indexOf('runCommand("file.save", { path })');
  assert.ok(newCommand >= 0, "welcome flow drives project.new");
  assert.ok(saveCommand > newCommand, "welcome flow saves after creating");
  assert.ok(
    grantIndex > newCommand,
    "the dialog grant must be minted AFTER project.new (it clears grants)",
  );
  assert.ok(grantIndex < saveCommand, "the grant must precede the file.save it authorizes");
});

test("the enqueue path re-uses dialog-authorized destinations", () => {
  const source = read("./useProjectOperations.ts");
  // chooseExportDestination is the single funnel for both start_export and
  // queue_export (enqueueExport.ts); its grant must sit next to the pick.
  const chooser = source.slice(source.indexOf("const chooseExportDestination"));
  assert.ok(chooser.includes("await grantWrite(path)"), "the export destination picker grants");
  assert.ok(
    chooser.indexOf("await grantWrite(path)") < chooser.indexOf("setExportOutputPath(path)"),
    "the grant is recorded before the destination is remembered",
  );
});

test("the engine write-command table stays covered (parity with the Rust surface)", () => {
  const rust = readFileSync(
    new URL("../../../src/write_grants.rs", import.meta.url),
    "utf8",
  );
  for (const id of [
    "file.save",
    "interop.export_edl",
    "interop.export_fcpxml",
    "interop.export_xmeml_xml",
    "interop.export_otio",
    "shortcuts.export",
    "export.presets.export",
  ]) {
    assert.ok(
      rust.includes(`"${id}"`),
      `saturn-tauri write_grants.rs must still gate ${id.replace(/"/g, "")}`,
    );
  }
});
