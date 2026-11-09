#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

// AF0 licence-policy gate (docs/audit/af0-inventory.md §6).
//
// What this gate enforces:
//   1. `cargo deny check licenses` passes for BOTH workspaces (root and
//      apps/saturn-tauri) under the scoped deny.toml policy (skipped with
//      --skip-deny when CI's cargo-deny-action steps already ran it).
//   2. Every AGPL-declared Cargo package in either resolved graph is a
//      first-party workspace member: name must match the expected set AND the
//      manifest must live inside this repository. This closes the hole that
//      cargo-deny's per-crate exceptions match by name only.
//   3. Negative controls (self-tests, in a throwaway scratch workspace):
//        a. a third-party GPL-3.0-only dependency FAILS the deny policy;
//        b. a foreign crate named like a first-party one (saturn-core) passes
//           cargo-deny by name but FAILS the repository-path assertion.
//   4. Production JavaScript dependencies report only allow-listed licences
//      (`pnpm licenses list --prod` in apps/saturn-tauri). Skipped when
//      node_modules is absent unless --require-js is given.
//
// Usage:
//   node scripts/check-licence-policy.mjs                  # full local gate
//   node scripts/check-licence-policy.mjs --skip-deny      # CI: assertions+controls+JS
//   node scripts/check-licence-policy.mjs --require-js     # JS gate must not skip
//   node scripts/check-licence-policy.mjs --skip-js        # JS gate off (e.g. no toolchain)

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DENY_CONFIG = path.join(REPO, "deny.toml");
const WORKSPACES = [
  { name: "root", cargoToml: path.join(REPO, "Cargo.toml") },
  { name: "desktop", cargoToml: path.join(REPO, "apps", "saturn-tauri", "Cargo.toml") },
];

// The only Cargo packages allowed to declare AGPL, per workspace. Keep in sync
// with deny.toml's exceptions and the workspace member lists.
const ROOT_AGPL = [
  "saturn-audio-dsp", "saturn-cli", "saturn-commands", "saturn-control", "saturn-core",
  "saturn-graphics", "saturn-mcp", "saturn-perception", "saturn-plugin-fixture",
  "saturn-plugin-worker", "saturn-project", "saturn-render", "saturn-text", "saturn-timeline",
];
const DESKTOP_AGPL = [
  "saturn-audio-dsp", "saturn-core", "saturn-perception", "saturn-render", "saturn-tauri",
  "saturn-text",
];

// Production JS identifiers accepted by the plan (deny-by-default; anything
// else fails and needs a recorded maintainer decision).
const JS_ALLOWED = new Set([
  "MIT", "MIT-0", "Apache-2.0", "ISC", "0BSD", "BSD-2-Clause", "BSD-3-Clause",
  "Unlicense", "Zlib",
]);

const flags = new Set(process.argv.slice(2));
const SKIP_DENY = flags.has("--skip-deny");
const REQUIRE_JS = flags.has("--require-js");
const SKIP_JS = flags.has("--skip-js");
let failed = false;

function fail(message) {
  console.error(`licence-policy: FAIL ${message}`);
  failed = true;
}

function pass(message) {
  console.log(`licence-policy: ok ${message}`);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    cwd: options.cwd ?? REPO,
    maxBuffer: 64 * 1024 * 1024,
    env: { ...process.env, CARGO_TERM_COLOR: "never" },
  });
  return { status: result.status, output: `${result.stdout ?? ""}${result.stderr ?? ""}` };
}

function agplPackages(workspace, locked) {
  const args = ["metadata", "--format-version", "1", "--manifest-path", workspace.cargoToml];
  if (locked) args.push("--locked");
  const result = run("cargo", args);
  if (result.status !== 0) {
    throw new Error(`cargo metadata failed for ${workspace.name}:\n${result.output.slice(-2000)}`);
  }
  const data = JSON.parse(result.output);
  return data.packages.filter((pkg) => (pkg.license ?? "").includes("AGPL"));
}

/**
 * AGPL may only appear on first-party manifests: the name must be in the
 * expected set and the manifest path must resolve inside the repository.
 * Throws with a clear message on the first violation. `expectedNames` defaults
 * to the workspace's approved set; pass [] to check paths only (lookalike
 * self-test).
 */
export function assertFirstPartyAgpl(workspace, repoRoot, locked, expectedNames) {
  const agpl = agplPackages(workspace, locked);
  const expected = expectedNames ?? (workspace.name === "root" ? ROOT_AGPL : DESKTOP_AGPL);
  const names = agpl.map((pkg) => pkg.name).sort();
  if (expectedNames === undefined) {
    const expectedSorted = [...expected].sort();
    const outside = names.filter((name) => !expected.includes(name));
    if (outside.length) {
      throw new Error(
        `${workspace.name} workspace has AGPL-declared packages outside the approved set: ${outside.join(", ")}`,
      );
    }
    const missing = expectedSorted.filter((name) => !names.includes(name));
    if (missing.length) {
      throw new Error(
        `${workspace.name} workspace is missing expected first-party AGPL declarations (metadata contradiction regression?): ${missing.join(", ")}`,
      );
    }
  }
  for (const pkg of agpl) {
    const manifest = path.resolve(pkg.manifest_path);
    if (!manifest.startsWith(repoRoot + path.sep)) {
      throw new Error(
        `${workspace.name} workspace: AGPL package "${pkg.name}" resolves outside the repository (${manifest}); the per-crate exception must not be inherited by foreign crates`,
      );
    }
  }
  return agpl.map((pkg) => pkg.name);
}

function checkDenoPolicy() {
  for (const workspace of WORKSPACES) {
    const result = run(
      "cargo",
      [
        "deny", "--manifest-path", workspace.cargoToml, "--locked",
        "--config", DENY_CONFIG, "check", "licenses",
      ],
    );
    if (result.status !== 0) {
      fail(`cargo deny check licenses failed for the ${workspace.name} workspace`);
      console.error(result.output.slice(-4000));
    } else {
      pass(`cargo deny check licenses (${workspace.name} workspace, scoped policy)`);
    }
  }
}

function checkFirstPartyOwnership() {
  try {
    for (const workspace of WORKSPACES) {
      const names = assertFirstPartyAgpl(workspace, REPO, true);
      pass(`AGPL declarations are first-party in the ${workspace.name} workspace (${names.length} packages)`);
    }
  } catch (error) {
    fail(String(error.message ?? error));
  }
}

function withScratchWorkspace(label, rootToml, memberToml, fn) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), `easy-edits-pro-${label}-`));
  try {
    fs.writeFileSync(path.join(dir, "Cargo.toml"), rootToml);
    fs.mkdirSync(path.join(dir, "stub"));
    fs.writeFileSync(path.join(dir, "stub", "Cargo.toml"), memberToml);
    fs.writeFileSync(path.join(dir, "stub", "lib.rs"), "// negative-control stub\n");
    return fn(dir);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

function checkNegativeControls() {
  // Control A: a third-party GPL-3.0-only dependency must FAIL cargo-deny.
  const gplResult = withScratchWorkspace(
    "gpl-negative",
    '[workspace]\nmembers = ["stub"]\nresolver = "3"\n',
    '[package]\nname = "negative-fixture"\nversion = "0.0.0"\nedition = "2021"\nlicense = "GPL-3.0-only"\n[lib]\npath = "lib.rs"\n',
    (dir) =>
      run(
        "cargo",
        ["deny", "--manifest-path", path.join(dir, "Cargo.toml"), "--config", DENY_CONFIG, "check", "licenses"],
        { cwd: dir },
      ),
  );
  if (gplResult.status === 0) {
    fail("negative control A: a GPL-3.0-only dependency was ACCEPTED by the deny policy");
  } else if (!gplResult.output.includes("GPL-3.0-only")) {
    fail("negative control A: cargo-deny failed, but the report does not name GPL-3.0-only");
    console.error(gplResult.output.slice(-2000));
  } else {
    pass("negative control A: third-party GPL-3.0-only dependency fails the deny policy");
  }

  // Control B: a foreign crate named like a first-party one passes cargo-deny
  // (exceptions match by name) and MUST fail the repository-path assertion.
  // The assertion runs inside the scratch lifecycle because cargo metadata
  // needs the manifest to still exist.
  const lookalike = withScratchWorkspace(
    "lookalike-negative",
    '[workspace]\nmembers = ["stub"]\nresolver = "3"\n',
    '[package]\nname = "saturn-core"\nversion = "0.0.0"\nedition = "2021"\nlicense = "AGPL-3.0-or-later"\n[lib]\npath = "lib.rs"\n',
    (dir) => {
      const cargoToml = path.join(dir, "Cargo.toml");
      const denyResult = run(
        "cargo",
        ["deny", "--manifest-path", cargoToml, "--config", DENY_CONFIG, "check", "licenses"],
        { cwd: dir },
      );
      try {
        assertFirstPartyAgpl({ name: "lookalike", cargoToml }, REPO, false, []);
        return { denyResult, assertionError: null };
      } catch (error) {
        return { denyResult, assertionError: String(error.message ?? error) };
      }
    },
  );
  if (lookalike.denyResult.status !== 0) {
    fail("negative control B: the lookalike scratch workspace unexpectedly failed cargo-deny (name exception gone?)");
    console.error(lookalike.denyResult.output.slice(-2000));
  }
  if (lookalike.assertionError && lookalike.assertionError.includes("outside the repository")) {
    pass("negative control B: foreign same-named crate is refused by the repository-path assertion");
  } else {
    fail(
      lookalike.assertionError
        ? `negative control B: unexpected assertion failure: ${lookalike.assertionError}`
        : "negative control B: the foreign saturn-core lookalike PASSED the repository-path assertion",
    );
  }
}

function parseJsLicence(expression) {
  return expression
    .replace(/[()]/g, " ")
    .split(/\s+OR\s+/i)
    .map((alternative) => alternative.split(/\s+AND\s+/i).map((id) => id.trim()).filter(Boolean));
}

function checkJsProduction() {
  const uiDir = path.join(REPO, "apps", "saturn-tauri");
  if (!fs.existsSync(path.join(uiDir, "node_modules"))) {
    if (REQUIRE_JS) {
      fail("JS gate: node_modules missing in apps/saturn-tauri and --require-js was given");
    } else {
      console.log("licence-policy: skip JS gate (node_modules absent; run pnpm install for full coverage)");
    }
    return;
  }
  const result = run("pnpm", ["licenses", "list", "--prod", "--json"], { cwd: uiDir });
  if (result.status !== 0) {
    fail(`JS gate: pnpm licenses list failed:\n${result.output.slice(-2000)}`);
    return;
  }
  const report = JSON.parse(result.output);
  let count = 0;
  for (const [reported, entries] of Object.entries(report)) {
    for (const entry of entries) {
      count += entry.versions.length;
      for (const alternative of parseJsLicence(reported)) {
        const unknown = alternative.filter((id) => !JS_ALLOWED.has(id));
        if (unknown.length) {
          fail(`JS gate: ${entry.name} declares "${reported}" with unapproved identifier(s): ${unknown.join(", ")}`);
        }
      }
    }
  }
  if (count === 0) {
    fail("JS gate: production dependency set is empty; refusing to pass vacuously");
    return;
  }
  if (!failed) pass(`JS gate: ${count} production packages report only allow-listed licences`);
}

if (SKIP_JS) {
  console.log("licence-policy: JS gate skipped by flag");
} else {
  checkJsProduction();
}
if (!SKIP_DENY) {
  checkDenoPolicy();
}
checkFirstPartyOwnership();
checkNegativeControls();

if (failed) {
  console.error("licence-policy: FAILED");
  process.exit(1);
}
console.log("licence-policy: all checks passed");
