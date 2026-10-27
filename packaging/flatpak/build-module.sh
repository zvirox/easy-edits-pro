#!/usr/bin/env bash
# Flatpak module build: install the packaged binary from the release payload
# and re-namespace the desktop assets under the flatpak app id. Runs INSIDE
# the flatpak-builder build sandbox; the dir source has copied the payload
# contents to the build root (usr/), and the metainfo file sits at the root.
set -euo pipefail

test -x usr/bin/saturn-tauri
test -f "usr/share/applications/Easy Edits Pro.desktop"

# One dynamic WebKitGTK/GStreamer client binary; the runtime provides the libs.
install -Dm755 usr/bin/saturn-tauri -t /app/bin/

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

echo "module build complete: /app/bin/saturn-tauri + desktop + metainfo + 4 icons"
