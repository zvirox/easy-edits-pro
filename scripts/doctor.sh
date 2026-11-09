#!/usr/bin/env bash

# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
# Checks that the tools and system libraries needed to build Easy Edits Pro are
# present. Read-only: it installs nothing. Usage: scripts/doctor.sh
set -u
ok=0; bad=0
pass() { printf '  [ok]   %s\n' "$1"; ok=$((ok+1)); }
fail() { printf '  [MISS] %s  -> %s\n' "$1" "$2"; bad=$((bad+1)); }

have() { command -v "$1" >/dev/null 2>&1; }
ver_ge() { [ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -n1)" = "$2" ]; }

echo "Toolchain"
if have rustc; then
  v=$(rustc --version | awk '{print $2}')
  if ver_ge "$v" "1.84.0"; then pass "rustc $v"; else fail "rustc $v (need >= 1.84)" "rustup update stable"; fi
else fail "rustc" "install from https://rustup.rs"; fi
have cargo && pass "cargo" || fail "cargo" "install from https://rustup.rs"
if have node; then
  major=$(node -p 'process.versions.node.split(".")[0]')
  if [ "$major" -ge 20 ]; then pass "node $(node -v)"; else fail "node $(node -v) (need >= 20)" "install Node 20 or 22"; fi
else fail "node" "install Node 20 or 22"; fi
have pnpm && pass "pnpm $(pnpm -v)" || fail "pnpm" "corepack enable && corepack prepare pnpm@latest --activate"
have git && pass "git" || fail "git" "install git"

echo "System libraries (pkg-config)"
if have pkg-config; then
  for lib in gstreamer-1.0 gstreamer-base-1.0 gstreamer-plugins-base-1.0 gst-editing-services-1.0 webkit2gtk-4.1 gtk+-3.0 librsvg-2.0; do
    if pkg-config --exists "$lib" 2>/dev/null; then pass "$lib $(pkg-config --modversion "$lib")"; else fail "$lib" "see Development Prerequisites in CONTRIBUTING.md for your distribution"; fi
  done
else fail "pkg-config" "install pkg-config"; fi

echo "GStreamer runtime"
if have gst-inspect-1.0; then
  for el in uridecodebin videoconvert audioconvert; do
    gst-inspect-1.0 "$el" >/dev/null 2>&1 && pass "element $el" || fail "element $el" "install gstreamer plugins-base/good"
  done
else fail "gst-inspect-1.0" "install gstreamer1.0-tools"; fi

echo
if [ "$bad" -eq 0 ]; then
  echo "All $ok checks passed. Next: cargo test --workspace"
else
  echo "$bad problem(s), $ok ok. Fix the lines marked MISS and run this again."
  exit 1
fi
