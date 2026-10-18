import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";

const sourceRoots = ["src", "ui/src", "../../crates/saturn-core/src"];
const extensions = new Set([".rs", ".ts", ".tsx", ".css"]);
const knownLargeFiles = new Set([
  "ui/src/App.tsx",
  "ui/src/engine/playbackEngine.ts",
  "../../crates/saturn-core/src/engine.rs",
  "../../crates/saturn-core/src/project.rs",
]);
const hardLimit = 1_000;
const largeFiles = [];

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
    if (lines > hardLimit) largeFiles.push({ path: relative(process.cwd(), path), lines });
  }
}

for (const root of sourceRoots) visit(root);

const unexpected = largeFiles.filter(({ path }) => !knownLargeFiles.has(path));
if (unexpected.length) {
  console.error(`Source files exceed the ${hardLimit}-line limit:`);
  for (const file of unexpected) console.error(`  ${file.path}: ${file.lines} lines`);
  process.exitCode = 1;
} else {
  console.log(`Source size check passed (${hardLimit}-line hard limit).`);
  for (const file of largeFiles) console.log(`  Existing extraction backlog: ${file.path} (${file.lines} lines)`);
}
