#!/usr/bin/env bash

# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
# Flatpak module build: install the packaged binary from the release payload
# and re-namespace the desktop assets under the flatpak app id. Runs INSIDE
# the flatpak-builder build sandbox; the dir source has copied the payload
# contents to the build root (usr/), and the metainfo file sits at the root.
set -euo pipefail

test -x usr/bin/saturn-tauri
test -x usr/bin/saturn-plugin-worker
test -f "usr/share/applications/Easy Edits Pro.desktop"

# One dynamic WebKitGTK/GStreamer client binary; the runtime provides the libs.
install -Dm755 usr/bin/saturn-tauri -t /app/bin/

# Plugin worker ships beside the app (crates/saturn-audio-dsp/src/sandbox.rs
# resolves saturn-plugin-worker as a sibling of the executable).
install -Dm755 usr/bin/saturn-plugin-worker -t /app/bin/

# Generated third-party notice bundle + licence text, from the deb payload.
test -f usr/share/doc/easy-edits-pro/third-party-notices/NOTICE-DEPENDENCIES.md
test -s usr/share/doc/easy-edits-pro/third-party-notices/inventory.json
test -f usr/share/doc/easy-edits-pro/copyright
mkdir -p /app/share/doc
cp -a usr/share/doc/easy-edits-pro /app/share/doc/

# Flatpak re-namespaces the desktop file and icon under the app id; a clean id
# filename also fixes the "desktop file name contains a space" lint warning.
sed -e 's/^Icon=saturn-tauri$/Icon=io.github.zvirox.EasyEditsPro/' \
    "usr/share/applications/Easy Edits Pro.desktop" \
    > io.github.zvirox.EasyEditsPro.desktop
install -Dm644 io.github.zvirox.EasyEditsPro.desktop -t /app/share/applications/
install -Dm644 io.github.zvirox.EasyEditsPro.metainfo.xml -t /app/share/metainfo/

for size in 32x32 64x64 128x128 256x256@2; do
  install -Dm644 "usr/share/icons/hicolor/$size/apps/saturn-tauri.png" \
    "/app/share/icons/hicolor/$size/apps/io.github.zvirox.EasyEditsPro.png"
done

echo "module build complete: /app/bin/saturn-tauri + saturn-plugin-worker + notices + desktop + metainfo + 4 icons"
