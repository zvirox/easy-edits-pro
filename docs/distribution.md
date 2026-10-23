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

A useful alpha report includes release version, package, desktop session,
diagnostics, imported media codec, reproduction steps, and resulting output or
error. Use that feedback to create a reproducible regression on the development
laptop where possible.
