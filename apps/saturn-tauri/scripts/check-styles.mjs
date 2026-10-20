#!/usr/bin/env node
/**
 * Phase 1 style gate.
 *
 * - No `transition: all` (layout/unknown-property animation is banned).
 * - No CSS file over 1,000 lines.
 * - Raw colours: token files are the only place a literal hex/rgb/hsl value may
 *   live. Every other file must use a token or color-mix() with a token.
 *   scripts/style-baseline.json is intentionally empty; any new literal fails.
 *
 * Usage: node scripts/check-styles.mjs [--update-baseline]
 */
import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, relative } from "node:path";

const APP_ROOT = fileURLToPath(new URL("..", import.meta.url));
const UI_SRC = join(APP_ROOT, "ui/src");
const BASELINE_PATH = join(APP_ROOT, "scripts/style-baseline.json");
const TOKEN_FILES = new Set(["styles/00-tokens-glass.css", "styles/01-editor.css"]);
const MAX_LINES = 1000;
const WARN_LINES = 700;

function walk(directory) {
  const files = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      if (entry === "node_modules" || entry === "spike") continue;
      files.push(...walk(path));
    } else if (entry.endsWith(".css")) {
      files.push(path);
    }
  }
  return files;
}

function countRawColours(source) {
  // Comments are documentation, not paint. Count only live declarations.
  const live = source.replace(/\/\*[\s\S]*?\*\//g, "");
  const hex = live.match(/#[0-9a-fA-F]{3,8}\b/g) ?? [];
  const rgb = live.match(/rgba?\([^)]*\)/g) ?? [];
  const hsl = live.match(/hsla?\([^)]*\)/g) ?? [];
  return hex.length + rgb.length + hsl.length;
}

function collect() {
  const files = {};
  const violations = [];
  for (const path of walk(UI_SRC)) {
    const key = relative(UI_SRC, path).replaceAll("\\", "/");
    const source = readFileSync(path, "utf8");
    const lines = source.split("\n").length;
    const transitionAll = (source.match(/transition:\s*all\b/g) ?? []).length;
    const rawColours = TOKEN_FILES.has(key) ? 0 : countRawColours(source);

    if (transitionAll > 0) {
      violations.push(`${key}: ${transitionAll} \`transition: all\` declaration(s)`);
    }
    if (lines > MAX_LINES) {
      violations.push(`${key}: ${lines} lines (max ${MAX_LINES})`);
    }
    files[key] = { lines, rawColours };
  }
  return { files, violations };
}

function loadBaseline() {
  if (!existsSync(BASELINE_PATH)) return { files: {} };
  return JSON.parse(readFileSync(BASELINE_PATH, "utf8"));
}

function main() {
  const update = process.argv.includes("--update-baseline");
  const { files, violations } = collect();
  for (const [key, value] of Object.entries(files)) {
    if (value.lines > WARN_LINES && value.lines <= MAX_LINES) {
      console.warn(`! ${key}: ${value.lines} lines (warning threshold ${WARN_LINES})`);
    }
  }

  if (update) {
    const baseline = {
      $comment:
        "Raw-colour ratchet for Phase 1 G1d. Regenerate with `node scripts/check-styles.mjs --update-baseline` after reducing debt. Never raise a number for new code.",
      files: Object.fromEntries(
        Object.entries(files)
          .filter(([, value]) => value.rawColours > 0)
          .sort(([left], [right]) => left.localeCompare(right))
          .map(([key, value]) => [key, { rawColours: value.rawColours }]),
      ),
    };
    writeFileSync(BASELINE_PATH, `${JSON.stringify(baseline, null, 2)}\n`);
    console.log(`✓ wrote ${relative(APP_ROOT, BASELINE_PATH)} (${Object.keys(baseline.files).length} files)`);
    return;
  }

  const baseline = loadBaseline();
  const increases = [];
  for (const [key, value] of Object.entries(files)) {
    const allowed = baseline.files[key]?.rawColours ?? 0;
    if (value.rawColours > allowed) {
      increases.push(`${key}: ${value.rawColours} raw colours (baseline ${allowed})`);
    }
  }
  const decreases = Object.entries(files).filter(([key, value]) => {
    const allowed = baseline.files[key]?.rawColours ?? 0;
    return value.rawColours < allowed;
  });

  for (const violation of [...violations, ...increases]) console.error(`✗ ${violation}`);
  for (const [key, value] of decreases) {
    console.log(
      `↓ ${key}: ${value.rawColours} raw colours (baseline ${baseline.files[key].rawColours}); update the baseline`,
    );
  }

  const total = Object.values(files).reduce((sum, value) => sum + value.rawColours, 0);
  if (violations.length + increases.length > 0) {
    console.error(
      `✗ styles: ${violations.length} rule violation(s), ${increases.length} raw-colour increase(s)`,
    );
    process.exitCode = 1;
    return;
  }
  console.log(`✓ styles: no transition:all, no oversized files, ${total} raw colours outside token files`);
}

main();
