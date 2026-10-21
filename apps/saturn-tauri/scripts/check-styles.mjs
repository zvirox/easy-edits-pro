#!/usr/bin/env node
/**
 * Phase 1 style gate.
 *
 * - No `transition: all` (layout/unknown-property animation is banned).
 * - No CSS file over 1,000 lines.
 * - No transition or keyframe animation of layout properties (width, height,
 *   inset, margin, padding, backdrop-filter). Phase 3: compositor-only motion.
 * - No transition or animation declarations on playhead selectors: the
 *   timeline playhead must never be animated behind the editor's back.
 * - `backdrop-filter` only lives in 00-tokens-glass.css and 120-glass.css.
 *   Media components may not carry glass classes or inline backdrop filters.
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
const GLASS_FILES = new Set(["styles/00-tokens-glass.css", "styles/120-glass.css"]);
const MEDIA_COMPONENTS = new Set([
  "features/preview/PreviewSection.tsx",
  "features/timeline/TimelineSection.tsx",
  "features/timeline/TimelineTrackLane.tsx",
  "components/VideoScopes.tsx",
  "components/AudioMonitorMixer.tsx",
  "components/SourceAudioVisualizer.tsx",
  "workspaces/ColorWorkspace.tsx",
]);
const MAX_LINES = 1000;
const WARN_LINES = 700;

/** Properties whose transition/animation forces layout work in WebKit. */
function isLayoutProperty(name) {
  return (
    name === "all" ||
    name === "width" ||
    name === "height" ||
    name === "top" ||
    name === "right" ||
    name === "bottom" ||
    name === "left" ||
    name.startsWith("inset") ||
    name.startsWith("margin") ||
    name.startsWith("padding") ||
    name === "backdrop-filter"
  );
}

/** First token of every comma-separated transition target, e.g. `width` in `transition: width 0.2s` */
function transitionTargets(declarationValue) {
  return declarationValue
    .split(",")
    .map((part) => part.trim().split(/\s+/)[0] ?? "")
    .filter((name) => /^[a-z-]+$/i.test(name));
}

/**
 * Walks a CSS source and returns flat rules: { kind: "rule" | "keyframes",
 * selector/name, body }. Nested at-rules (@media/@supports/@layer) are
 * flattened so their inner rules are also checked.
 */
function collectCssRules(source) {
  const css = source.replace(/\/\*[\s\S]*?\*\//g, "");
  const rules = [];
  function walk(chunk) {
    let position = 0;
    while (position < chunk.length) {
      const open = chunk.indexOf("{", position);
      if (open === -1) break;
      const prelude = chunk.slice(position, open).trim().replace(/[\n;]+/g, " ");
      let depth = 1;
      let end = open + 1;
      while (end < chunk.length && depth > 0) {
        const character = chunk[end];
        if (character === "{") depth += 1;
        else if (character === "}") depth -= 1;
        end += 1;
      }
      const body = chunk.slice(open + 1, end - 1);
      if (prelude.startsWith("@keyframes")) {
        rules.push({ kind: "keyframes", name: prelude.split(/\s+/)[1] ?? "", body });
      } else if (prelude.startsWith("@media") || prelude.startsWith("@supports") || prelude.startsWith("@layer") || prelude.startsWith("@container")) {
        walk(body);
      } else {
        for (const selector of prelude.split(",")) {
          if (selector.trim()) rules.push({ kind: "rule", selector: selector.trim(), body });
        }
      }
      position = end;
    }
  }
  walk(css);
  return rules;
}

function walk(directory, extension = ".css") {
  const files = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      if (entry === "node_modules" || entry === "spike") continue;
      files.push(...walk(path, extension));
    } else if (entry.endsWith(extension)) {
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
    const backdropFilters = (source.match(/backdrop-filter\s*:/g) ?? []).length;
    const rawColours = TOKEN_FILES.has(key) ? 0 : countRawColours(source);
    const layoutTransitions = [];
    const layoutKeyframes = [];
    const playheadMotion = [];

    for (const rule of collectCssRules(source)) {
      if (rule.kind === "keyframes") {
        // Keyframe steps carry declarations inside nested { } blocks.
        for (const step of rule.body.match(/\{([^{}]*)\}/g) ?? []) {
          for (const [, property] of step.matchAll(/[{;]\s*([-\w]+)\s*:/g)) {
            const name = property.toLowerCase();
            if (name.startsWith("--") || !isLayoutProperty(name)) continue;
            layoutKeyframes.push(`${rule.name} (${name})`);
          }
        }
        continue;
      }
      if (/playhead/i.test(rule.selector)) {
        if (/transition(-property)?\s*:/.test(rule.body) || /(^|[^-\w])animation(-name|-duration)?\s*:/.test(rule.body)) {
          playheadMotion.push(rule.selector);
        }
      }
      for (const match of rule.body.matchAll(/transition(-property)?\s*:\s*([^;{}]+)/g)) {
        for (const name of transitionTargets(match[2])) {
          if (isLayoutProperty(name.toLowerCase())) {
            layoutTransitions.push(`${rule.selector} (${name})`);
          }
        }
      }
    }

    if (transitionAll > 0) {
      violations.push(`${key}: ${transitionAll} \`transition: all\` declaration(s)`);
    }
    for (const target of layoutTransitions) {
      violations.push(`${key}: layout-property transition ${target}`);
    }
    for (const target of layoutKeyframes) {
      violations.push(`${key}: layout-property keyframe animation ${target}`);
    }
    for (const selector of playheadMotion) {
      violations.push(`${key}: playhead must not animate \`${selector}\``);
    }
    if (backdropFilters > 0 && !GLASS_FILES.has(key)) {
      violations.push(`${key}: ${backdropFilters} ad-hoc \`backdrop-filter\` declaration(s)`);
    }
    if (lines > MAX_LINES) {
      violations.push(`${key}: ${lines} lines (max ${MAX_LINES})`);
    }
    files[key] = { lines, rawColours };
  }

  for (const path of walk(UI_SRC, ".tsx")) {
    const key = relative(UI_SRC, path).replaceAll("\\", "/");
    const source = readFileSync(path, "utf8");
    for (const value of source.matchAll(/transition:\s*(["'])([^"']+)\1/g)) {
      for (const name of transitionTargets(value[2])) {
        if (isLayoutProperty(name.toLowerCase())) {
          violations.push(`${key}: layout-property transition (inline style, ${name})`);
        }
      }
    }
    if (!MEDIA_COMPONENTS.has(key)) continue;
    if (/\bglass\b|data-elev|backdropFilter|backdrop-filter/.test(source)) {
      violations.push(`${key}: glass/backdrop effects are not allowed on media components`);
    }
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
  console.log(`✓ styles: no transition:all, no layout-property animation, no playhead animation, no oversized files, ${total} raw colours outside token files`);
}

main();
