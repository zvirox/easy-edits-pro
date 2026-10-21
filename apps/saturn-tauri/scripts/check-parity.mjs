#!/usr/bin/env node
/**
 * FilmCraft parity report.
 *
 * Compares the checked-in EEP catalogue with the inventory recorded from the
 * installed FilmCraft 0.8.0 (`filmcraft-cli describe`) on 2026-10-21.
 * Informational: prints per-family coverage and exits non-zero only when the
 * reference files are malformed.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "../../..");
const eep = JSON.parse(
  readFileSync(resolve(root, "docs/reference/command-catalogue.json"), "utf8"),
);
const filmcraft = JSON.parse(
  readFileSync(resolve(root, "docs/reference/filmcraft-commands.json"), "utf8"),
);

if (!Array.isArray(eep) || !Array.isArray(filmcraft)) {
  console.error("Parity check failed: reference files must be arrays");
  process.exit(1);
}

const family = (id) => String(id).split(".")[0];
const count = (items) => {
  const map = new Map();
  for (const item of items) map.set(family(item.id), (map.get(family(item.id)) ?? 0) + 1);
  return map;
};

const eepCounts = count(eep);
const fcCounts = count(filmcraft);
const eepIds = new Set(eep.map((item) => item.id));
const overlap = filmcraft.filter((item) => eepIds.has(item.id));

const families = [...new Set([...fcCounts.keys(), ...eepCounts.keys()])].sort(
  (a, b) => (fcCounts.get(b) ?? 0) - (fcCounts.get(a) ?? 0),
);

const rows = families.map((name) => ({
  family: name,
  filmcraft: fcCounts.get(name) ?? 0,
  eep: eepCounts.get(name) ?? 0,
}));

const width = Math.max(...rows.map((row) => row.family.length), 6);
console.log(
  `${"family".padEnd(width)}  ${"filmcraft".padStart(9)}  ${"eep".padStart(5)}  gap`,
);
for (const row of rows) {
  const gap = Math.max(0, row.filmcraft - row.eep);
  console.log(
    `${row.family.padEnd(width)}  ${String(row.filmcraft).padStart(9)}  ${String(row.eep).padStart(5)}  ${gap || ""}`,
  );
}

console.log("");
console.log(`FilmCraft commands: ${filmcraft.length}`);
console.log(`EEP commands:       ${eep.length}`);
console.log(`Exact id overlap:   ${overlap.length}`);
console.log("");
console.log("Informational report. Missing families are scheduled in the phase plan.");
