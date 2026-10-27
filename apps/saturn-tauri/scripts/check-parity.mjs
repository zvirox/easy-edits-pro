#!/usr/bin/env node
/**
 * Reference-editor parity report (packet D1, extended P13).
 *
 * Compares the checked-in EEP catalogue with the inventory recorded from the
 * reference editor on 2026-10-21 (the recorded inventory of that product's
 * command catalogue; see docs/reference/reference-editor-commands.json).
 *
 * - A family alias map maps EEP family prefixes onto the canonical reference-editor
 *   family names they serve (`graphic` -> `graphics`, `subtitle` -> `captions`,
 *   `speech` -> `tts`, ...).
 * - Coverage is **per id**, not per count. `docs/reference/tier<N>-audit.json`
 *   maps every reference-editor tier command to a disposition:
 *     `shipped`    - the same id exists in the EEP catalogue and ships;
 *     `equivalent` - a shipped EEP command fulfils the behavior (documented
 *                    semantic mapping, may use a different id);
 *     `new`        - a genuine gap; once implemented the entry carries an
 *                    `eep` id that must exist and ship in the catalogue;
 *     `na`         - documented divergence (excluded from the denominator).
 * - A tier without an audit file reports its reference-editor count without a
 *   coverage claim, so coverage is never asserted without id-level evidence.
 * - Families are grouped by parity tier (see the plan, section 5.2).
 * - The report is written to `docs/PARITY.md`. Run with `--check` to verify
 *   the committed report is not stale (used by CI); exits non-zero on drift.
 *
 * Informational: without `--check` it exits non-zero when the reference files
 * are malformed or an audit entry is inconsistent with the catalogue.
 */
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(import.meta.dirname, "../../..");
const appRoot = resolve(import.meta.dirname, "..");
const eep = JSON.parse(
  readFileSync(resolve(root, "docs/reference/command-catalogue.json"), "utf8"),
);
const referenceEditor = JSON.parse(
  readFileSync(resolve(root, "docs/reference/reference-editor-commands.json"), "utf8"),
);
const reportPath = resolve(root, "docs/PARITY.md");
const checkOnly = process.argv.includes("--check");

if (!Array.isArray(eep) || !Array.isArray(referenceEditor)) {
  console.error("Parity check failed: reference files must be arrays");
  process.exit(1);
}

// EEP family prefix -> canonical reference-editor family. Families that do not map
// to a reference-editor family (workspace plumbing, node graph, interop, ...) stay
// as-is and land under "app-only" in the report.
const familyAlias = {
  graphic: "graphics",
  title: "graphics",
  marker: "markers",
  caption: "captions",
  subtitle: "captions",
  speech: "tts",
};

// Tier assignment per reference-editor family (plan section 5.2). Anything unlisted
// is app plumbing and is reported outside the tiers.
const tierByFamily = {
  1: ["file", "project", "sequence", "clip", "timeline", "edit", "trim", "playhead", "markers", "captions", "export", "media", "shortcuts", "prefs", "history"],
  2: ["graphics", "effects", "masks", "lumetri", "lut", "presets", "color", "mixer", "clipMixer", "audio", "multicam", "transcript"],
  3: ["tts", "mediaBrowser", "collab"],
};
const familyTier = new Map();
for (const [tier, families] of Object.entries(tierByFamily)) {
  for (const family of families) familyTier.set(family, Number(tier));
}

const familyOf = (id) => {
  const raw = String(id).split(".")[0];
  return familyAlias[raw] ?? raw;
};

// --- Per-id audit ----------------------------------------------------------
const eepById = new Map(eep.map((item) => [item.id, item]));
const audits = new Map(); // tier -> { byId: Map, covered: Set, na: Set }
for (const tier of [1, 2, 3]) {
  const path = resolve(root, `docs/reference/tier${tier}-audit.json`);
  if (!existsSync(path)) continue;
  const raw = JSON.parse(readFileSync(path, "utf8"));
  const byId = new Map(Object.entries(raw));
  const fcInTier = referenceEditor.filter(
    (item) => familyTier.get(familyOf(item.id)) === tier,
  );
  const errors = [];
  const covered = new Set();
  const na = new Set();
  for (const [id, entry] of byId) {
    if (familyTier.get(familyOf(id)) !== tier) {
      errors.push(`${id}: audit entry is not a Tier-${tier} command`);
    }
    if (entry.disposition === "shipped" || entry.disposition === "equivalent") {
      const spec = entry.eep ? eepById.get(entry.eep) : undefined;
      if (!spec || !spec.shipped) {
        errors.push(`${id}: mapped ${entry.eep ?? "(none)"} is not a shipped EEP command`);
      } else {
        covered.add(id);
      }
    } else if (entry.disposition === "new") {
      if (entry.eep) {
        const spec = eepById.get(entry.eep);
        if (!spec || !spec.shipped) {
          errors.push(`${id}: implemented ${entry.eep ?? "(none)"} is not a shipped EEP command`);
        } else {
          covered.add(id);
        }
      }
    } else if (entry.disposition === "na") {
      na.add(id);
      if (!entry.note) errors.push(`${id}: na disposition requires a note`);
    } else {
      errors.push(`${id}: unknown disposition ${entry.disposition}`);
    }
  }
  for (const item of fcInTier) {
    if (!byId.has(item.id)) errors.push(`${item.id}: Tier-${tier} command missing from the audit`);
  }
  if (errors.length) {
    console.error(`Parity check failed: tier${tier}-audit.json is inconsistent with the catalogue:`);
    for (const error of errors) console.error(`  - ${error}`);
    process.exit(1);
  }
  audits.set(tier, { byId, covered, na, total: fcInTier.length });
}

const tally = (items) => {
  const map = new Map();
  for (const item of items) {
    const family = familyOf(item.id);
    const entry = map.get(family) ?? { shipped: 0, planned: 0 };
    entry[String(item.shipped) === "true" ? "shipped" : "planned"] += 1;
    map.set(family, entry);
  }
  return map;
};

const fcCounts = new Map();
for (const item of referenceEditor) {
  const family = familyOf(item.id);
  fcCounts.set(family, (fcCounts.get(family) ?? 0) + 1);
}
const eepTally = tally(eep);
const families = [...new Set([...fcCounts.keys(), ...eepTally.keys()])].sort((a, b) => {
  const tierDelta = (familyTier.get(a) ?? 9) - (familyTier.get(b) ?? 9);
  if (tierDelta !== 0) return tierDelta;
  return (fcCounts.get(b) ?? 0) - (fcCounts.get(a) ?? 0);
});

const rows = families.map((family) => {
  const tier = familyTier.get(family) ?? null;
  const audit = tier === null ? undefined : audits.get(tier);
  let covered = null;
  let na = 0;
  if (audit) {
    covered = 0;
    for (const item of referenceEditor) {
      if (familyOf(item.id) === family) {
        if (audit.covered.has(item.id)) covered += 1;
        if (audit.na.has(item.id)) na += 1;
      }
    }
  }
  const referenceEditorCount = fcCounts.get(family) ?? 0;
  const tallyRow = eepTally.get(family) ?? { shipped: 0, planned: 0 };
  return {
    family,
    tier,
    referenceEditor: referenceEditorCount,
    covered,
    na,
    gap:
      covered === null
        ? ""
        : Math.max(0, referenceEditorCount - na - covered) || "",
  };
});

const tierSummary = [1, 2, 3].map((tier) => {
  const tierRows = rows.filter((row) => row.tier === tier);
  const fc = tierRows.reduce((sum, row) => sum + row.referenceEditor, 0);
  const audit = audits.get(tier);
  const covered = audit ? audit.covered.size : null;
  const na = audit ? audit.na.size : 0;
  const percent = covered === null ? null : Math.round((covered / (fc - na)) * 100);
  return { tier, fc, covered, na, percent };
});

// --- Markdown report -------------------------------------------------------
const lines = [
  "# Reference-editor parity report",
  "",
  "<!-- Generated by apps/saturn-tauri/scripts/check-parity.mjs from",
  "     docs/reference/command-catalogue.json,",
  "     docs/reference/reference-editor-commands.json, and",
  "     docs/reference/tier<N>-audit.json. Regenerate with",
  "     `pnpm check:parity`; CI verifies this file with `--check`. -->",
  "",
  "Verified-before-done rule: a command counts as shipped only with a UI",
  "affordance, a test, and `shipped: true` in the catalogue.",
  "",
  "Coverage is per id: a reference-editor command counts as covered when the audit",
  "maps it to a shipped EEP command (`shipped` id match, documented",
  "`equivalent`, or an implemented `new` entry). Documented `na` divergences",
  "are excluded from the denominator. Tiers without an audit file report no",
  "coverage.",
  "",
  "## Tier coverage",
  "",
  "| Tier | Reference editor | N/A | Covered | Coverage |",
  "|---|---:|---:|---:|---:|",
  ...tierSummary.map((entry) =>
    entry.covered === null
      ? `| Tier ${entry.tier} | ${entry.fc} | - | - | audit pending |`
      : `| Tier ${entry.tier} | ${entry.fc} | ${entry.na} | ${entry.covered} | ${entry.percent}% |`,
  ),
  "",
  "Tier 1 targets Beta 1; Tier 2 comes after Beta 1; Tier 3 is later.",
  "Full parity (all reference-editor commands) is explicitly **not** a Beta goal.",
  "",
  "## Per-family coverage",
  "",
  "| Family | Tier | Reference editor | Covered | N/A | Gap |",
  "|---|---|---:|---:|---:|---:|",
  ...rows.map((row) =>
    row.covered === null
      ? `| ${row.family} | ${row.tier ?? "-"} | ${row.referenceEditor} | - | - | |`
      : `| ${row.family} | ${row.tier ?? "-"} | ${row.referenceEditor} | ${row.covered} | ${row.na} | ${row.gap} |`,
  ),
  "",
  "## Totals",
  "",
  `Reference-editor commands: ${referenceEditor.length}`,
  "",
  `EEP commands: ${eep.length} (${eep.filter((item) => item.shipped).length} shipped, ${eep.filter((item) => !item.shipped).length} planned)`,
  "",
  "App-only families (no reference-editor counterpart): " +
    rows.filter((row) => row.tier === null && row.referenceEditor === 0).map((row) => row.family).sort().join(", "),
  "",
];
const report = lines.join("\n");

if (checkOnly) {
  const committed = readFileSync(reportPath, "utf8");
  if (committed !== report) {
    console.error(
      "Parity report drift: docs/PARITY.md does not match the catalogue.",
    );
    console.error("Run `pnpm check:parity` to regenerate it, then commit.");
    process.exit(1);
  }
  console.log("Parity report is up to date.");
} else {
  writeFileSync(reportPath, report);
  const width = Math.max(...rows.map((row) => row.family.length), 6);
  console.log(
    `${"family".padEnd(width)}  ${"tier".padStart(4)}  ${"fc".padStart(5)}  ${"covered".padStart(7)}  ${"na".padStart(4)}  gap`,
  );
  for (const row of rows) {
    console.log(
      `${row.family.padEnd(width)}  ${String(row.tier ?? "-").padStart(4)}  ${String(row.referenceEditor).padStart(5)}  ${row.covered === null ? "-".padStart(7) : String(row.covered).padStart(7)}  ${row.covered === null ? "-".padStart(4) : String(row.na).padStart(4)}  ${row.gap || ""}`,
    );
  }
  console.log("");
  console.log(`Reference-editor commands: ${referenceEditor.length}`);
  console.log(
    `EEP commands:       ${eep.length} (${eep.filter((item) => item.shipped).length} shipped, ${eep.filter((item) => !item.shipped).length} planned)`,
  );
  for (const entry of tierSummary) {
    console.log(
      entry.covered === null
        ? `Tier ${entry.tier} coverage:      audit pending (${entry.fc} commands)`
        : `Tier ${entry.tier} coverage:      ${entry.covered}/${entry.fc - entry.na} (${entry.percent}%)`,
    );
  }
  console.log("");
  console.log(`Wrote ${reportPath}`);
}
