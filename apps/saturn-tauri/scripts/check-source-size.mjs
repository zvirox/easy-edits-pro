import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";

const sourceRoots = ["src", "ui/src", "../../crates/saturn-core/src"];
const extensions = new Set([".rs", ".ts", ".tsx", ".css"]);
// Explicitly tracked pre-existing files. New source must stay at or below the limit.
const extractionBacklog = new Set([
  "ui/src/App.tsx",
  "ui/src/workspaces/FairlightWorkspace.tsx",
  "ui/src/workspaces/KeyframesWorkspace.tsx",
  "../../crates/saturn-core/src/interop.rs",
]);
const hardLimit = 800;
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
    const lines = readFileSync(path, "utf8").split("\n").length;
    if (lines > hardLimit) oversized.push({ path: relative(process.cwd(), path), lines });
  }
}

for (const root of sourceRoots) visit(root);
const untrackedOversized = oversized.filter(({ path }) => !extractionBacklog.has(path));
if (untrackedOversized.length) {
  console.error(`New source files exceed the ${hardLimit}-line limit:`);
  for (const file of untrackedOversized) console.error(`  ${file.path}: ${file.lines} lines`);
  process.exitCode = 1;
} else {
  console.log(`Source size check passed (${hardLimit}-line limit for new files).`);
}
if (oversized.length) {
  console.warn("Tracked extraction backlog:");
  for (const file of oversized) console.warn(`  ${file.path}: ${file.lines} lines`);
}
