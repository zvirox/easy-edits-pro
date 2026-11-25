# Design QA

**Updated:** 2026-11-25  
**UI System Status:** **FINAL & LOCKED** (Solid Linear UI — no further UI style changes).

## Scope

This document describes the finalized **Linear-style UI architecture** across desktop workspaces (`Welcome`, `Media`, `Edit`, `Fusion`, `Color`, `Fairlight`, `Effects`, `Keyframes`, `Deliver`) and the self-hosted **Web Desk (`?webdesk=1`)**. It does not certify hardware encoding, native decoding, or installed packages, which are covered by `tests/native/`.

## Locked Design System

- **Window Chrome & Navigation:**
  - Frameless 40 px custom `TitleBar` (`decorations: false`, `transparent: false`, `shadow: true`, `1280×800` default, `1024×640` minimum), reading the version tag from `tauri.conf.json`, with drag-region center, double-click maximize, exclusive Dark/Light `ThemeToggle` (`ui-theme` / `data-theme="dark|light"`), divider, and minimize / maximize / `#E5484D` close buttons.
  - Collapsible workspace `Sidebar` (`240px` expanded / `56px` collapsed automatically at `<=1160px`), workspace `Breadcrumb` bar, and unified `EditorShellFrame` wrapping all workspaces.
- **Surfaces, Color & Elevation:**
  - **Dark (`#050505` / `#111213` / `#1A1B1E` / `#202128`):** `1px` borders (`#1F2125` / `#2C2E33`), primary `#F5F5F6`, muted `#8A8B91`, `box-shadow: none`.
  - **Light (`#F3F4F6` / `#FFFFFF` / `#F4F5F7` / `#EAF0FF`):** `1px` borders (`#E5E7EB` / `#D1D5DB`), primary `#0F1115`, muted `#6B7280`, card shadow `0 1px 2px rgb(16 24 40 / 5%)`.
  - Shared accent tokens (`#3355FF`, `#38B6F0`, `#22C55E`, `#F43F5E`, `#EAB308`, `#1FB5A5`, `#EC4899`, `#7C5CE6`, `#F97316`) and precomputed solid chip tints (`--tint-*`).
  - Strict invariants enforced by `pnpm --dir apps/saturn-tauri check:linear`: zero `backdrop-filter: blur()`, zero translucent surface fills (`rgba()` / `bg-*/10`), zero glow, zero inset highlights, zero dark-theme shadows, zero surface gradients (subtle vertical fill fades allowed on line/sparkline charts only).
- **Shape, Typography & Motion:**
  - Minimal rounding: cards `8px`, buttons/inputs `6px`, chips/tags/tooltips `4px` (`rounded-full` restricted to status dots, avatars, and donut arcs).
  - Self-hosted Inter font only (`ui/public/fonts/InterVariable.woff2`).
  - `150ms ease-out` transitions on discrete non-layout properties; honours `prefers-reduced-motion`; playhead never animated via CSS transitions.
- **Raw Media Surfaces:** Preview canvases, video scopes, audio meters, and Web Review proxy players render raw pixels with zero overlays or blur.

## Verification & Automated Gates

- `pnpm --dir apps/saturn-tauri check:linear` — validates zero blur/glow/dark-shadow/inset/translucent fills, WCAG contrast pairs (`check:contrast`), `check:styles`, `build-token-docs.mjs --check`, and `check-token-freeze.mjs`.
- `pnpm --dir apps/saturn-tauri check:source-size --max-lines=800` — enforces 800-line ceiling on all `.rs`, `.ts`, `.tsx`, and `.css` sources.
- `pnpm --dir apps/saturn-tauri build` & `pnpm test` — TypeScript + Vite production build + Node unit test suite.
- `pnpm --dir apps/saturn-tauri test:e2e` — 13-area Playwright E2E suite against mocked IPC across responsive viewports (`900x620`, `1024x640`, `1024x768`, `1280x800`, `1366x700`, `1600x900`, `1920x1080`).

## Acceptance rule

A UI capability is complete only when its affordance is understandable, its action reaches the Rust-owned project model, it has focused regression evidence, it passes `check:linear` + E2E geometry/overflow checks, and it reports unsupported work as an error rather than a fake success.
