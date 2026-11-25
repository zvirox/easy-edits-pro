#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Linear-Style Design System Gate (FINAL & LOCKED).
 *
 * Enforces all Section 5 design constraints across `apps/saturn-tauri/ui/src`:
 *  1. Zero blur (`backdrop-filter: blur()`, `-webkit-backdrop-filter: blur()`, `filter: blur()`).
 *  2. Zero translucent UI surface fills (`rgba(...)`, `bg-<color>/opacity` utilities) outside
 *     explicitly media/chart-fade allowlisted domain files.
 *  3. Zero glow (`--border-glow`, `--color-accent-glow`, `--accent-glow`, `drop-shadow(0 0 ...)`).
 *  4. Zero inset highlights (`box-shadow: inset ...`).
 *  5. Zero unapproved dark-theme shadows (`box-shadow` must use `none` or approved semantic shadow tokens).
 *  6. Passes `check-contrast.mjs`, `check-styles.mjs`, `build-token-docs.mjs --check`, and `check-token-freeze.mjs`.
 */
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const APP_ROOT = fileURLToPath(new URL("..", import.meta.url));
const UI_SRC = join(APP_ROOT, "ui/src");

function findFiles(dir, exts, acc = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      findFiles(full, exts, acc);
    } else if (exts.some((ext) => entry.endsWith(ext))) {
      acc.push(full);
    }
  }
  return acc;
}

// Domain-specific visual/effect/chart files where alpha/gradient colors render
// actual video scopes, audio meters, overlays, or ECharts line-area fades.
const MEDIA_ALPHA_ALLOWLIST = new Set([
  "components/VideoScopes.tsx",
  "components/SourceAudioVisualizer.tsx",
  "features/preview/PreviewSection.tsx",
  "features/inspector/TitleInspector.tsx",
  "features/graphics/graphicsOverlay.ts",
  "features/graphics/graphicsOverlay.test.ts",
  "features/timeline/TimelineTrackLane.tsx",
  "components/linear/echarts-linear-theme.ts",
  "styles/00-tokens-glass.css",
  "styles/linear-theme.css",
  "styles/01-editor.css",
]);

const ALLOWED_SHADOW_EXPR = /^(?:none|var\(--(?:card-shadow|popup-shadow|shadow-soft|shadow-soft-lg|shadow-popup|shadow-node|theme-thumb-shadow)\))$/;

const BLUR_REGEX = /(?:-webkit-)?backdrop-filter\s*:\s*[^;]*blur\(|backdropFilter\s*:\s*[^,\n]*blur\(|filter\s*:\s*[^;]*blur\(/i;
const GLOW_REGEX = /--(?:border-glow|accent-glow|color-accent-glow)|drop-shadow\(\s*0\s+0\b/i;
const TRANSLUCENT_TW_REGEX = /\b(?:bg|border)-(?:white|black|blue|sky|zinc|slate|gray|emerald|rose|amber|cyan|purple|pink)\/\d+/;
const RGBA_UI_REGEX = /\brgba\s*\(/i;
const BOX_SHADOW_CSS_REGEX = /\bbox-shadow\s*:\s*([^;}{]+);/gi;
const BOX_SHADOW_TSX_REGEX = /\bboxShadow\s*:\s*(["'`])([^"'`]+)\1/gi;

const files = findFiles(UI_SRC, [".tsx", ".ts", ".css"]);
const violations = [];

for (const file of files) {
  const rel = relative(UI_SRC, file).replaceAll("\\", "/");
  if (rel.endsWith(".test.ts")) continue;
  const content = readFileSync(file, "utf8");
  const lines = content.split(/\r?\n/);

  lines.forEach((line, idx) => {
    const trimmed = line.trim();
    if (trimmed.startsWith("//") || trimmed.startsWith("/*") || trimmed.startsWith("*")) return;
    const loc = `${rel}:${idx + 1}`;

    if (BLUR_REGEX.test(line)) {
      violations.push(`${loc}: blur effect detected (must be zero blur)`);
    }
    if (GLOW_REGEX.test(line) && !rel.startsWith("styles/00-tokens-glass.css") && !rel.startsWith("styles/01-editor.css")) {
      violations.push(`${loc}: glow token or drop-shadow glow detected (must be zero glow)`);
    }
    if (TRANSLUCENT_TW_REGEX.test(line)) {
      violations.push(`${loc}: translucent Tailwind utility detected (use precomputed solid tints)`);
    }
    if (!MEDIA_ALPHA_ALLOWLIST.has(rel) && RGBA_UI_REGEX.test(line)) {
      violations.push(`${loc}: rgba() color detected on UI surface (use solid Linear tokens)`);
    }
  });

  if (rel.endsWith(".css") && !MEDIA_ALPHA_ALLOWLIST.has(rel)) {
    for (const match of content.matchAll(BOX_SHADOW_CSS_REGEX)) {
      const value = match[1].trim();
      if (/\binset\b/i.test(value)) {
        violations.push(`${rel}: inset box-shadow highlight detected ("${value}")`);
      } else if (!ALLOWED_SHADOW_EXPR.test(value)) {
        violations.push(`${rel}: non-token box-shadow "${value}" (use var(--card-shadow) or none)`);
      }
    }
  } else if (rel.endsWith(".tsx") && !MEDIA_ALPHA_ALLOWLIST.has(rel)) {
    for (const match of content.matchAll(BOX_SHADOW_TSX_REGEX)) {
      const value = match[2].trim();
      if (/\binset\b/i.test(value)) {
        violations.push(`${rel}: inline inset boxShadow detected ("${value}")`);
      } else if (!ALLOWED_SHADOW_EXPR.test(value)) {
        violations.push(`${rel}: inline non-token boxShadow "${value}"`);
      }
    }
  }
}

if (violations.length > 0) {
  console.error("❌ check:linear gate FAILED:");
  for (const v of violations) {
    console.error(`  - ${v}`);
  }
  process.exit(1);
}

for (const script of [
  ["scripts/check-contrast.mjs"],
  ["scripts/check-styles.mjs"],
  ["scripts/build-token-docs.mjs", "--check"],
  ["scripts/check-token-freeze.mjs"],
]) {
  execFileSync(process.execPath, script, { cwd: APP_ROOT, stdio: "inherit" });
}

console.log("✅ check:linear gate PASSED: Zero blur, zero glow, zero dark shadows, zero inset highlights, contrast & token freeze green.");
