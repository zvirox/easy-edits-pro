#!/usr/bin/env bash

# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
# Tauri `beforeBundleCommand` hook: builds everything the deb/rpm bundle
# consumes beyond the application binary:
#   1. the release-build saturn-plugin-worker binary (ships at /usr/bin, the
#      sibling lookup target of crates/saturn-audio-dsp/src/sandbox.rs);
#   2. the third-party notice bundle (scripts/generate-notice-bundle.py)
#      staged at target/third-party-notices.
# Paths are resolved from this script's own location, so the hook works for
# any invocation cwd. The steps are incremental; cargo build reuses the
# release profile produced by `tauri build` itself.

set -euo pipefail
SCRIPTS="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$SCRIPTS")"

cargo build --locked --release --manifest-path "$REPO/Cargo.toml" \
  -p saturn-plugin-worker

WORKER="$REPO/target/release/saturn-plugin-worker"
test -x "$WORKER"
STAGE="$REPO/target/third-party-notices"
python3 "$SCRIPTS/generate-notice-bundle.py" --out "$STAGE"

# The bundle must describe exactly what the package will contain; refuse to
# bundle when a mandated gap is still unresolved and the bundle is thin.
test -s "$STAGE/NOTICE-DEPENDENCIES.md"
test -s "$STAGE/inventory.json"
count=$(ls "$STAGE/texts" 2>/dev/null | wc -l)
if [ "$count" -lt 400 ]; then
  echo "prepare-bundle-inputs: notice bundle is unusually thin: $count text dirs" >&2
  exit 1
fi
echo "prepare-bundle-inputs: worker + notice bundle staged" \
  "($WORKER, $count text directories)"
