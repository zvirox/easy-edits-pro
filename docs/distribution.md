# Distribution and releases

**Updated:** 2026-10-27

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
  WebKitGTK and GStreamer versions. The E.5 spike (2026-10-27) proved the full
  loop inside the sandbox on GNOME 49 - build (manifest + pinned GES module in
  `packaging/flatpak/`), user-scope install, launch, MCP-driven edit, H.264
  render verified by ffprobe - so the decision is FOR Beta 1. It is not yet a
  *published* artifact: no tagged release has shipped one, and the spike drive
  used a CI dispatch build of main rather than a release asset. It becomes a
  supported route once the manifest runs in CI against a tagged release and a
  launch result is recorded for it.
- AppImage is not part of the current delivery plan.
- Future alpha and beta releases gather compatibility feedback through issue
  reports. The project does not maintain a planned external hardware matrix.

### Beta delivery matrix (live status, planning.md 6e E.3)

A checkmark below means a *recorded launch result exists*, never merely a
successful build. The table is a status snapshot; the per-release evidence
table at the bottom of this document is the actual record.

| Artifact | Build | Install | Launch and edit-to-render | Remaining step |
|---|---|---|---|---|
| .deb | CI-proven since alpha.1 | **Container install PASS (2026-10-27, run 40495646190)**: real `apt install` inside debian:12 on the CI runner, ldd cleared, launched under Xvfb; extracted-prefix PASS (alpha.26) | **Container edit-to-render PASS (2026-10-27)**: import + split + save + H.264 MP4 render verified in-container (1920x1080, 6.02s) via the acceptance runbook | One non-dev-machine report |
| .rpm | CI-proven since alpha.1 | **Container install PASS (2026-10-27, run 40495646190)**: real `dnf install` inside fedora:latest after the Requires fix (rpm used to require rpmfusion-only gstreamer1-plugins-ugly/libav, uninstallable on stock Fedora); ldd cleared; launched under Xvfb. E.5 lint PASS (alpha.26); 2 recorded warnings carry (desktop filename contains a space, no appstream metainfo) | **Container edit-to-render PASS (2026-10-27)**: import + split + save + WebM/VP8 render verified in-container via gst-discoverer (1920x1080, 6.03s); renders as WebM on stock Fedora (free repos only) | One non-dev-machine report |
| Flatpak | **CI-proven (2026-10-27, run 40513781606, main@`d2387e5`)**: full flatpak-builder build on the CI runner via flathub `org.flatpak.Builder` against `org.gnome.Platform//49` (GES 1.26.11 compiles in-sandbox), payload installed from the built .deb artifact; single-file bundle exported | **CI install PASS**: installed user-scope from the local build repo, then acceptance drive launches the installed app through `flatpak run` (uid-1001 runtime-dir grant) | **CI edit-to-render PASS (2026-10-27)**: sandboxed launch, MCP import/split/save, GUI-queued render ffprobe-verified (H.264 1920x1080, 6.02s) - run 40513781606, work `/tmp/eep-pkg-s_aztxjc`. **Tagged re-verification PASS (v0.1.1-alpha.1, run 40514895960, built at the tag)**: H.264 6.02s again | One non-dev-machine report |
| SHA256SUMS | Verified in release assets | - | - | - |
| Windows / macOS | Not built | - | - | Out of scope for Beta 1 |

Before beta, either every row above is honest or the artifact is removed from
the release; a row with no launch result may still ship only when the notes
say so explicitly (the beta release-notes template makes this mandatory).

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
| main dispatch (unreleased, `1112645`) | .deb/.rpm | debian:12 and fedora:latest containers on the CI runner (ubuntu-22.04 host, Xvfb, software WebKit) - run 40495646190, 2026-10-27 | **Real-install PASS, both packages**: apt/dnf install of the built artifact, ldd zero unresolved, bridge bound. This run also fixed a blocker the proof caught: the .rpm Requires listed rpmfusion-only/SUSE-style names (gstreamer1-plugins-ugly, gstreamer1-libav, gstreamer1-editing-services) and was uninstallable on stock Fedora | Import + timeline split + save + render verified in-container: .deb → H.264 MP4 1920x1080/6.02s (ffmpeg); .rpm → WebM/VP8 1920x1080/6.03s (gst-discoverer; Fedora free repos only) | Evidence: workflow run logs + container stdout in the job log; PASS lines quoted above |
| main dispatch (unreleased, `d2387e5`) | .flatpak bundle | CI runner ubuntu-22.04, user-scope flatpak (flathub `org.flatpak.Builder`, GNOME 49 Platform/SDK) - run 40513781606, 2026-10-27 | **Flatpak CI PASS**: full manifest build in the Builder sandbox (GES 1.26.11 compiled in-sandbox), app installed from the built repo, bundle exported, acceptance drive launched the installed app via `flatpak run` with a host runtime-dir grant (runner uid 1001 vs manifest `/run/user/1000`) | `PASS - flatpak spike of io.github.zvirox.EasyEditsPro: h264 render verified, 6.02s, 1466175 bytes` - identical import/split/save/GUI-render drive as the deb/rpm container proofs, entirely inside the sandbox | Evidence: run 40513781606 job log (grep `PASS - flatpak`); bundle artifact `easy-edits-pro-flatpak-bundle` |
| v0.1.1-alpha.1 (tagged) | .deb/.rpm/.flatpak bundle + SHA256SUMS | Same CI proof environments as the dispatch run above, but everything rebuilt at the tag - run 40514895960, 2026-10-27 | **All three routes PASS at the tagged release commit**: the release assets (`.deb`, `.rpm`, `easy-edits-pro-linux-x86_64.flatpak` + `.sha256`, `SHA256SUMS`) all carry `0.1.1-alpha.1`; the flatpak bundle is attached by the proof job itself after its acceptance drive | deb: H.264 MP4 1920x1080/6.02s; rpm: WebM/VP8 1920x1080 (via gst-discoverer); flatpak: `PASS - flatpak spike ... h264 render verified, 6.02s, 1471781 bytes` - the tagged proof is the release asset's provenance, not just a same-tree build | Evidence: run 40514895960 (logs archived at attempt 1), release <https://github.com/zvirox/easy-edits-pro/releases/tag/v0.1.1-alpha.1> |

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
   packaged binary, hicolor icons), a shared-library check, desktop
   validation via `desktop-file-validate`, and a file-conflict scan
   (nothing outside app-private prefixes: `/usr/bin`,
   `/usr/share/{applications,icons,saturn-tauri,metainfo}`,
   `/usr/lib/saturn-tauri`). The rpm runs the same gates offline (`--rpm`)
   through bsdtar/libarchive — no rpm tooling required.
3. Install: `--container` does a real `apt install` inside debian:12 and
   hard-fails on any unresolved library before launch; `--rpm-container`
   does the same via `dnf install` inside fedora:latest (renders WebM/VP8;
   Fedora free repositories ship no x264, so the package Requires cover
   only free GStreamer plugins); the default mode
   extracts the data archive to a temp prefix (this laptop is Arch with
   no dpkg) - the PASS line
   always states the mode so an extracted-prefix result can never be
   mistaken for an install proof.
4. Launch: the packaged binary runs with an isolated HOME/XDG dirs on
   the real desktop session; the runbook refuses to run when a live
   editor already owns the MCP bridge socket.
5. Edit: the local MCP bridge (`$XDG_RUNTIME_DIR/easy-edits-pro-mcp.sock`,
   same JSONL protocol the desktop app serves) drives project.new,
   project.add_media (probe_media-derived metadata), timeline.add_clip,
   timeline.split_clip, and file.save - then asserts the split survived.
6. Render: `render_start`/`render_status` through the GUI's own job
   queue, then probe_media (ffprobe, or gst-discoverer-1.0 where ffmpeg
   is absent) verifies codec, duration, and frame size.
7. Quit: SIGTERM, exit recorded.

The PASS line printed on success is the single line to quote in release
verification records; the work directory retains all evidence.
