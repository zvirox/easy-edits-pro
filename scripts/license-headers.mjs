#!/usr/bin/env node
/**
 * SPDX header tool for the dual-licence plan
 * (docs/planning/dual-licensing-plan.md §5, §7).
 *
 *   node scripts/license-headers.mjs apply   # insert missing headers
 *   node scripts/license-headers.mjs check   # CI: verify every file has one
 *
 * Every first-party source file carries the AGPL-3.0-or-later SPDX id plus a
 * pointer to the commercial licence option (see LICENSING.md). The script is
 * idempotent: files already containing an SPDX id are left alone.
 */
import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const BYTES_LIMIT = 512;

const EXT = {
  ".rs": "line",
  ".ts": "line",
  ".tsx": "line",
  ".mjs": "line",
  ".js": "line",
  ".py": "hash",
  ".sh": "hash",
  ".css": "block",
};

const HEADER_LINES = [
  "SPDX-License-Identifier: AGPL-3.0-or-later",
  "Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors",
  "Also available under the EEP commercial licence — see LICENSING.md",
];

const SKIP_DIRS = new Set([
  "node_modules",
  "target",
  "debug",
  "dist",
  "gen",
  "test-results",
  ".git",
  ".github",
  ".prowl",
  ".claude",
  ".codex",
  ".agents",
  ".opencode",
  "licenses",
  "assets",
  "docs",
]);

const SKIP_FILES = new Set(["pnpm-lock.yaml", "Cargo.lock"]);

function isSourceFile(name) {
  return Object.keys(EXT).some((ext) => name.endsWith(ext));
}

function wrap(path, header) {
  if (path.endsWith(".css")) {
    return `/*\n${header
      .split("\n")
      .map((line) => ` * ${line}`)
      .join("\n")}\n */\n`;
  }
  const style = EXT[Object.keys(EXT).find((ext) => path.endsWith(ext))];
  const marker = style === "hash" ? "# " : "// ";
  return `${header
    .split("\n")
    .map((line) => `${marker}${line}`)
    .join("\n")}\n`;
}

function walk(dir, files) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return;
  }
  for (const entry of entries) {
    if (SKIP_DIRS.has(entry) || SKIP_FILES.has(entry)) continue;
    const path = join(dir, entry);
    let stat;
    try {
      stat = statSync(path);
    } catch {
      continue;
    }
    if (stat.isDirectory()) {
      walk(path, files);
    } else if (stat.isFile() && isSourceFile(entry)) {
      files.push(path);
    }
  }
}

const files = [];
walk(ROOT, files);

const missing = [];
const flagged = [];

for (const path of files) {
  const text = readFileSync(path, "utf8");
  if (text.includes("SPDX-License-Identifier")) continue;
  // Vendored/third-party files keep to their own licence terms (NOTICE); our
  // header tool refuses to stamp them silently.
  if (/third[- ]party|upstream|(c) Copyright/i.test(text.slice(0, BYTES_LIMIT))) {
    flagged.push(path);
    continue;
  }
  missing.push(path);
}

function headerFor(path) {
  const text = readFileSync(path, "utf8");
  const comment = wrap(path, HEADER_LINES.join("\n"));
  if (text.startsWith("#!")) {
    const [shebang, ...rest] = text.split("\n");
    return [shebang, "", comment.trimEnd(), rest.join("\n")].join("\n");
  }
  return comment + text;
}

const mode = process.argv[2] ?? "check";

if (mode === "apply") {
  for (const path of missing) {
    writeFileSync(path, headerFor(path));
  }
  for (const path of flagged) {
    console.error(`SKIPPED (looks third-party): ${relative(ROOT, path)}`);
  }
  console.log(
    `license-headers: applied to ${missing.length} file(s), ${files.length - missing.length} already had SPDX ids`,
  );
} else {
  for (const path of flagged) {
    console.error(`SKIPPED (looks third-party): ${relative(ROOT, path)}`);
  }
  if (missing.length > 0) {
    console.error(
      `license-headers: ${missing.length} source file(s) missing SPDX headers:\n` +
        missing.map((path) => `  ${relative(ROOT, path)}`).join("\n"),
    );
    console.error("Run `node scripts/license-headers.mjs apply` first.");
    process.exit(1);
  }
  console.log(
    `license-headers: OK (${files.length} source files carry SPDX headers)`,
  );
}
