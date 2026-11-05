// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";

const sourceRoots = ["src", "ui/src", "../../crates/saturn-core/src"];
const extensions = new Set([".rs", ".ts", ".tsx", ".css"]);
// Explicitly tracked pre-existing files. New source must stay at or below the limit.
const extractionBacklog = new Set([
  "ui/src/workspaces/FairlightWorkspace.tsx",
  "ui/src/workspaces/KeyframesWorkspace.tsx",
  "../../crates/saturn-core/src/interop.rs",
]);
// Default hard limit is 800 lines; can be overridden via --max-lines=N or CHECK_SOURCE_SIZE_MAX_LINES env
const hardLimit = Math.max(1, Number(process.env.CHECK_SOURCE_SIZE_MAX_LINES ?? process.argv.find(a => a.startsWith("--max-lines="))?.split("=")[1] ?? 800));
const oversized = [];

function visit(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      if (!["node_modules", "dist", "target"].includes(entry.name)) visit(path);
      continue;
    }
    const extension = path.slice(path.lastIndexOf("."));
    if (!extensions.has(extension)) continue;
    const lines = countSourceLines(readFileSync(path, "utf8"));
    if (lines > hardLimit) oversized.push({ path: relative(process.cwd(), path), lines });
  }
}

// The SPDX licence header (and any leading prose banner) is boilerplate added
// by the licence gate, not source: strip the leading comment block so it never
// counts against the size limit.
function countSourceLines(text) {
  const lines = text.split("\n");
  let start = 0;
  while (start < lines.length) {
    const line = lines[start].trim();
    const isBoilerplate =
      line === "" ||
      line.startsWith("#!") ||
      line.startsWith("//") ||
      line.startsWith("/*") ||
      line.startsWith("*") ||
      line.startsWith("*/");
    if (!isBoilerplate) break;
    start += 1;
  }
  return lines.length - start;
}

for (const root of sourceRoots) visit(root);
const untrackedOversized = oversized.filter(({ path }) => !extractionBacklog.has(path));
if (untrackedOversized.length) {
  console.error(`New source files exceed the ${hardLimit}-line limit:`);
  for (const file of untrackedOversized) console.error(`  ${file.path}: ${file.lines} lines`);
  process.exitCode = 1;
} else {
  console.log(`Source size check passed (${hardLimit}-line limit).`);
}
if (oversized.length) {
  console.warn("Tracked extraction backlog:");
  for (const file of oversized) console.warn(`  ${file.path}: ${file.lines} lines`);
}
