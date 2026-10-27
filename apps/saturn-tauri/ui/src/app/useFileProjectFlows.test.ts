import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

// E.4 seam: file.revert must route through the shared open-project flow
// (loadProjectFromPath), not a bare file.open. Reverting replaces the whole
// document, so editor-held selection and the playhead have to reset exactly
// the way File > Open resets them. The flows module is orchestration over
// Tauri APIs (not node-importable logic), so the guard watches the wiring
// the same way the e2e suite watches behavior.

const flowsSource = readFileSync(new URL("./useFileProjectFlows.ts", import.meta.url), "utf8");
const operationsSource = readFileSync(new URL("./useProjectOperations.ts", import.meta.url), "utf8");

test("file.revert routes through loadProjectFromPath", () => {
  const revertCase = flowsSource.slice(flowsSource.indexOf('case "file.revert"'));
  assert.ok(revertCase.length > 0, "the file.revert case exists");
  const caseEnd = revertCase.indexOf('case "', 10);
  const body = caseEnd > 0 ? revertCase.slice(0, caseEnd) : revertCase;
  assert.match(body, /await loadProjectFromPath\(projectPath\)/);
  assert.doesNotMatch(body, /"file\.open"/);
});

test("createFileProjectFlows receives the shared loader", () => {
  assert.match(
    operationsSource,
    /loadProjectFromPath,\s*\n\s*\}\);/,
    "the options object must pass loadProjectFromPath to createFileProjectFlows",
  );
  assert.match(flowsSource, /loadProjectFromPath: \(path: string\) => Promise<void>/);
});
