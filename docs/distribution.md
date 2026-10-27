# Distribution and releases

**Updated:** 2026-10-23

## Current release evidence

[Easy Edits Pro v0.1.0-alpha.1](https://github.com/zvirox/easy-edits-pro/releases/tag/v0.1.0-alpha.1)
is the first public alpha. The release workflow successfully built and attached:

- an x86_64 Debian package (.deb),
- an x86_64 RPM package (.rpm), and
- SHA-256 checksums.

These artifacts are experimental alpha packages. They prove that the GitHub
release workflow can build and publish packages; they do **not** prove that an
installed package launches or completes an edit on another system.

## Delivery policy

- The development laptop is the only controlled verification environment.
- Software H.264 export is the verified local render path. Hardware encoding
  remains optional and unverified.
- Flatpak is the intended supported delivery route because it can bundle known
  WebKitGTK and GStreamer versions. A Flatpak manifest, CI artifact, and
  install-and-launch result do not yet exist, so it is not a supported artifact
  today.
- AppImage is not part of the current delivery plan.
- Future alpha and beta releases gather compatibility feedback through issue
  reports. The project does not maintain a planned external hardware matrix.

## Publishing an alpha

Pushing a semantic alpha tag starts the Linux package workflow:

~~~bash
git tag v0.1.0-alpha.2
git push origin v0.1.0-alpha.2
~~~

The workflow creates a prerelease and uploads its packages plus checksums. Before
creating a tag, the commit must have a green CI run and the laptop workflow in
[planning.md](planning.md) must pass.

## What to record after a real install

When a tester or maintainer runs an artifact, record only supplied evidence:

| Release | Package | Environment | Install and launch | Import/edit/render | Notes |
|---|---|---|---|---|---|
| v0.1.0-alpha.1 | .deb/.rpm | Not yet recorded | Not yet recorded | Not yet recorded | Published build artifacts only |
| v0.1.0-alpha.2 | .deb/.rpm | Not yet recorded | Not yet recorded | Native laptop workflow PASS at the exact release commit (`c9ecc2f`), software H.264 | Pale-glass UI overhaul part 1: tokens, MDI teardown, persistent shell, compact timeline |
| v0.1.0-alpha.3 | .deb/.rpm | Not yet recorded | Not yet recorded | Native laptop workflow PASS at the exact release commit (`6d5f48a`), software H.264 | Pale-glass UI overhaul part 2: all eight workspaces docked, invariance across every mode, menu/status/icon/motion polish |
| v0.1.0-alpha.26 | .deb | Arch Linux dev laptop (the only verification host) | **Extracted-prefix launch PASS** (`tests/native/installed_package_workflow.py`) - packaged `usr/bin/saturn-tauri` launched with an isolated HOME from the deb's data layout; this is NOT a dependency-managed install: the host is Arch (no dpkg), ldd reports 0 unresolved libraries | Import + timeline split + save + H.264 render (6.06s, hits h264/duration/frame-size checks) driven over the app's local MCP bridge | Package lint green: control fields, desktop entry (Exec matches the shipped binary), 4 hicolor icons. Container mode (real dpkg install in debian:12) is implemented in the same script and awaits a running docker/podman daemon; the .rpm has never been installed anywhere and remains unproven |

A useful alpha report includes release version, package, desktop session,
diagnostics, imported media codec, reproduction steps, and resulting output or
error. Use that feedback to create a reproducible regression on the development
laptop where possible.

## Beta acceptance runbook (planning.md 6e E.1)

`tests/native/installed_package_workflow.py` is the scripted acceptance
check for packaged builds:

1. Locate the package: `--deb PATH`, or the newest alpha release asset
   (downloaded read-only via gh - no release is created).
2. Package lint: control metadata (Package/Version/Architecture), data
   inventory (`usr/bin` binary, desktop entry whose `Exec` matches a
   packaged binary, hicolor icons), and a shared-library check.
3. Install: `--container` does a real `apt install` inside debian:12 and
   hard-fails on any unresolved library before launch; the default mode
   extracts the data archive to a temp prefix (this laptop is Arch with
   no dpkg, and the container daemon is not running) - the PASS line
   always states the mode so an extracted-prefix result can never be
   mistaken for an install proof.
4. Launch: the packaged binary runs with an isolated HOME/XDG dirs on
   the real desktop session; the runbook refuses to run when a live
   editor already owns the MCP bridge socket.
5. Edit: the local MCP bridge (`$XDG_RUNTIME_DIR/easy-edits-pro-mcp.sock`,
   same JSONL protocol the desktop app serves) drives project.new,
   project.add_media (ffprobe-derived metadata), timeline.add_clip,
   timeline.split_clip, and file.save - then asserts the split survived.
6. Render: `render_start`/`render_status` through the GUI's own job
   queue, then ffprobe verifies codec, duration, and frame size.
7. Quit: SIGTERM, exit recorded.

The PASS line printed on success is the single line to quote in release
verification records; the work directory retains all evidence.
