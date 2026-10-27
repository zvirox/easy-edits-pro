#!/usr/bin/env bash
# Build the payload directory the Flatpak manifest consumes: the packaged
# application from the newest (or given) GitHub alpha release .deb.
# No dpkg needed: ar + tar are the only tools (same policy as the acceptance
# runbook). Output: THIS_DIR/payload/usr (cleaned first).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="${1:-$HERE/payload}"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

if [[ -n "${DEB:-}" ]]; then
  cp -- "$DEB" "$STAGE/app.deb"
else
  echo "downloading newest alpha .deb from GitHub releases"
  gh release download --repo zvirox/easy-edits-pro \
    --pattern '*.deb' --output "$STAGE/app.deb"
fi

rm -rf "$WORK"
mkdir -p "$WORK"
(cd "$WORK" && ar p "$STAGE/app.deb" data.tar.gz | tar -xz)
echo "payload ready: $WORK"
