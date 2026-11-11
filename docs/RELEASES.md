# Release Workflow & Standard Pattern

This document defines the standard procedure that **MUST** be followed every time a release tag is created and pushed for **Easy Edits Pro**.

---

## 1. Versioning & Tagging Conventions

- **Alpha Releases**: `v0.1.X-alpha.Y` (e.g. `v0.1.2-alpha.6`)
- **Beta Releases**: `v0.1.X-beta.Y`
- **Stable Releases**: `vX.Y.Z`

Tags must always be annotated git tags with the `v` prefix matching Semantic Versioning (`^v[0-9]+\.[0-9]+\.[0-9]+(-[a-z]+(\.[0-9]+)?)?$`).

---

## 2. Mandatory Pre-Release Verification Gates

Before cutting or pushing any release tag, all repository gates **must** pass cleanly with zero errors:

```bash
# 1. Rust engine and workspace tests
cargo test --workspace

# 2. Command catalogue & parity synchronization
pnpm --dir apps/saturn-tauri catalogue:sync
pnpm --dir apps/saturn-tauri check:parity
node apps/saturn-tauri/scripts/check-parity.mjs --check

# 3. Design token, glass surface & contrast enforcement
pnpm --dir apps/saturn-tauri check:glass

# 4. Command catalogue & menu alignment
pnpm --dir apps/saturn-tauri check:commands

# 5. Source file line limit (<= 800 lines)
pnpm --dir apps/saturn-tauri check:source-size --max-lines=800

# 6. TypeScript compilation & Vite production build
pnpm --dir apps/saturn-tauri build

# 7. Frontend unit tests
pnpm --dir apps/saturn-tauri test

# 8. Browser Playwright E2E contract suite
pnpm --dir apps/saturn-tauri test:e2e
```

Never push a release tag if any gate is failing.

---

## 3. Step-by-Step Release Procedure

### Step 1: Update Documentation & Changelog
1. Update `CHANGELOG.md` moving relevant items from `[Unreleased]` into a new section:
   ```markdown
   ## [0.1.2-alpha.X] - YYYY-MM-DD
   ### Added
   ...
   ### Changed
   ...
   ### Fixed
   ...
   ```
2. Verify that generated files (`docs/COMMANDS.md`, `docs/reference/command-catalogue.json`, `docs/PARITY.md`, `docs/glass-tokens.md`) are up to date.
3. Commit documentation updates to `main` and push:
   ```bash
   git add CHANGELOG.md docs/
   git commit -m "docs: prepare release v0.1.2-alpha.X"
   git push origin main
   ```

### Step 2: Create and Push the Tag
```bash
git tag -a v0.1.2-alpha.X -m "Release v0.1.2-alpha.X"
git push origin v0.1.2-alpha.X
```

### Step 3: CI Automation & Artifact Packaging
Pushing the tag triggers `.github/workflows/linux-packages.yml` on GitHub Actions:
- Injects version string into `apps/saturn-tauri/tauri.conf.json`.
- Builds Debian package (`.deb`) and RPM package (`.rpm`).
- Generates `SHA256SUMS`.
- Compiles the Flatpak bundle (`easy-edits-pro-linux-x86_64.flatpak`).
- Runs acceptance tests (`tests/native/installed_package_workflow.py`) verifying in-container installation and edit-to-render proof.
- Automatically attaches all packages and checksums to the GitHub release.

### Step 4: Publish Standard Release Notes on GitHub
Every release on GitHub must contain the following standardized sections:
1. **Overview / Headline Summary**: Brief 2–3 sentence description of the milestone.
2. **New Additions / Features**: Key capabilities added since previous release.
3. **Fixes & Hardening**: Bugs patched, security/origin fences added, or parity drifts resolved.
4. **Changes & Deprecations**: Architectural updates, schema migrations, or token adjustments.
5. **Measured Verification Evidence**: Summary of passing gates, test coverage, and benchmarks.
6. **Package Checksums**: SHA-256 hashes of attached binary artifacts.
