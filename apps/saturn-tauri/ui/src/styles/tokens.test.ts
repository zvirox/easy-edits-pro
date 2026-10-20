import test from "node:test";
import assert from "node:assert/strict";
import {
  auditContrast,
  contrastRatio,
  parseColor,
  parseThemes,
  resolveToken,
} from "../../../scripts/check-contrast.mjs";

const REQUIRED_COLOUR_TOKENS = [
  "--accent-text",
  "--accent-text-hover",
  "--accent-text-violet",
  "--text-primary",
  "--text-secondary",
  "--text-muted",
  "--bg-canvas",
  "--bg-base",
  "--bg-surface",
  "--bg-elevated",
  "--bg-glass",
  "--glass-border",
];

const REQUIRED_GLASS_TOKENS = [
  "--glass-l1",
  "--glass-l2",
  "--glass-l3",
  "--glass-l4",
  "--glass-highlight",
  "--glass-shadow",
  "--glass-saturate",
  "--blur-l1",
  "--blur-l2",
  "--blur-l3",
  "--blur-l4",
  "--bg-stage",
  "--ease-spring",
  "--ease-out",
  "--dur-fast",
  "--dur-med",
  "--dur-slow",
];

test("both themes define every semantic colour token", () => {
  const themes = parseThemes();
  for (const theme of ["dark", "light"]) {
    for (const name of REQUIRED_COLOUR_TOKENS) {
      assert.ok(resolveToken(name, themes[theme]), `${theme} is missing ${name}`);
    }
  }
});

test("both themes define every glass and motion token", () => {
  const themes = parseThemes();
  for (const theme of ["dark", "light"]) {
    for (const name of REQUIRED_GLASS_TOKENS) {
      assert.ok(themes[theme].has(name), `${theme} is missing ${name}`);
    }
  }
});

test("dark accent text does not reuse the accent surface colour", () => {
  const themes = parseThemes();
  // Light theme may reuse #9f1249: it is legible on light surfaces. Dark theme
  // must not, because #9f1249 is only ~2.4:1 on the dark canvas.
  assert.notEqual(
    resolveToken("--accent-text", themes.dark),
    resolveToken("--accent-primary", themes.dark),
  );
});

test("every semantic text/surface pair meets WCAG AA", () => {
  const { failures } = auditContrast();
  assert.deepEqual(failures, []);
});

test("contrast maths matches the WCAG reference values", () => {
  const black = parseColor("#000000");
  const white = parseColor("#ffffff");
  assert.ok(black && white);
  assert.equal(contrastRatio(black, white), 21);
  assert.equal(contrastRatio(white, white), 1);
});
