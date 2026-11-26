# Design QA

**Updated:** 2026-11-26  
**UI System Status:** **FINAL & LOCKED** (Shared Linear Solid UI · Orange Accent · see [DESIGN.md](../DESIGN.md)).

## Scope

This document describes the finalized **Linear-style UI architecture** across desktop workspaces (`Welcome`, `Media`, `Edit`, `Fusion`, `Color`, `Fairlight`, `Effects`, `Keyframes`, `Deliver`) and the self-hosted **Web Desk (`?mode=desk`)**. It does not certify hardware encoding, native decoding, or installed packages, which are covered by `tests/native/`.

## Locked Design System

- **Window Chrome & Navigation:**
  - Frameless 40 px custom `TitleBar` (`decorations: false`, `transparent: false`, `shadow: true`, `1280×800` default, `1024×640` minimum), reading the version tag from `tauri.conf.json`, with drag-region center, double-click maximize, exclusive Dark/Light `ThemeToggle` (`ui-theme` / `data-theme="dark|light"`), divider, and minimize / maximize / `#E5484D` close buttons.
  - Collapsible workspace `Sidebar` (`240px` expanded / `56px` collapsed automatically at `<=1160px`), workspace `Breadcrumb` bar, and unified `EditorShellFrame` wrapping all workspaces.
- **Surfaces, Color & Elevation (`linear-theme.css` + `app-accent.css`):**
  - **Dark (`#1C1C1C` / `#171717` / `#242424` / `#2D2D2D`):** `1px` borders (`#2F2F2F` / `#3B3B3B`), primary `#E6E6E6`, muted `#8F8F8F`, `--shadow-card: 0 12px 28px -6px rgba(0,0,0,0.5)`.
  - **Light (`#F8F9FA` / `#F1F3F5` / `#FFFFFF` / `#F3F3F3`):** `1px` borders (`#DEE2E6` / `#C6CDD4`), primary `#1C1C1C`, muted `#636970`, `--shadow-card: 0 1px 2px rgb(16 24 40 / 5%)`.
  - **EEP Orange Primary:** `#C2410C` / `#AB390B` / `#FFFFFF` / `#FBF2EE` (light) and `#F97316` / `#FA8432` / `#1C1C1C` / `#FB923C` / `#3B281B` (dark). Positive/healthy states use **Sky** (`#0284C7` / `#38BDF8`), never green.
  - Strict invariants enforced by `pnpm --dir apps/saturn-tauri check:linear`: byte-for-byte shared `linear-theme.css`, zero green hues (`75°–170°`), zero foreign primaries, zero `backdrop-filter: blur()`, zero translucent surface fills, zero glow, zero inset highlights, zero arbitrary/non-`--shadow-card` shadows, and zero charting libraries (`echarts` / `DonutLegend` removed).
- **Shape, Typography & Motion:**
  - Minimal rounding: cards `8px`, buttons/inputs `6px`, chips/tags/tooltips `4px` (`rounded-full` restricted to status dots, playhead dots, and avatars).
  - Self-hosted Inter font only (`ui/public/fonts/InterVariable.woff2`).
  - `150ms ease-out` transitions on discrete non-layout properties; honours `prefers-reduced-motion`; playhead never animated via CSS transitions.
- **Raw Media & Technical Surfaces:** Preview canvases, Canvas `VideoScopes` (`--sky`, `--amber`, `--red`, `--primary`), audio meters, and Web Review proxy players render raw pixels with zero overlays or blur.

## Verification & Automated Gates

- `pnpm --dir apps/saturn-tauri check:linear` — validates zero blur/glow/dark-shadow/inset/translucent fills, WCAG contrast pairs (`check:contrast`), `check:styles`, `build-token-docs.mjs --check`, and `check-token-freeze.mjs`.
- `pnpm --dir apps/saturn-tauri check:source-size --max-lines=800` — enforces 800-line ceiling on all `.rs`, `.ts`, `.tsx`, and `.css` sources.
- `pnpm --dir apps/saturn-tauri build` & `pnpm test` — TypeScript + Vite production build + Node unit test suite.
- `pnpm --dir apps/saturn-tauri test:e2e` — 13-area Playwright E2E suite against mocked IPC across responsive viewports (`900x620`, `1024x640`, `1024x768`, `1280x800`, `1366x700`, `1600x900`, `1920x1080`).

## Acceptance rule

A UI capability is complete only when its affordance is understandable, its action reaches the Rust-owned project model, it has focused regression evidence, it passes `check:linear` + E2E geometry/overflow checks, and it reports unsupported work as an error rather than a fake success.
