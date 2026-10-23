# Distribution (packet B1 — decision record)

**Decision (2026-10-23): Flatpak is the supported distribution path.**

## Rationale

- Bundles a known WebKitGTK 4.1 + GStreamer (incl. `libav`/H.264) inside the
  sandbox, so version drift across distros — the single biggest
  compatibility risk for a WebKitGTK editor — disappears.
- One artifact works on Arch, Fedora, and Ubuntu, including the integrator's
  Arch test machine that the existing `.deb`/`.rpm` outputs do not serve.
- Sandbox permissions are auditable and limited to what the app needs
  (user media folders, Wayland/X11, GPU).

## Alpha package releases (B4)

Pushing a semantic alpha tag such as `v0.1.0-alpha.1` starts the Linux package
workflow. It builds `.deb` and `.rpm` packages using the tag's version, creates
SHA-256 checksums, and attaches all three outputs to a GitHub prerelease. The
packages remain best-effort alpha artifacts; Flatpak remains the supported
distribution path until the install matrix is complete.

Create and push an alpha tag from the commit to release:

```bash
git tag v0.1.0-alpha.1
git push origin v0.1.0-alpha.1
```

## Consequences

- The existing `.deb`/`.rpm` CI outputs remain built, but become
  **best-effort secondary artifacts** (not the tested path) until the
  install matrix says otherwise.
- AppImage stays dropped (see `ca270fd`); do not reintroduce it without a
  new decision.
- Follow-up packets: **B2** Flatpak manifest + CI artifact, **B3**
  `--self-test` flag, **B5** install matrix
  (Ubuntu 22.04/24.04, Fedora current, Arch — record WebKitGTK version,
  GPU, result).

## Verified install results (B5 — fill from real runs only)

| Distro / version | WebKitGTK | GPU | Format | Result |
|---|---|---|---|---|
| (none recorded yet — do not claim a row without a real install + launch) | | | | |
