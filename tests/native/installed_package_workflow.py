#!/usr/bin/env python3
"""Installed-package acceptance workflow (planning.md 6e E.1).

Proves the SHIPPED package can be installed/launched and complete a small
edit on this machine - the distribution.md rule: package build success is
not compatibility evidence. Drives the real desktop app over its local MCP
bridge (apps/saturn-tauri/src/mcp_bridge.rs):

  locate/download the alpha .deb
  -> package lint: control metadata, desktop file, icons, dependency check
  -> install [container: real dpkg install | local: extracted prefix]
  -> launch the desktop binary with an isolated HOME
  -> wait for the MCP bridge socket ($XDG_RUNTIME_DIR/easy-edits-pro-mcp.sock)
  -> drive an edit over the socket: import fixture, add + split clip, save
  -> re-inspect: the split survived in the live engine
  -> render an MP4 through the GUI job queue (render_start/render_status)
  -> verify the render with ffprobe (codec, duration, dimensions)
  -> quit the app, record the exit

Modes (this laptop is Arch - no dpkg - so the default mode extracts):
  default      extract the package data into a temp prefix and launch the
               binary from there. Proves package contents, launch, and the
               edit-to-render workflow. NOT a dependency-management proof:
               the PASS line says "extracted-prefix".
  --container  real install proof inside a debian:12 container (needs a
               running docker/podman daemon): apt-get install the .deb,
               verify every dependency resolves, launch under Xvfb, drive
               the same workflow, then ffprobe inside the container.
  --rpm-container  real dnf install inside fedora:latest (with --rpm):
               same proof on the RPM side; renders WebM/VP8 because Fedora
               free repositories ship no x264; probes via
               gst-discoverer-1.0.

Usage:
  python3 tests/native/installed_package_workflow.py [--deb PATH] [--container]
The work directory (printed in the PASS line) is always kept: it is the
acceptance evidence trail (lint extraction, saved project, rendered MP4,
app stderr).
Exit code 0 = the packaged app launched and completed the workflow.
"""

import argparse
import json
import math
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "apps" / "saturn-tauri" / "tests" / "fixtures" / "editor-smoke.webm"
MCP_SOCKET_RELATIVE = "easy-edits-pro-mcp.sock"
RENDER_TIMEOUT_SECONDS = 300
LAUNCH_TIMEOUT_SECONDS = 60


def fail(step, message):
    print(f"FAIL [{step}] {message}", file=sys.stderr)
    sys.exit(1)


def step(name):
    print(f"[pkg] {name}", flush=True)


def locate_deb(args) -> Path:
    if args.deb:
        deb = Path(args.deb)
        if not deb.is_file():
            fail("locate", f"--deb {deb} is not a file")
        return deb
    step("download newest alpha .deb from GitHub releases (no release is created)")
    result = subprocess.run(
        ["gh", "release", "download", "--repo", "zvirox/easy-edits-pro",
         "--pattern", "*.deb"],
        capture_output=True, text=True, cwd=str(args.work),
    )
    if result.returncode != 0:
        fail("locate", f"gh release download failed: {result.stderr[:500]}")
    debs = list(args.work.glob("*.deb"))
    if len(debs) != 1:
        fail("locate", f"expected exactly one .deb, got {[d.name for d in debs]}")
    return debs[0]


def deb_lint(deb: Path, work: Path) -> dict:
    """Inspect control metadata and the data inventory; return the facts the
    launch step needs (binary relative path, desktop file name)."""
    step("lint package: control metadata and file inventory")
    # ar-only inspection: control.tar.gz / data.tar.gz (dpkg is not required).
    result = subprocess.run(["ar", "t", str(deb)], capture_output=True, text=True)
    if result.returncode != 0 or "data.tar" not in result.stdout:
        fail("lint", f"not a deb archive: {result.stdout[:200]}")
    for member in ("control.tar.gz", "data.tar.gz"):
        if member not in result.stdout:
            fail("lint", f"missing {member} in {deb.name}")
    base = deb.name  # control fields from the filename the release ships
    data = work / "data"
    data.mkdir(exist_ok=True)
    # Extract control + data streams directly.
    for stream, dest in (("control.tar.gz", work / "control"), ("data.tar.gz", data)):
        dest.mkdir(exist_ok=True)
        extract = subprocess.Popen(
            ["ar", "p", str(deb), stream], stdout=subprocess.PIPE
        )
        untar = subprocess.run(["tar", "-xz", "-C", str(dest)], stdin=extract.stdout)
        extract.stdout.close()
        if untar.returncode != 0:
            fail("lint", f"could not unpack {stream}")
    control = work / "control" / "control"
    if not control.is_file():
        fail("lint", "control/control missing")
    fields = {}
    for line in control.read_text().splitlines():
        if ":" in line:
            key, _, value = line.partition(":")
            fields[key.strip()] = value.strip()
    package = fields.get("Package", "")
    if "easy" not in package.lower() and "saturn" not in package.lower():
        fail("lint", f"unexpected Package field: {package!r}")
    if not fields.get("Version"):
        fail("lint", "control has no Version")
    if fields.get("Architecture") != "amd64":
        fail("lint", f"unexpected Architecture: {fields.get('Architecture')!r}")
    binaries = sorted((data / "usr" / "bin").glob("*")) if (data / "usr" / "bin").is_dir() else []
    if not binaries:
        fail("lint", "no program in usr/bin")
    desktops = list((data / "usr" / "share" / "applications").glob("*.desktop"))
    if not desktops:
        fail("lint", "no . desktop entry shipped")
    desktop_text = desktops[0].read_text()
    exec_name = ""
    for line in desktop_text.splitlines():
        if line.startswith("Exec="):
            exec_name = line[5:].strip()
            break
    if not exec_name or not any(b.name == exec_name for b in binaries):
        fail("lint", f"desktop Exec={exec_name!r} does not match a packaged binary")
    icons = list((data / "usr" / "share" / "icons").rglob("*.png"))
    if not icons:
        fail("lint", "no hicolor icons shipped")
    validate = subprocess.run(
        ["desktop-file-validate", str(desktops[0])], capture_output=True, text=True
    )
    if validate.returncode != 0 or "error" in validate.stderr.lower():
        fail("lint", f"desktop-file-validate error on {desktops[0].name!r}: {validate.stderr.strip()[:300]}")
    allowed_roots = ("usr/bin", "usr/share/applications", "usr/share/icons",
                     "usr/lib/saturn-tauri", "usr/share/saturn-tauri",
                     "usr/share/metainfo")
    outside = sorted(
        str(p.relative_to(data)) for p in data.rglob("*")
        if p.is_file() and not str(p.relative_to(data)).startswith(allowed_roots)
    )
    if outside:
        fail("lint", f"{len(outside)} file(s) outside app-private prefixes: {outside[:5]}")
    missing = subprocess.run(
        ["ldd", str(next(b for b in binaries if b.is_file() and not b.is_symlink()))],
        capture_output=True, text=True,
    )
    unresolved = [ln for ln in missing.stdout.splitlines() if "not found" in ln]
    if unresolved:
        print(f"[pkg] note: host cannot resolve {len(unresolved)} shared libs "
              f"(informational on non-Debian hosts; container mode enforces zero): "
              f"{unresolved[:3]}")
    return {
        "package": package,
        "version": fields["Version"],
        "binary": exec_name,
        "data_root": data,
        "source": deb.name,
        "unresolved_host_libs": len(unresolved),
    }


def pick_container_cmd() -> list:
    for candidate in ("docker", "podman"):
        path = shutil.which(candidate)
        if path:
            check = subprocess.run(
                [path, "info", "--format", "{{.ServerVersion}}"],
                capture_output=True, text=True, timeout=20,
            )
            if check.returncode == 0 and check.stdout.strip():
                return [path]
    fail("container", "no running docker/podman daemon; start one or use default (extracted-prefix) mode")


def container_mode(deb: Path, work: Path):
    """Real dpkg install + launch inside debian:12, driven over the bridge."""
    step("container: real apt install of the package")
    cmd = pick_container_cmd()
    run_in = [
        *cmd, "run", "--rm",
        "-v", f"{deb}:/pkg/app.deb:ro",
        "-v", f"{FIXTURE}:/fixture/editor-smoke.webm:ro",
        "-v", f"{REPO_ROOT}/tests/native/container_driver.py:/driver.py:ro",
        "debian:12", "bash", "-lc",
        "export DEBIAN_FRONTEND=noninteractive; "
        "apt-get update -qq && apt-get install -y -qq "
        "libwebkit2gtk-4.1-0 libxdo3 libayatana-appindicator3-1 "
        "libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 "
        "libgstreamer-plugins-bad1.0-0 libges-1.0-0 gstreamer1.0-plugins-good "
        "gstreamer1.0-plugins-ugly gstreamer1.0-libav xvfb x11-utils "
        "gstreamer1.0-tools ffmpeg python3-minimal python3 >/dev/null && "
        "apt-get install -y -qq /pkg/app.deb && "
        "ldd /usr/bin/saturn-tauri | tee /tmp/ldd.txt; "
        "if grep -q 'not found' /tmp/ldd.txt; then echo 'FAIL unresolved libs'; exit 3; fi; "
        "Xvfb :99 -screen 0 1280x800x24 & "
        "sleep 2; DISPLAY=:99 WEBKIT_DISABLE_DMABUF_RENDERER=1 "
        "LIBGL_ALWAYS_SOFTWARE=1 HOME=/root python3 /driver.py --binary /usr/bin/saturn-tauri "
        "--fixture /fixture/editor-smoke.webm --work /root/work",
    ]
    result = subprocess.run(run_in, capture_output=True, text=True, timeout=1800)
    print(result.stdout[-3000:])
    if result.returncode != 0:
        fail("container", f"exit {result.returncode}: {result.stderr[-2000:]}")
    if "[pkg] PASS" not in result.stdout:
        fail("container", "driver did not report PASS")
    print(f"[pkg] PASS - container install ({deb.name})")
    return 0


def rpm_container_mode(rpm: Path, work: Path):
    """Real dnf install + launch inside fedora:latest, driven over the bridge.

    Fedora free repos ship no x264/libav, so the render target is
    WebM/VP8 (gstreamer1-plugins-good: vpx + vorbisenc + webmmux) and the
    output probe falls back to gst-discoverer-1.0 (package names verified
    against packages.fedoraproject.org on 2026-10-27: gst-editing-services,
    gstreamer1-plugins-base-tools carries gst-discoverer-1.0).
    """
    step("rpm container: real dnf install of the package")
    cmd = pick_container_cmd()
    run_in = [
        *cmd, "run", "--rm",
        "-v", f"{rpm}:/pkg/app.rpm:ro",
        "-v", f"{FIXTURE}:/fixture/editor-smoke.webm:ro",
        "-v", f"{REPO_ROOT}/tests/native/container_driver.py:/driver.py:ro",
        "fedora:latest", "bash", "-lc",
        "set -e; "
        "dnf install -y --setopt=install_weak_deps=False -q "
        "webkit2gtk4.1 libxdo libayatana-appindicator-gtk3 "
        "gstreamer1 gstreamer1-plugins-base gstreamer1-plugins-base-tools "
        "gstreamer1-plugins-good gstreamer1-plugins-bad-free "
        "gst-editing-services xorg-x11-server-Xvfb python3 >/dev/null && "
        "dnf install -y -q /pkg/app.rpm >/dev/null && "
        "test -x /usr/bin/saturn-tauri && "
        "ldd /usr/bin/saturn-tauri | tee /tmp/ldd.txt; "
        "if grep -q 'not found' /tmp/ldd.txt; then echo 'FAIL unresolved libs'; exit 3; fi; "
        "Xvfb :99 -screen 0 1280x800x24 & "
        "sleep 2; DISPLAY=:99 WEBKIT_DISABLE_DMABUF_RENDERER=1 "
        "LIBGL_ALWAYS_SOFTWARE=1 HOME=/root python3 /driver.py --binary /usr/bin/saturn-tauri "
        "--fixture /fixture/editor-smoke.webm --work /root/work --format webm",
    ]
    result = subprocess.run(run_in, capture_output=True, text=True, timeout=2400)
    print(result.stdout[-3000:])
    if result.returncode != 0:
        fail("rpm-container", f"exit {result.returncode}: {result.stderr[-2000:]}")
    if "[pkg] PASS" not in result.stdout:
        fail("rpm-container", "driver did not report PASS")
    print(f"[pkg] PASS - rpm container install ({rpm.name})")
    return 0


def bridge_call(sock_path: Path, method: str, params=None, timeout=30):
    """One JSONL request over the app's local MCP bridge; return result."""
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(timeout)
    try:
        client.connect(str(sock_path))
        request = {"method": method}
        if params is not None:
            request["params"] = params
        client.sendall(json.dumps(request).encode() + b"\n")
        buffer = bytearray()
        while not buffer.endswith(b"\n"):
            count = client.recv(65536)
            if not count:
                fail("bridge", f"socket closed while waiting for {method}")
            buffer.extend(count)
        response = json.loads(bytes(buffer))
    finally:
        client.close()
    if "error" in response:
        fail("bridge", f"{method} error: {response['error']}")
    return response.get("result")


def rpm_lint(rpm: Path, work: Path) -> dict:
    """Lint the .rpm without rpm tooling (bsdtar/libarchive reads the header
    and payload): metadata sanity from the header tag area, payload inventory,
    desktop validation, and a file-conflict scan. The rpm still has no
    recorded *install* result - this is the E.5 lint gate only.
    """
    step("rpm lint: payload inventory via bsdtar, desktop + conflict checks")
    listing = subprocess.run(["bsdtar", "-tf", str(rpm)], capture_output=True, text=True)
    if listing.returncode != 0:
        fail("rpm-lint", f"bsdtar could not read {rpm.name}: {listing.stderr[:300]}")
    entries = [ln[2:] if ln.startswith("./") else ln for ln in listing.stdout.splitlines() if ln.strip()]
    binaries = [p for p in entries if p.startswith("usr/bin/") and not p.endswith("/")]
    if not binaries:
        fail("rpm-lint", f"no program in usr/bin: {entries[:8]}")
    desktops = [p for p in entries if p.startswith("usr/share/applications/") and p.endswith(".desktop")]
    if not desktops:
        fail("rpm-lint", f"no desktop entry shipped: {entries[:8]}")
    icons = [p for p in entries if p.startswith("usr/share/icons/") and p.endswith(".png")]
    if not icons:
        fail("rpm-lint", "no hicolor icons shipped")

    payload = work / "rpm-data"
    payload.mkdir(exist_ok=True)
    extract = subprocess.run(["bsdtar", "-xf", str(rpm), "-C", str(payload)],
                             capture_output=True, text=True)
    if extract.returncode != 0:
        fail("rpm-lint", f"could not unpack payload: {extract.stderr[:300]}")

    desktop = payload / desktops[0]
    validate = subprocess.run(["desktop-file-validate", str(desktop)], capture_output=True, text=True)
    if validate.returncode != 0 or "error" in validate.stderr.lower():
        fail("rpm-lint", f"desktop-file-validate error on {desktops[0]!r}: {validate.stderr.strip()[:300]}")

    # File-conflict scan: everything must live under app-private prefixes.
    # Anything outside these roots can collide with files a distro package owns.
    allowed_roots = ("usr/bin", "usr/share/applications", "usr/share/icons",
                     "usr/lib/saturn-tauri", "usr/share/saturn-tauri",
                     "usr/share/metainfo")
    outside = [p for p in entries if p and not p.endswith("/") and not p.startswith(allowed_roots)]
    if outside:
        fail("rpm-lint", f"{len(outside)} file(s) outside app-private prefixes: {outside[:5]}")

    exec_name = ""
    for line in desktop.read_text().splitlines():
        if line.startswith("Exec="):
            exec_name = line[5:].strip().split(" ")[0]
            break
    if not exec_name or f"usr/bin/{exec_name}" not in binaries:
        fail("rpm-lint", f"desktop Exec={exec_name!r} does not match a packaged binary")

    warnings = []
    if " " in desktops[0]:
        warnings.append(f"desktop file name contains a space: {desktops[0]!r}")
    metainfo = [p for p in entries if p.startswith("usr/share/metainfo/")]
    if not metainfo:
        warnings.append("no appstream metainfo in the payload (appstore metadata gap)")
    binary = payload / binaries[0]
    ldd = subprocess.run(["ldd", str(binary)], capture_output=True, text=True)
    unresolved = [ln for ln in ldd.stdout.splitlines() if "not found" in ln]
    if unresolved:
        warnings.append(f"{len(unresolved)} host-unresolved libs (informational on a non-RPM host; the deb lint records the same caveat)")
    print(f"[pkg] rpm warnings: {warnings if warnings else 'none'}")
    return {
        "payload_files": len(entries),
        "binaries": binaries,
        "desktop": desktops[0],
        "icons": len(icons),
        "conflicts": outside,
        "warnings": warnings,
        "source": rpm.name,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--deb", help="path to a .deb (default: newest alpha release)")
    parser.add_argument("--rpm", help="path to a .rpm; run the RPM lint pass only (the rpm cannot be installed on this host)")
    parser.add_argument("--container", action="store_true",
                        help="real dpkg install inside debian:12 (needs a container daemon)")
    parser.add_argument("--rpm-container", action="store_true",
                        help="real dnf install inside fedora:latest (needs --rpm; "
                        "renders WebM/VP8 - Fedora free repos have no x264)")
    parser.add_argument("--flatpak", action="store_true",
                        help="E.5 spike: launch the installed flatpak and drive edit-to-render")
    parser.add_argument("--flatpak-env", action="append", default=[],
                        help="extra VAR=VALUE passed into the sandbox (repeatable), e.g. GST_DEBUG=...")
    parser.add_argument("--work", default=None, help="reuse a work directory")
    args = parser.parse_args()
    if args.work:
        args.work = Path(args.work)
        args.work.mkdir(parents=True, exist_ok=True)
    else:
        args.work = Path(tempfile.mkdtemp(prefix="eep-pkg-"))
    work = args.work

    if args.flatpak:
        return flatpak_mode(args)

    if args.rpm:
        rpm = Path(args.rpm)
        if not rpm.is_file():
            fail("locate", f"--rpm {rpm} is not a file")
        if args.rpm_container:
            return rpm_container_mode(rpm, work)
        result = rpm_lint(rpm, work)
        print(json.dumps(result, indent=2))
        step("rpm lint complete (no install/launch evidence recorded; see delivery matrix)")
        return

    deb = locate_deb(args)
    info = deb_lint(deb, work)
    print(f"[pkg] {info['package']} {info['version']} ({info['source']}): "
          f"usr/bin/{info['binary']}, desktop entry + {len(list((work / 'data' / 'usr' / 'share' / 'icons').rglob('*.png')))} icons")

    if args.container:
        return container_mode(deb, work)

    # --- Extracted-prefix launch (default on this Arch laptop) --------------
    step("launch extracted desktop binary with isolated HOME")
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR")
    if not runtime_dir:
        fail("environment", "XDG_RUNTIME_DIR is not set; launch the test from a desktop session")
    sock_path = Path(runtime_dir) / MCP_SOCKET_RELATIVE
    def socket_alive(path: Path) -> bool:
        return path.exists() and socket.socket(
            socket.AF_UNIX, socket.SOCK_STREAM
        ).connect_ex(str(path)) == 0

    if socket_alive(sock_path):
        fail("environment", f"a live editor owns {sock_path}; close the running Easy Edits Pro first")
    if sock_path.exists():
        sock_path.unlink()

    home = work / "home"
    home.mkdir(exist_ok=True)
    binary = work / "data" / "usr" / "bin" / info["binary"]
    env = dict(os.environ)
    env.update({
        "HOME": str(home),
        "XDG_DATA_HOME": str(home / ".local" / "share"),
        "XDG_CONFIG_HOME": str(home / ".config"),
        "XDG_CACHE_HOME": str(home / ".cache"),
    })
    if not FIXTURE.is_file():
        fail("fixture", f"missing maintained fixture: {FIXTURE}")
    stderr_log = (work / "app-stderr.log").open("wb")
    app = subprocess.Popen(
        [str(binary)],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=stderr_log,
    )
    try:
        # Wait for the MCP bridge to bind (GUI needs a few seconds to boot).
        deadline = time.time() + LAUNCH_TIMEOUT_SECONDS
        while time.time() < deadline:
            if app.poll() is not None:
                fail("launch", f"app exited early with {app.returncode}; see {work / 'app-stderr.log'}")
            if sock_path.exists():
                break
            time.sleep(0.5)
        else:
            fail("launch", f"MCP socket never appeared at {sock_path}")
        if socket.socket(socket.AF_UNIX, socket.SOCK_STREAM).connect_ex(str(sock_path)) != 0:
            fail("launch", f"socket exists but refuses connections: {sock_path}")
        print(f"[pkg] bridge listening at {sock_path}")
        summary = drive_edit_to_render(sock_path, work, app)
    finally:
        if app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=15)
            except subprocess.Popen.TimeoutExpired:
                app.kill()
                app.wait(timeout=10)
        stderr_log.close()

    print(f"[pkg] app exit code after SIGTERM: {app.returncode}")
    # The work dir is kept: it is the evidence trail (lint output, saved
    # project, rendered MP4, app stderr) for the acceptance record.
    fixture_media = summary["fixture_media"]
    print(f"[pkg] PASS - extracted-prefix launch of {deb.name}: "
          f"h264 {fixture_media['width']}x{fixture_media['height']}, "
          f"{summary['output_seconds']:.2f}s, "
          f"{summary['output_path'].stat().st_size} bytes, work dir {work}")
    return 0


def flatpak_mode(args) -> int:
    """E.5 Flatpak spike: launch the installed app and run the identical
    edit-to-render drive (fixture import, split, save, GUI-queue render,
    ffprobe). The sandbox provides WebKitGTK/GStreamer; the host provides
    nothing except the flatpak session. Skips the deb extraction entirely."""
    app_id = "io.github.zvirox.EasyEditsPro"
    check = subprocess.run(["flatpak", "info", "--user", app_id],
                           capture_output=True, text=True)
    if check.returncode != 0:
        fail("flatpak", f"{app_id} is not installed for this user: "
                        f"{check.stderr.strip()[:200]}")
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR")
    if not runtime_dir:
        fail("environment", "XDG_RUNTIME_DIR is not set; launch the test from a desktop session")
    sock_path = Path(runtime_dir) / MCP_SOCKET_RELATIVE
    sock_path.unlink(missing_ok=True)
    work = args.work
    if not FIXTURE.is_file():
        fail("fixture", f"missing maintained fixture: {FIXTURE}")
    stderr_log = (work / "flatpak-stderr.log").open("wb")
    step(f"launch installed flatpak {app_id} (sandboxed HOME)")
    launch_cmd = ["flatpak", "run"]
    for pair in getattr(args, "flatpak_env", []):
        if "=" in pair:
            launch_cmd.append(f"--env={pair}")
    launch_cmd.append(app_id)
    app = subprocess.Popen(
        launch_cmd,
        stdout=subprocess.DEVNULL,
        stderr=stderr_log,
    )
    try:
        deadline = time.time() + LAUNCH_TIMEOUT_SECONDS
        while time.time() < deadline:
            if app.poll() is not None:
                fail("launch", f"flatpak run exited early with {app.returncode}; "
                               f"see {work / 'flatpak-stderr.log'}")
            if sock_path.exists():
                break
            time.sleep(0.5)
        else:
            fail("launch", f"MCP socket never appeared at {sock_path}; the manifest "
                           f"needs --filesystem=xdg-run/easy-edits-pro-mcp.sock and "
                           f"--share=network")
        print(f"[pkg] bridge listening at {sock_path}")
        # Save/render inside the sandbox-visible home; driver artifacts
        # (logs, harness files) stay in the work dir.
        io_root = Path.home() / ".eep-flatpak-spike"
        io_root.mkdir(exist_ok=True)
        summary = drive_edit_to_render(sock_path, work, app, io_root=io_root)
    finally:
        subprocess.run(["flatpak", "kill", app_id], capture_output=True)
        try:
            app.wait(timeout=15)
        except subprocess.Popen.TimeoutExpired:
            app.kill()
            app.wait(timeout=10)
        stderr_log.close()
    output_path = summary["output_path"]
    print(f"[pkg] PASS - flatpak spike of {app_id}: h264 render verified, "
          f"{summary['output_seconds']:.2f}s, {output_path.stat().st_size} bytes, "
          f"work dir {work}")
    return 0

def drive_edit_to_render(sock_path, work: Path, app: subprocess.Popen,
                         io_root: Path | None = None) -> dict:
    """The shared acceptance drive: doc inspect, fixture import, timeline
    split, save, render through the GUI job queue, ffprobe verification.
    Used verbatim by the extracted-prefix run and the Flatpak spike.
    io_root overrides where the project + render land (the flatpak sandbox
    can only write where the manifest exposes a filesystem permission)."""
    io_root = io_root or work
    # Launch sanity: the app reports a fresh document.
    step("doc_inspect on the fresh session")
    doc = bridge_call(sock_path, "doc_inspect")
    if doc.get("is_dirty") is None:
        fail("inspect", f"unexpected doc_inspect shape: {list(doc)}")

    # Drive a small edit through shipped engine commands.
    step("import fixture through command_run (ffprobe-derived metadata)")
    ffprobe = subprocess.run([
        "ffprobe", "-v", "error", "-print_format", "json",
        "-show_streams", "-show_format", str(FIXTURE),
    ], capture_output=True, text=True)
    if ffprobe.returncode != 0:
        fail("fixture", f"ffprobe failed: {ffprobe.stderr[:300]}")
    probe = json.loads(ffprobe.stdout)
    video = next(s for s in probe["streams"] if s.get("codec_type") == "video")
    seconds = float(probe["format"]["duration"])
    ticks = lambda s: round(s * 254_016_000_000)
    numerator, _, denominator = video["avg_frame_rate"].partition("/")
    bridge_call(sock_path, "command_run", {
        "id": "project.new", "params": {"name": "Package Acceptance"},
    })
    doc = bridge_call(sock_path, "doc_inspect")
    settings = doc["project"]["project"]["settings"]
    bridge_call(sock_path, "command_run", {
        "id": "project.add_media",
        "params": {
            "name": FIXTURE.name, "path": str(FIXTURE), "kind": "video",
            "duration": ticks(seconds), "width": video["width"],
            "height": video["height"],
            "fps": round(int(numerator) / float(denominator or 1), 6),
            "codec": video.get("codec_name", "vp8"),
        },
    })
    doc = bridge_call(sock_path, "doc_inspect")
    project = doc["project"]["project"]
    media_id = project["media"][0]["id"]
    video_track = next(t for t in project["sequence"]["tracks"] if t["kind"] == "video")

    step("timeline edit: add clip, split at midpoint, save through the app")
    bridge_call(sock_path, "command_run", {
        "id": "timeline.add_clip",
        "params": {"track_id": video_track["id"], "media_id": media_id},
    })
    bridge_call(sock_path, "command_run", {
        "id": "timeline.split_clip",
        "params": {
            "track_id": video_track["id"], "clip_index": 0,
            "at": ticks(seconds / 2),
        },
    })
    save_path = io_root / "package-edit.saturn"
    bridge_call(sock_path, "command_run", {
        "id": "file.save", "params": {"path": str(save_path)},
    })
    if not save_path.is_file():
        fail("save", f"file.save reported ok but {save_path} is missing")

    sequence = bridge_call(sock_path, "sequence_inspect")
    saved_track = next(t for t in sequence["tracks"] if t["kind"] == "video")
    if len(saved_track["clips"]) != 2:
        fail("verify", f"expected 2 clips after split, got {len(saved_track['clips'])}")

    # Render through the GUI's own job queue, then ffprobe the output.
    step("render MP4 through the GUI job queue (render_start)")
    output_path = io_root / "package-out.mp4"
    started = bridge_call(sock_path, "render_start", {
        "output_path": str(output_path),
        "range": {"start_ticks": 0, "end_ticks": ticks(seconds)},
    })
    job_id = started["job_id"]
    deadline = time.time() + RENDER_TIMEOUT_SECONDS
    last_report = 0.0
    while time.time() < deadline:
        status = bridge_call(sock_path, "render_status", {"job_id": job_id})
        state = status.get("state")
        now = time.time()
        if now - last_report > 15:
            last_report = now
            print(f"[pkg] render status after {int(now - (deadline - RENDER_TIMEOUT_SECONDS))}s: {json.dumps(state)[:200]}", flush=True)
        if isinstance(state, dict):
            # JobState serializes as {"status": "running"|"completed"|...
            # with "phase"/"progress" while running, "output" when done}.
            tag = state.get("status")
            if tag == "completed":
                break
            if tag == "failed":
                fail("render", f"render job failed: {state}")
        elif state == "failed":
            fail("render", f"render job failed: {status}")
        if app.poll() is not None:
            fail("render", f"app exited during render with {app.returncode}")
        time.sleep(2)
    else:
        fail("render", f"render job {job_id} did not finish in {RENDER_TIMEOUT_SECONDS}s")
    if not output_path.is_file():
        fail("render", f"render completed without output at {output_path}")

    step("verify render output with ffprobe")
    probed = json.loads(subprocess.run(
        ["ffprobe", "-v", "error", "-print_format", "json",
         "-show_streams", "-show_format", str(output_path)],
        capture_output=True, text=True).stdout)
    vstream = next(s for s in probed["streams"] if s.get("codec_type") == "video")
    if vstream.get("codec_name") != "h264":
        fail("ffprobe", f"expected h264, got {vstream.get('codec_name')}")
    out_seconds = float(probed["format"]["duration"])
    if not math.isclose(out_seconds, seconds, abs_tol=0.6):
        fail("ffprobe", f"duration drifted: {out_seconds:.2f}s vs source {seconds:.2f}s")
    # The render honors the project's settings, not the source media size
    # (an upscaled fixture is expected).
    if vstream["width"] != settings["width"] or vstream["height"] != settings["height"]:
        fail("ffprobe", f"output is {vstream['width']}x{vstream['height']}, "
                        f"project settings are {settings['width']}x{settings['height']}")
    return {
        "fixture_media": {"width": video["width"], "height": video["height"]},
        "output_seconds": out_seconds,
        "output_path": output_path,
    }


if __name__ == "__main__":
    sys.exit(main())
