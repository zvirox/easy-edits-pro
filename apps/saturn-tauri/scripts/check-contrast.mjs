#!/usr/bin/env node
/**
 * Phase 1 contrast gate.
 *
 * 1. Parses the semantic colour tokens from 01-editor.css and
 *    00-tokens-glass.css for dark and light themes (aliases resolved).
 * 2. Checks every text token against every surface token (WCAG AA 4.5:1),
 *    plus white-on-accent pairs.
 * 3. Bans accent/muted colours used directly as text. Those must go through
 *    --accent-text / --accent-text-violet / --text-muted.
 *
 * Usage: node scripts/check-contrast.mjs [--verbose]
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const APP_ROOT = fileURLToPath(new URL("..", import.meta.url));
const UI_SRC = join(APP_ROOT, "ui/src");
const STYLES = join(UI_SRC, "styles");

export const TOKEN_FILES = [
  join(STYLES, "01-editor.css"),
  join(STYLES, "00-tokens-glass.css"),
];

export const TEXT_TOKENS = [
  "--text-primary",
  "--text-secondary",
  "--text-muted",
  "--accent-text",
  "--accent-text-hover",
  "--accent-text-violet",
];

export const SURFACE_TOKENS = [
  "--bg-canvas",
  "--bg-base",
  "--bg-surface",
  "--bg-elevated",
  "--bg-glass",
];

/**
 * Component roles introduced by G1d. They resolve to the core surfaces, but
 * auditing them directly catches a role that drifts to a low-contrast value.
 */
export const ROLE_SURFACE_TOKENS = [
  "--bg-workspace",
  "--bg-card",
  "--bg-card-hover",
  "--bg-card-selected",
  "--bg-control",
  "--bg-control-subtle",
  "--bg-status",
  "--bg-node",
  "--bg-strip",
  "--bg-inset",
  "--bg-meter-pair",
  "--bg-chip",
  "--bg-scopes",
  "--bg-scopes-header",
  "--bg-panel",
  "--bg-subpanel",
  "--bg-input",
  "--bg-selected",
  "--bg-hover",
  "--bg-active",
];

export const ON_ACCENT_TOKENS = [
  "--accent-primary",
  "--accent-primary-hover",
  "--accent-secondary",
];

const MIN_TEXT = 4.5;

/** Remove @media/@supports blocks so only top-level token blocks remain. */
function stripAtRuleBlocks(css) {
  let output = "";
  let index = 0;
  while (index < css.length) {
    if (css.startsWith("@media", index) || css.startsWith("@supports", index)) {
      const open = css.indexOf("{", index);
      if (open === -1) break;
      let depth = 1;
      let cursor = open + 1;
      while (cursor < css.length && depth > 0) {
        if (css[cursor] === "{") depth += 1;
        else if (css[cursor] === "}") depth -= 1;
        cursor += 1;
      }
      output += "\n".repeat(css.slice(index, cursor).split("\n").length);
      index = cursor;
    } else {
      output += css[index];
      index += 1;
    }
  }
  return output;
}

function topLevelBlocks(css) {
  const blocks = [];
  let index = 0;
  while (index < css.length) {
    const open = css.indexOf("{", index);
    if (open === -1) break;
    let depth = 1;
    let cursor = open + 1;
    while (cursor < css.length && depth > 0) {
      if (css[cursor] === "{") depth += 1;
      else if (css[cursor] === "}") depth -= 1;
      cursor += 1;
    }
    const selector = css.slice(index, open).trim();
    if (!selector.startsWith("@")) {
      blocks.push({ selector, body: css.slice(open + 1, cursor - 1) });
    }
    index = cursor;
  }
  return blocks;
}

function declarations(body) {
  const found = [];
  // Strip comments first: a comment like "4.5:1" would otherwise swallow the
  // next declaration when we split on semicolons.
  const clean = body.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const chunk of clean.split(";")) {
    const colon = chunk.indexOf(":");
    if (colon === -1) continue;
    const name = chunk.slice(0, colon).trim();
    if (!name.startsWith("--")) continue;
    found.push([name, chunk.slice(colon + 1).trim()]);
  }
  return found;
}

/** Parse both theme token maps. 00-tokens-glass is applied last, so it wins. */
export function parseThemes(files = TOKEN_FILES) {
  const themes = { dark: new Map(), light: new Map() };
  for (const file of files) {
    const css = stripAtRuleBlocks(readFileSync(file, "utf8"));
    for (const { selector, body } of topLevelBlocks(css)) {
      // Only theme blocks belong in the maps. [data-glass="lite|off"] and
      // component selectors are degradation/override modes, not themes.
      // `:root` is global: it seeds both maps, then the theme-specific blocks
      // override it. This mirrors how the browser applies the CSS.
      const selectors = selector.split(",").map((part) => part.trim());
      const appliesToRoot = selectors.some((part) => part === ":root");
      const appliesToDark = appliesToRoot || selectors.some((part) => part.includes("dark"));
      const appliesToLight = appliesToRoot || selectors.some((part) => part.includes("light"));
      if (!appliesToDark && !appliesToLight) continue;
      const found = declarations(body);
      if (appliesToDark) {
        for (const [name, value] of found) themes.dark.set(name, value);
      }
      if (appliesToLight) {
        for (const [name, value] of found) themes.light.set(name, value);
      }
    }
  }
  return themes;
}

function resolveValue(name, tokens, seen = new Set()) {
  if (seen.has(name)) return null;
  const raw = tokens.get(name);
  if (raw === undefined) return null;
  const nextSeen = new Set(seen).add(name);
  return raw.replace(
    /var\(\s*(--[\w-]+)\s*(?:,\s*([^)]*))?\)/g,
    (_, reference, fallback) => {
      const resolved = resolveValue(reference, tokens, nextSeen);
      return resolved ?? fallback ?? "";
    },
  );
}

export function resolveToken(name, tokens) {
  return resolveValue(name, tokens);
}

export function parseColor(value) {
  if (!value) return null;
  const text = value.trim().toLowerCase();
  if (text === "transparent") return { r: 0, g: 0, b: 0, a: 0 };
  if (text === "white") return { r: 255, g: 255, b: 255, a: 1 };
  if (text === "black") return { r: 0, g: 0, b: 0, a: 1 };

  const hex = /^#([0-9a-f]{3,8})$/.exec(text);
  if (hex) {
    let digits = hex[1];
    if (digits.length === 3 || digits.length === 4) {
      digits = digits
        .split("")
        .map((digit) => digit + digit)
        .join("");
    }
    if (digits.length !== 6 && digits.length !== 8) return null;
    const [r, g, b] = [0, 2, 4].map((offset) => Number.parseInt(digits.slice(offset, offset + 2), 16));
    const a = digits.length === 8 ? Number.parseInt(digits.slice(6, 8), 16) / 255 : 1;
    return { r, g, b, a };
  }

  const rgb = /^rgba?\(([^)]+)\)$/.exec(text);
  if (rgb) {
    const parts = rgb[1].split(/[\s,/]+/).filter(Boolean);
    if (parts.length < 3) return null;
    const channel = (part) => {
      if (part.endsWith("%")) return (Number.parseFloat(part) / 100) * 255;
      return Number.parseFloat(part);
    };
    const alphaPart = parts[3];
    let alpha = 1;
    if (alphaPart !== undefined) {
      alpha = alphaPart.endsWith("%")
        ? Number.parseFloat(alphaPart) / 100
        : Number.parseFloat(alphaPart);
    }
    const [r, g, b] = parts.slice(0, 3).map(channel);
    if ([r, g, b].some((item) => Number.isNaN(item))) return null;
    return { r, g, b, a: Number.isNaN(alpha) ? 1 : alpha };
  }

  // G1d uses color-mix() for translucent overlays. The audit only needs the
  // common shape: an opaque colour mixed with transparent at a percentage.
  const mix = /^color-mix\(in srgb,\s*(.+?)\s+([\d.]+)%,\s*transparent\)$/.exec(text);
  if (mix) {
    const base = parseColor(mix[1]);
    if (!base) return null;
    const weight = Number.parseFloat(mix[2]) / 100;
    if (Number.isNaN(weight)) return null;
    return { ...base, a: (base.a ?? 1) * weight };
  }

  return null;
}

function composite(foreground, background) {
  const alpha = foreground.a ?? 1;
  return {
    r: foreground.r * alpha + background.r * (1 - alpha),
    g: foreground.g * alpha + background.g * (1 - alpha),
    b: foreground.b * alpha + background.b * (1 - alpha),
    a: 1,
  };
}

function luminance(color) {
  const linear = (channel) => {
    const value = channel / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b);
}

export function contrastRatio(foreground, background) {
  const light = Math.max(luminance(foreground), luminance(background));
  const dark = Math.min(luminance(foreground), luminance(background));
  return (light + 0.05) / (dark + 0.05);
}

function effectiveColor(value, tokens, canvas) {
  const parsed = parseColor(value);
  if (!parsed) return null;
  return parsed.a < 1 ? composite(parsed, canvas) : parsed;
}

/** Audit every semantic text/surface pair for both themes. */
export function auditContrast(files = TOKEN_FILES) {
  const themes = parseThemes(files);
  const rows = [];
  const failures = [];

  for (const [theme, tokens] of Object.entries(themes)) {
    const canvas = effectiveColor(resolveToken("--bg-canvas", tokens), tokens, {
      r: 0,
      g: 0,
      b: 0,
      a: 1,
    });
    if (!canvas) {
      failures.push({ theme, message: "--bg-canvas is missing or not a colour" });
      continue;
    }

    for (const textToken of TEXT_TOKENS) {
      for (const surfaceToken of [...SURFACE_TOKENS, ...ROLE_SURFACE_TOKENS]) {
        const foreground = effectiveColor(resolveToken(textToken, tokens), tokens, canvas);
        const background = effectiveColor(resolveToken(surfaceToken, tokens), tokens, canvas);
        if (!foreground || !background) {
          failures.push({
            theme,
            message: `${textToken} on ${surfaceToken}: missing or unsupported colour`,
          });
          continue;
        }
        const ratio = contrastRatio(foreground, background);
        const row = {
          theme,
          foreground: textToken,
          background: surfaceToken,
          role: ROLE_SURFACE_TOKENS.includes(surfaceToken),
          ratio,
          minimum: MIN_TEXT,
          pass: ratio >= MIN_TEXT,
        };
        rows.push(row);
        if (!row.pass) failures.push(row);
      }
    }

    for (const backgroundToken of ON_ACCENT_TOKENS) {
      const foreground = effectiveColor(resolveToken("--text-on-accent", tokens), tokens, canvas);
      const background = effectiveColor(resolveToken(backgroundToken, tokens), tokens, canvas);
      if (!foreground || !background) {
        failures.push({ theme, message: `--text-on-accent on ${backgroundToken}: missing colour` });
        continue;
      }
      const ratio = contrastRatio(foreground, background);
      const row = {
        theme,
        foreground: "--text-on-accent",
        background: backgroundToken,
        ratio,
        minimum: MIN_TEXT,
        pass: ratio >= MIN_TEXT,
      };
      rows.push(row);
      if (!row.pass) failures.push(row);
    }
  }

  return { themes, rows, failures };
}

const BANNED_TEXT_COLOURS = [
  {
    pattern: /(?<![-])color:\s*var\(--accent-primary\)/i,
    message: "accent background token used as text; use --accent-text",
  },
  {
    pattern: /(?<![-])color:\s*var\(--accent-secondary\)/i,
    message: "violet background token used as text; use --accent-text-violet",
  },
  {
    pattern: /(?<![-])color:\s*var\(--color-accent-primary/i,
    message: "legacy accent alias used as text; use --accent-text",
  },
  {
    pattern: /(?<![-])color:\s*#(?:9f1249|be185d|d9468f|a855f7|c084fc|6b21a8)\b/i,
    message: "raw accent colour used as text; use an --accent-text-* token",
  },
  {
    pattern: /(?<![-])color:\s*#(?:7d6377|bfa5b8)\b/i,
    message: "raw muted colour used as text; use --text-muted",
  },
];

function walkCssAndTsx(directory) {
  const files = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      if (entry === "spike" || entry === "node_modules") continue;
      files.push(...walkCssAndTsx(path));
      continue;
    }
    if (/\.(css|tsx|ts)$/.test(entry) && !/\.test\.ts$/.test(entry)) files.push(path);
  }
  return files;
}

export function auditTextColours() {
  const violations = [];
  for (const file of walkCssAndTsx(UI_SRC)) {
    const lines = readFileSync(file, "utf8").split("\n");
    lines.forEach((line, index) => {
      const trimmed = line.trim();
      if (trimmed.startsWith("/*") || trimmed.startsWith("*")) return;
      for (const rule of BANNED_TEXT_COLOURS) {
        if (rule.pattern.test(line)) {
          violations.push({
            file: relative(APP_ROOT, file),
            line: index + 1,
            text: trimmed,
            message: rule.message,
          });
          break;
        }
      }
    });
  }
  return violations;
}

export function run({ verbose = false } = {}) {
  const { rows, failures } = auditContrast();
  const textViolations = auditTextColours();
  const contrastFailures = failures.filter((failure) => "foreground" in failure);

  if (verbose) {
    for (const row of rows) {
      const status = row.pass ? "✓" : "✗";
      console.log(
        `${status} ${row.theme.padEnd(5)} ${row.foreground} on ${row.background}: ${row.ratio.toFixed(2)}:1`,
      );
    }
  }

  for (const failure of contrastFailures) {
    console.error(
      `✗ contrast ${failure.theme}: ${failure.foreground} on ${failure.background} = ${failure.ratio.toFixed(2)}:1 (min ${failure.minimum}:1)`,
    );
  }
  for (const failure of failures.filter((item) => !("foreground" in item))) {
    console.error(`✗ contrast ${failure.theme}: ${failure.message}`);
  }
  for (const violation of textViolations) {
    console.error(`✗ text colour ${violation.file}:${violation.line}: ${violation.message}`);
    console.error(`  ${violation.text}`);
  }

  const failed = contrastFailures.length + textViolations.length;
  if (failed === 0) {
    console.log(`✓ contrast: ${rows.length} token pairs pass, 0 banned text colours`);
    return 0;
  }
  console.error(
    `✗ contrast: ${contrastFailures.length} failed pairs, ${textViolations.length} banned text colours`,
  );
  return 1;
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  process.exitCode = run({ verbose: process.argv.includes("--verbose") });
}
