# Easy Edits Pro v0.1.3-beta.1

Welcome to **Easy Edits Pro v0.1.3-beta.1** — our first official Beta release! This milestone closes out the comprehensive **UI Layout & Workspace Consistency Remediation Plan (`UI-P0..UI-P7`)**, brings full responsive dock degradation, fluid multi-aspect preview scaling, and locks in automated end-to-end geometry invariance across the entire 6-viewport × 9-workspace matrix.

---

### Highlights & Key Improvements

- **UI Layout & Workspace Remediation Closed (`UI-P0..UI-P7`)**:
  - **Window Chrome Authority (`UI-01`)**: Single clean native titlebar authority, eliminated duplicate window controls, and locked the root application frame to `100dvh` with zero document scrollbar leakage.
  - **Panel Scroll Containment (`UI-02`)**: Universal `StudioPanel` scroll containment across Inspector, Telemetry, Mixer, and docks; eliminated crushed Inspector body states.
  - **Color & Audio Rack Unclipping (`UI-03`)**: Responsive 2×2 Color wheels with zero raw hex; 132px Fairlight strips with unclipped automation pills and reachable faders.
  - **Fluid Preview Scaling & Deliver Split (`UI-04`)**: Aspect-safe 16:9 (`1.778`) and portrait 9:16 (`0.562`) preview monitor scaling with zero `.glass` blur over video; responsive 2-column Deliver workspace layout.
  - **Workspace Identity (`UI-05`)**: Prominent Tier 1 suite badges, glowing active suite indicators, and Program Monitor suite headers across all 8 suites (`Media`, `Edit`, `Fusion`, `Color`, `Fairlight`, `Effects`, `Curves`, `Deliver`).
  - **Welcome Screen Clarity (`UI-08`)**: Decluttered single-line project recovery banner with clear `Recover` action and nested path ellipsis without document scrollbars down to `900x620`.
  - **Density Harmonization (`UI-09`)**: Standardized 11px compact dock tabs with normalized token padding; renamed tab to `Graphics` to eliminate third-party proprietary naming.
  - **Responsive Dock Degradation (`UI-10`)**: Replaced abrupt `<=1280px` dock hiding with an accessible overlay dock drawer (`.dock-drawer-toggle`), auto-opening on panel events with close button and `Escape` dismiss down to `900x620`.
  - **Full 6×9 Regression Matrix (`UI-P7`)**: Playwright automated assertions locked across all 6 viewports (`900x620` to `1920x1080`) × 9 workspaces with zero root document overflow.

- **AI-Native & Pro Supervision Infrastructure**:
  - Task Sheet work orders (TS0–TS6) & Capture Event Track (CET v0.1).
  - Real-time agent GUI supervision IPC stream and safe branch review/merge/rollback.
  - Multi-target delivery packaging (`web-1080p`, `vertical-9x16`, `broadcast-r128`) and automated export verification.

---

### Package Artifacts (Linux x86_64)

- **Debian package**: `.deb` for Ubuntu 22.04+, Debian 12+
- **RPM package**: `.rpm` for Fedora 38+, RHEL 9+
- **Flatpak bundle**: `.flatpak` for all standard Linux distributions
- **Checksums**: SHA-256 hashes published in `SHA256SUMS` and `.sha256` files.
