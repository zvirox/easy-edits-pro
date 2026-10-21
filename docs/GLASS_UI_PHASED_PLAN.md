# Easy Edits Pro — Glass UI Phased Implementation Plan v2.1

> **Source:** `glass-ui-plan-v2.md` + skill analysis (glassmorphism, progressive-blur, baseline-ui, reverse-engineer-anything) + risk mitigation strategies
> **Status:** Phases 1–3 complete; Phase 4 core shipped (see the Phase 4 "Shipped this pass" note below)
> **Total estimated effort:** 2.5–3 weeks (13–18 work days)

---

## Phase Overview

| Phase | Duration | Focus | Gate |
|-------|----------|-------|------|
| **0** | 0.5–1 day | Feasibility spike (WebKitGTK performance) | **GO (DMA-BUF default enabled)** |
| **1** | 1–2 days | Token cleanup, contrast, CI gates | All contrast ≥4.5:1, 0 raw colours |
| **2** | 1–2 days | Glass surfaces, background, rename CSS | 0 ad-hoc backdrop-filter |
| **3** | 1–2 days | Animation hygiene, motion/react | 0 `transition: all` |
| **4** | 3–5 days | Floating panel engine | Drag/resize/snap/dock/persist |
| **5** | 2–3 days | Responsiveness, container queries | Breakpoints work |
| **6** | 2 days | Performance, a11y, QA, CI | `test:e2e` green |

**Total:** ~12–15 work days + 1–2 days buffer = **2.5–3 weeks**

### Repository reality check (verified 2026-10-20)

The plan predates the current tree. These corrections apply to the phases below:

- **App location:** the Tauri shell is `apps/saturn-tauri` (its own Cargo
  workspace), with the frontend in `apps/saturn-tauri/ui/src`. There is no
  repository-root `src-tauri`.
- **Muted text is already lifted:** `--text-muted` is `#a890a0` (~4.5:1 on
  `#180d17`). The Phase 1 G1b reference to `#7d6377` and "~139 rules" is stale;
  recount the current tree before batch-editing.
- **Motion is already present:** `motion ^14.2.0` is installed and used by
  `components/interior/*` (`motion/react`, `useReducedMotion`). Phase 3 needs no
  dependency add.
- **Existing glass debt (pre-G1d snapshot):** `styles/` contained **11 ad-hoc
  `backdrop-filter` rules** (15 declarations including `-webkit-`), **41
  `transition: all`**, and 762 hex + 436 `rgb()/rgba()` literals across 19 CSS
  files (8,553 lines). Phase 1 G1d has since replaced all non-token literals and
  removed every `transition: all`; there are **no `.glass` selectors** outside
  the Phase 0 spike yet.

---

## Phase 0 — Feasibility Spike (½–1 day) ⚡ **CRITICAL GATE**

### Objective
Validate `backdrop-filter` performance in Tauri's WebKitGTK **before committing to glass**.

### Tasks
- [x] Create throwaway branch `feat/glass-spike`
- [x] Implement minimal test: `--bg-stage` background + 3 `.glass` panels (L1/L2/L3) + 1 draggable panel + playing `<video>`
- [x] Measure frame time while dragging + video playing on:
  - [x] Integrated-graphics machine (Intel Iris Xe / i5-1155G7 — the plan's low-end class)
  - [ ] Discrete / high-end GPU (not available; follow-up)
- [x] Test **inside Tauri window** (WebKitGTK), not Chrome
- [x] Document results in `docs/glass-spike.md`
- [x] *Out of scope:* release packaging. Dev and release share the same WebKitGTK runtime; the spike ran in a real Tauri window.

### Result (2026-10-20)

| Scenario (1600×900 @ DPR 2, 30 fps video) | DMA-BUF off (app default) | DMA-BUF on |
|---|---:|---:|
| Drag 1 full-glass panel | 2.4 fps | **58.9 fps** |
| Drag 2 full-glass panels (stress) | 1.4 fps | **54.6 fps** |
| Drag 1 panel, Lite | 3.3 fps | 60.0 fps |
| Drag 1 panel, Off (no blur) | 3.5 fps | 60.0 fps |
| Video-only baseline | 1.1 fps / 0 presented frames | 60.0 fps / 30 fps video |

**Decision:** Full glass passes the ≥30 fps gate with ~2× headroom on
integrated graphics — **but only when WebKit's DMA-BUF renderer is enabled**.
With the old default (`WEBKIT_DISABLE_DMABUF_RENDERER=1`), the preview collapsed
to 1–3.5 fps even without any blur, so the blocker was the video compositing
path, not glass.

**Runtime follow-up (resolved):** the unconditional disable was replaced by an
opt-out policy (`EEP_DISABLE_DMABUF=1`, or a driver/vendor match), so DMA-BUF is
on by default with a per-driver escape hatch. The real editor was then measured
with a 10 s 1080p H.264 clip at 1600×900 @ DPR 2:

| Real editor scenario | DMA-BUF on | DMA-BUF forced off |
|---|---:|---:|
| Video only | **54.0 fps** (30 fps video) | 13.1 fps, 0 presented frames |
| Drag full glass, run 1 | **43.9 fps** | 8.5 fps |
| Drag full glass, run 2 | **47.9 fps** | 7.2 fps |

After the G1d token burn-down the same production build held 51.6 fps
video-only and 47.2/47.8 fps while dragging, with 0 dropped video frames. Full
glass is approved; Phase 2 must preserve that headroom.

### Go/No-Go Criteria
| Metric | Threshold | Decision |
|--------|-----------|----------|
| Dragging FPS (low-end) | ≥30 fps | **Full glass viable** |
| Dragging FPS (low-end) | 15–30 fps | **Lite default, Full opt-in** |
| Dragging FPS (low-end) | <15 fps | **No glass; shadow-only approximation** |

### Output
- `docs/glass-spike.md` with: WebKitGTK version, GPU vendor, driver, FPS numbers, decision
- If **No-Go** → Skip Phases 2–3, implement shadow-only "Lite" mode in Phase 6

### Risk Mitigation (NEW)
- [x] Pre-build **Lite fallback** (CSS gradient + shadow, zero `backdrop-filter`) for slow paths — `[data-glass="lite"]` / `[data-glass="off"]`
- [x] Rust-side GPU detection (`apps/saturn-tauri/src/gpu_detect.rs`) — exposes GPU class/driver/DMA-BUF state to the UI, which sets `[data-webkit-gtk-slow]` at startup
- [x] Feature flag `VITE_GLASS_ENABLED` for instant kill switch — build-time;
  `VITE_GLASS_ENABLED=0` (or `false`) sets `data-glass="off"` and removes blur

---

## Phase 1 — Token Cleanup & Contrast (1–2 days) 🎨 **FOUNDATION**

> **Status 2026-10-20:** Phase 1 is complete. G1a–G1c and G1e landed on
> `feat/glass-phase-1`; G1d landed on `feat/glass-runtime` after the DMA-BUF fix.
> All 951 remaining raw colours were replaced, the 44 light-theme selectors were
> deleted, and light mode now resolves entirely through tokens. The style gate
> has an empty baseline, so any new literal fails immediately.

### Objective
Establish token-only color system with WCAG AA contrast; eliminate all raw literals.

### Work Packets

#### G1a — Glass Token File
- [x] Create `styles/00-tokens-glass.css` (imported right after `01-editor.css`)
- [x] Define tokens from plan + accessibility additions:
  ```css
  --accent-text: #f472b6;           /* dark; light is #9f1249 */
  --accent-text-hover: #f9a8d4;
  --accent-text-violet: #c084fc;
  --text-muted: #a890a0;            /* already shipped in 01-editor.css */
  --glass-l1..l4, --glass-border, --glass-highlight, --glass-shadow
  --glass-saturate: 145%;           /* light: 135% */
  --blur-l1..l4, --ease-spring, --ease-out
  --dur-fast: 120ms, --dur-med: 220ms, --dur-slow: 360ms
  --bg-stage (radial glows + canvas)
  ```
- [x] Add `[data-glass="lite|off"]` and `@media (prefers-reduced-transparency/motion)` fallbacks

#### G1b — Contrast Fixes
- [x] Switch accent-as-text rules (`#9f1249`, `#6b21a8`, `#be185d`, `#d9468f`,
  `#a855f7`, `#c084fc`) → `--accent-text` / `--accent-text-violet`:
  153 declarations across 18 files (98 accent-text, 33 text-muted, 22 violet).
- [x] Recount and update rules using `--text-muted` — the token is already
  `#a890a0`; the `#7d6377` / "~139 rules" figure is stale. The 15 remaining
  raw `#7d6377` and `#bfa5b8` text rules were moved to `--text-muted`.
- [x] Verify: no small text uses dark rose/violet. `check:contrast` now fails on
  those literals if they come back.

#### G1c — Contrast CI Gate
- [x] Create `scripts/check-contrast.mjs`:
  - Parses all token blocks (dark + light), resolves `var()` aliases
  - Computes contrast for every semantic text/surface pair (306 pairs,
    including the G1d component roles), including
    translucent `--bg-glass` composited over the canvas
  - Fails CI if any pair is <4.5:1, or if a banned accent/muted text literal appears
- [x] Add to CI: `pnpm check:contrast` runs on every PR

#### G1d — Literal Elimination
- [x] Introduce a raw-colour ratchet (`scripts/style-baseline.json`) and then
  zero it: the baseline is now `{}`, so any literal hex/rgb/hsl outside the
  token files fails CI
- [x] Replace all 951 remaining raw colours across `styles/` with semantic
  tokens or `color-mix()` over tokens; raw literals also stripped from
  `var(--token, literal)` fallbacks
- [x] Add the role tokens the component rules needed in both themes:
  `--bg-workspace`, `--bg-card*`, `--bg-control*`, `--bg-strip`, `--bg-inset`,
  `--bg-scopes*`, `--bg-meter-*`, `--bg-status`, `--border-node`,
  `--border-inset`, `--border-meter-pair`, `--master-accent`, `--strip-accent`,
  `--shadow-node`, `--curve-grid-*`, and status/text roles
- [x] Delete all 44 `[data-theme="light"]` selectors in `16-editor.css`; the
  base component rules now consume the theme-aware roles
- [x] Fix the Keyframes curve grid/labels with `--curve-grid-line*` /
  `--curve-grid-text` so the graph stays legible in light mode
- [x] Target: **0 raw hex/rgba in `styles/`** (except token files) — met

#### G1e — Lint Gates
- [x] Add style gates (`scripts/check-styles.mjs`, dependency-free rather than
  Stylelint so it runs without a new toolchain):
  - raw-colour check outside token files (comments stripped, `hsl()` included)
  - `transition: all` ban
  - max-lines-per-file: 1,000 hard, 700 warning
- [x] Remove all 41 `transition: all` declarations (expanded to explicit
  background/border/color/shadow/opacity/transform transitions)
- Note: a Stylelint plugin can replace the script later; the CI contract is the
  same.

### Acceptance Criteria
- [x] `pnpm check:contrast` passes (306 pairs at 4.5:1, 0 banned text colours)
- [x] 0 raw hex/rgba in `styles/` — empty ratchet baseline; `hsl()` also checked
- [x] 0 `transition: all` in codebase
- [x] 0 `[data-theme="light"]` overrides in `styles/` (token files only)
- [x] Light theme works via tokens only — verified in the real editor across
  Edit, Fusion, Color, Fairlight, Curves, and Deliver

### Risk Mitigation (NEW)
- [x] **Contract tests** for token values:
  `ui/src/styles/tokens.test.ts` (adapted to the repo's `node --test` layout)
- [x] **Living docs**: `docs/glass-tokens.md` generated by
  `scripts/build-token-docs.mjs`; `pnpm check:tokens` fails if code ≠ docs

---

## Phase 2 — Glass Surfaces & Background (1–2 days) 🪟

### Objective
Apply single `.glass` system; remove all ad-hoc glass; establish elevation scale.

### Work Packets

#### G2a — CSS Rename (Rename-Only Commit) — DEFERRED
> **Implementation note (2026-10-21):** The rename map below does not match the
> actual files. For example, `12-editor.css` contains Fairlight bus routing and
> Fusion nodes, while `16-editor.css` still contains live track/color-studio
> rules. Renaming to the proposed domain names would mislabel the code and
> create a misleading diff. Phase 2 ships the functional glass work first; the
> rename waits for a real domain split instead of a mechanical rename.

- [ ] Rename `NN-editor.css` → domain names **without changing import order**:
  ```
  01-editor.css → 10-base.css           (tokens, reset, base)
  02-editor.css → 20-layout.css         (grid, flex, spacing)
  03-editor.css → 30-typography.css     (type scale, text utilities)
  04-editor.css → 40-components.css     (buttons, inputs, cards)
  05-editor.css → 50-workspaces.css     (workspace ribbon, panels)
  06-editor.css → 60-light-overrides.css (DELETE after Phase 1)
  07-editor.css → 70-timeline.css       (timeline-specific)
  08-editor.css → 80-media.css          (preview, waveforms, scopes)
  09-editor.css → 90-effects.css        (color wheels, keyframes)
  10-editor.css → 100-dialogs.css       (modals, popovers, drawers)
  11-editor.css → 110-tooltips.css      (tooltips, toasts)
  12-editor.css → 120-glass.css         (NEW: .glass, elevations)
  13-editor.css → 130-animations.css    (motion tokens, keyframes)
  14-editor.css → 140-responsive.css    (container queries, breakpoints)
  15-editor.css → 150-print.css         (print styles)
  16-editor.css → (DELETE)
  17-editor.css → 170-utilities.css     (helpers, debug)
  18-editor.css → 180-vendor.css        (third-party overrides)
  ```
- [ ] **Single commit, rename-only** — no logic changes

#### G2b — Apply Glass System
- [x] Create `styles/120-glass.css` with `.glass` + `data-elev="1-4"` from plan
- [x] Apply to components:
  - [x] `StudioPanel` (L1)
  - [x] `AppHeader`, menus, popovers (L1/L3)
  - [x] `BatchQueueDrawer` (L3)
  - [x] `OllamaChatDrawer` (L3)
  - [x] `features/dialogs/*` modals (L4)
  - [x] Tooltips (glass tokens; the pseudo-element cannot take `.glass`), toasts
  - [x] Status bar (L1)
- [x] **Delete 11 ad-hoc `backdrop-filter` rules** (menus, banners, overlays,
  job drawer, welcome, scopes, tooltips, drawers). `backdrop-filter` now lives
  only in `00-tokens-glass.css` (fallbacks) and `120-glass.css`.

#### G2c — Background & Gaps
- [x] Put `--bg-stage` on the app root (`app-shell`, welcome shell)
- [x] Docked workspaces are transparent so the existing 8px gutters show the stage
- [x] Verify background glows visible (dark/light screenshots)

#### G2d — Media Surface Protection
- [x] **Audit**: Ensure NO glass on:
  - [x] Preview (`features/preview/PreviewSection.tsx`)
  - [x] Timeline clip area (`features/timeline/TimelineSection.tsx`,
    `TimelineTrackLane.tsx`)
  - [x] Scopes (`components/VideoScopes.tsx`, `ColorWorkspace.tsx`)
  - [x] Waveforms (`components/AudioMonitorMixer.tsx`,
    `SourceAudioVisualizer.tsx`)
- [x] Add dependency-free gate: `scripts/check-styles.mjs` rejects glass classes
  and inline backdrop filters in those media components. The repo has no ESLint
  dependency, so the gate follows the existing Node-script pattern.

### Acceptance Criteria
- [x] Single `.glass` class + `data-elev` used everywhere
- [x] 0 ad-hoc `backdrop-filter` rules outside the glass token/primitive files
- [x] Screenshots: dark/light × Full/Lite/Off for each workspace (36 captures)
- [x] Media surfaces render sharp, no blur

### Risk Mitigation (NEW)
- [x] **Architectural boundary**: media components are allowlisted in the style
      gate; any `.glass`/`data-elev`/backdrop filter in them fails CI
- [ ] **Visual regression thresholds** per component (PreviewFrame = 0%
      tolerance) — deferred to the visual-regression harness; this phase
      captured the 36-state baseline manually

---

## Phase 3 — Animation Foundation (1–2 days) ✨

> **Shipped:** `feat(glass-phase-3)` on `main`. Discovery found `transition: all`
> already at zero, so G3a converted the 6 layout-property transitions that did
> remain: job/batch progress fills now animate `transform: scaleX()` and the five
> live audio meters (VU, VU-channel, GR track, GR meter, loudness) carry no
> transition at all; JS progress writers feed `scaleX` directly. G3b added
> `ui/src/motion/variants.ts` (opacity/scale/x-only variants + `useMotionPreset`)
> and `ui/src/motion/GlassDialog.tsx`; `AnimatePresence` now owns mount/unmount
> for all nine dialogs, both drawers (batch queue + AI chat), the command
> palette, the job popover, and the menubar (converted from `<details>` to a
> controlled popover with outside-click/Escape close). G3c extended
> `scripts/check-styles.mjs` to reject layout-property transitions, layout
> keyframes, any animation on playhead selectors, and layout-property inline
> transitions in TSX. `130-animations.css` adds a global reduced-motion reset.
> Reduced motion is verified twice: `editor-smoke.mjs` emulates
> `prefers-reduced-motion: reduce` and asserts the CSS reset collapses
> animations, and a motion-QA harness reads palette opacity mid-flight in each
> glass mode (animated) versus reduced (instant). Note: Motion 14's
> `useReducedMotion` snapshots the OS preference once at mount, so Phase 3 owns
> a reactive hook (`useReducedMotionPreference`) instead.

### Objective
Eliminate layout animation; use motion/react for all transitions; respect reduced-motion.

### Work Packets

#### G3a — Transition Cleanup
- [ ] Replace 59 `transition: all` → explicit properties:
  ```css
  /* Allowed */
  transition: transform 220ms var(--ease-out),
              opacity 180ms var(--ease-out),
              background-color 180ms var(--ease-out),
              border-color 180ms var(--ease-out),
              box-shadow 220ms var(--ease-out);
  /* Forbidden */
  transition: all;
  transition: width, height, top, left, margin, padding, backdrop-filter;
  ```

#### G3b — Motion/React Integration
- [ ] Use `motion/react` (already installed and used by `components/interior/*`) for:
  - Panel mount/unmount (scale + opacity)
  - Menu/dialog enter/exit
  - Toast notifications
- [ ] Reuse `useReducedMotion` from `components/interior/*`
- [ ] Create shared animation variants in `styles/130-animations.css`

#### G3c — Animation Rules (CI Enforced)
- [ ] Add Stylelint rule: forbid animating `width`, `height`, `top`, `left`, `margin`, `padding`, `backdrop-filter` (blur radius)
- [ ] Add rule: timeline/playhead elements never animate
- [ ] Verify `@media (prefers-reduced-motion)` disables all transitions/animations globally

### Acceptance Criteria
- [ ] 0 `transition: all` in codebase
- [ ] 0 layout property animations
- [ ] `motion/react` used for all mount/unmount
- [ ] `prefers-reduced-motion` → zero animation (tested in CI)

### Risk Mitigation (NEW)
- [ ] **Performance budget**: `pnpm perf:glass` script measures frame time during animation
- [ ] **Mutation testing** for animation logic: `npx stryker run --mutate="src/features/panels/*.ts"`

---

## Phase 4 — Floating Panel Engine (3–5 days) 📦 **HIGHEST COMPLEXITY**

> **Shipped this pass (core):** `ui/src/features/panels/` engine is live — `PanelHost` (portals + z-manager with
> click-to-raise), `FloatingPanel` (glass shell, drag header, 8-direction resize, collapse-to-chip, presence
> animations from the Phase 3 presets), `usePanelDrag` / `usePanelResize` / `usePanelLayout`, `snapping.ts`
> (edge + panel snap, 12px, region-local pixels) and `layouts.ts` (fraction-based defaults mirroring the old
> dock). The media pool and inspector in the Edit workspace are engine-managed floating panels; they position
> against the editor region below the app chrome so they can never cover the menu bar or status bar, and the
> docked grid keeps preview/timeline in their old columns. Persistence goes through the new
> `save_panel_layout` / `load_panel_layout` Tauri commands (`src/commands/panels.rs`, per-workspace JSON in the
> app data dir, atomic write) with a localStorage fallback for collapse/closed state. The dock rail reopens
> closed panels and hosts "Reset layout". Verified in Chromium QA (drag, snap-to-edge and neighbour, resize,
> collapse/restore, close/reopen, reduced-motion instant restore, layout survives workspace switches, light
> theme) plus a Playwright run of `editor-smoke.mjs`; 10 new node:test units cover snapping/clamping/sanitising.
> **Deferred:** dock zones with ghost preview, pin semantics beyond the flag, `panelStore` (React state in
> `usePanelLayout` was enough — no Zustand dependency added), per-workspace migration beyond Edit,
> Window-menu reopen commands, stryker mutation testing.

### Objective
Build production-ready floating panel system with persist, snap, dock, responsiveness.

### New Structure
```
src/features/panels/
├── PanelHost.tsx           # Root portal + z-manager
├── FloatingPanel.tsx       # Glass panel wrapper
├── usePanelDrag.ts         # Drag logic (header only)
├── usePanelResize.ts       # 8-handle resize
├── usePanelLayout.ts       # Layout state + persistence
├── snapping.ts             # Edge/panel snap (12px)
├── layouts.ts              # Preset layouts per workspace
├── panelStore.ts           # Zustand/Redux store
└── index.ts                # Public API
```

### Work Packets

#### G4a — Core Interactions
- [ ] Drag by header (cursor: grab/grabbing)
- [ ] 8-handle resize (n, ne, e, se, s, sw, w, nw)
- [ ] Bring-to-front on interaction
- [ ] Snap to viewport edges + other panels (~12px threshold)
- [ ] Dock zones with ghost preview (drop target highlight)
- [ ] Collapse to chip (title only, fixed width)
- [ ] Pin (exclude from auto-dock)
- [ ] Close with scale-fade (motion/react)
- [ ] Window menu → reopen closed panels
- [ ] "Reset layout" command

#### G4b — Layout Persistence
- [ ] Store positions as **viewport fractions** (0–1) with min/max clamps
- [ ] On window resize: recalculate pixel positions from fractions
- [ ] Persist per workspace (Rust command `save_panel_layout` + `load_panel_layout`)
- [ ] LocalStorage fallback for non-project state
- [ ] Validate on load against current screen size (clamp if needed)

#### G4c — Migration Order (Low Risk → High Risk)
1. [ ] **Inspector** (simple, read-only-ish)
2. [ ] **Media Pool** (drag source, no internal state)
3. [ ] **Effects** (parameter controls)
4. [ ] **Scopes** (read-only visual)
5. [ ] **Chat** (OllamaChatDrawer — already floating)
- [ ] Keep Preview + Timeline **docked** (never floating)

#### G4d — Unit Tests
- [ ] `snapping.ts`: edge cases (overlapping snaps, viewport boundaries)
- [ ] `layouts.ts`: fraction→pixel roundtrip, clamp validation
- [ ] `usePanelLayout.ts`: persist/load, resize handling

### Acceptance Criteria
- [ ] All interactions smooth (60fps on reference machine)
- [ ] Layout survives window resize, reload, project switch
- [ ] Unit tests pass in CI
- [ ] No panel strands off-screen

### Risk Mitigation (NEW)
- [ ] **Contract tests** for panel engine: `tests/panels/contracts.test.ts`
- [ ] **Visual regression**: screenshots per panel state (docked/floating/collapsed/pinned)
- [ ] **Mutation testing**: `npx stryker run --mutate="src/features/panels/snapping.ts,layouts.ts"`

---

## Phase 5 — Responsiveness & Polish (2–3 days) 📱

### Objective
Container queries, breakpoints, touch targets, layout presets.

### Work Packets

#### G5a — Container Queries
- [ ] Each panel: `container-type: inline-size`
- [ ] Header controls scale: `clamp(22px, 2.2cqi, 32px)`
- [ ] Titles collapse to icons at narrow widths
- [ ] Close button **≥24px hit target** (always)

#### G5b — Window Breakpoints
| Width | Behavior |
|-------|----------|
| ≥1440px | All panels floating/docked freely |
| 1100–1440px | Floating panels auto-dock to icon rail |
| 900–1100px | One floating panel at a time; others in drawer |
| <900px | Single panel mode; full-screen drawers |

- [ ] Implement in `styles/140-responsive.css`
- [ ] Test at 900, 1100, 1440, 1920px

#### G5c — Layout Presets
- [ ] Presets per workspace: "Editing", "Color", "Audio", "Effects", "Delivery"
- [ ] Save/load via panel store
- [ ] Validate on load (screen size changed → recompute)

### Acceptance Criteria
- [ ] Controls scale smoothly in DevTools device toolbar
- [ ] Breakpoints trigger correctly
- [ ] Close button always ≥24px
- [ ] Presets apply and persist

---

## Phase 6 — Performance, Accessibility, QA (2 days) ✅

### Objective
Production hardening: settings, budgets, a11y, CI gates.

### Work Packets

#### G6a — Settings UI
- [ ] Settings panel: Glass (Auto / Full / Lite / Off) + Animations (Full / Reduced)
- [ ] **Auto mode**: Startup frame-time probe (3s) → picks mode
- [ ] **Auto-downgrade**: During playback/export, if frame time >20ms → drop to Lite
- [ ] Persist choice in project settings

#### G6b — Performance Budget
- [ ] Enforce in code + CI:
  - Max 3–4 blurred surfaces visible simultaneously
  - Blur radius ≤20px (configurable via `--blur-max`)
  - Never render blurred panel over `<video>` / live preview
  - `will-change: transform` only during active animation
- [ ] Add `pnpm perf:glass` script (Playwright + Lighthouse CI)

#### G6c — Accessibility Pass
- [ ] Focus ring visible on glass (contrast ≥3:1 against glass bg)
- [ ] `aria-label` on all panel controls (close, pin, collapse, dock)
- [ ] Keyboard navigation: Tab through panels, Esc closes, arrows move focus
- [ ] Screen reader test: NVDA/Orca announces panel state changes
- [ ] `prefers-reduced-transparency` → solid fallback (tested in CI)
- [ ] `prefers-reduced-motion` → zero animation (tested in CI)

#### G6d — QA & CI Integration
- [ ] Extend `scripts/editor-smoke.mjs`:
  - Screenshots per workspace × theme × glass mode (Auto/Full/Lite/Off)
  - Compare against baselines in `tests/visual/baselines/`
- [ ] **Add `pnpm test:e2e` to CI** (GitHub Actions)
- [ ] Visual regression thresholds:
  ```js
  // playwright.config.ts
  THRESHOLDS = {
    'TimelineSection': 0.1,     // strict
    'PreviewFrame': 0.0,        // zero tolerance
    'FloatingPanel': 0.5,
    'AppHeader': 0.2,
    'BatchQueueDrawer': 0.3,
  }
  ```
- [ ] Run full test suite: `cargo test --workspace && pnpm test && pnpm check:source-size && pnpm test:e2e`

### Acceptance Criteria
- [ ] Settings persist and work
- [ ] Auto mode selects correctly on low-end device
- [ ] Performance budget met (frame time ≤16ms idle, ≤33ms during drag)
- [ ] WCAG AA compliance (axe-core + manual)
- [ ] `pnpm test:e2e` green in CI
- [ ] Visual regression baselines committed

### Risk Mitigation (NEW)
- [ ] **Staged rollout metrics**: Telemetry (opt-in) for `glass_mode_active`, `frame_time_p99`, `fallback_triggered`
- [ ] **Automatic rollback** if `fallback_triggered` >5% of sessions
- [ ] **Documentation as contract**: `docs/glass-performance.md`, `docs/glass-accessibility.md`, `docs/glass-qa-checklist.md`

---

## Cross-Phase Infrastructure (Setup Before Phase 1)

### CI/CD Additions
```yaml
# .github/workflows/glass-ui.yml
name: Glass UI Checks
on: [pull_request]
jobs:
  contrast:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pnpm check:contrast
  tokens:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pnpm check:tokens  # no raw hex/rgba
  transitions:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pnpm check:transitions  # no transition:all
  size:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pnpm check:source-size  # max 1000 lines
  e2e:
    runs-on: ubuntu-latest
    needs: [contrast, tokens, transitions, size]
    steps:
      - uses: actions/checkout@v4
      - run: pnpm test:e2e
```

### AGENTS.md (Create at Repo Root)
```markdown
# AGENTS.md — Hard Rules for AI Developers

## Colour System
- Colours ONLY via tokens in `styles/01-editor.css` + `00-tokens-glass.css`
- NEVER use raw hex/rgba in components or styles
- Palette: warm plum (`--bg-canvas #0b060a`), rose accent `#9f1249`, violet `#6b21a8`
- Text on glass: `--accent-text #f472b6`, `--text-muted #a58aa0`

## Animation
- NEVER animate layout props (width, height, top, left, margin, padding)
- NEVER animate `backdrop-filter` / blur radius per frame
- ONLY `transform`, `opacity`, `background-color`, `border-color`, `box-shadow`
- Use `motion/react` for complex animations; `tw-animate-css` for entrance
- Respect `prefers-reduced-motion` globally

## Glass
- `backdrop-filter` ONLY via `.glass[data-elev="1-4"]`
- NO ad-hoc glass rules
- Media surfaces (preview, timeline, scopes, waveforms) NEVER get glass
- Max 4 blurred surfaces; blur ≤20px

## Files
- Max 1000 lines (hard), 700 warning
- New panels → `features/panels/`
- Glass components → `components/glass/`

## Commands Before Commit
pnpm build && pnpm test && pnpm check:source-size && pnpm test:e2e && cargo test --workspace

## Review
- Different agent reviews each PR
- Screenshots required for UI changes
- Visual regression baseline updated if intentional
```

### Required New Scripts
```json
// package.json additions
{
  "scripts": {
    "check:contrast": "node scripts/check-contrast.mjs",
    "check:tokens": "node scripts/check-tokens.mjs",
    "check:transitions": "node scripts/check-transitions.mjs",
    "check:source-size": "node scripts/check-source-size.mjs --max 1000 --warn 700",
    "perf:glass": "playwright test tests/perf/glass-budget.spec.ts",
    "test:e2e": "playwright test tests/e2e/glass-regression.spec.ts"
  }
}
```

---

## Dependency Graph

```
Phase 0 (Spike)
    │
    ├─ Go → Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6
    │
    └─ No-Go → Lite-only path (Phase 1 + Phase 6 with shadow approximation)
```

**Critical Path:** 0 → 1 → 2 → 3 → 4 → 5 → 6
**Parallelizable:** G1c/G1d/G1e, G2a/G2b, G3a/G3b, G6a/G6b/G6c

---

## Definition of Done (Final)

- [ ] One `.glass` surface style with documented elevation scale (L1–L5)
- [ ] Zero ad-hoc `backdrop-filter`; zero `transition: all`
- [ ] Dark & light themes driven by tokens only; contrast ≥4.5:1 (CI verified)
- [ ] Panels drag, resize, snap, dock, close, restore; layouts persist per workspace
- [ ] Window resize never strands a panel
- [ ] Header controls scale with container queries; close button ≥24px
- [ ] Glass Auto/Full/Lite/Off + Animations Full/Reduced in Settings
- [ ] Auto mode selects from startup probe; downgrades during playback/export
- [ ] `pnpm test:e2e` + contrast/token/transition/size checks run in CI and pass on `main`
- [ ] `docs/glass-spike.md`, `docs/glass-tokens.md`, `docs/glass-performance.md`, `docs/glass-accessibility.md`, `docs/glass-architecture.md`, `docs/glass-qa-checklist.md` committed
- [ ] `AGENTS.md` at repo root

---

## Emergency Rollback Plan

If any phase introduces unrecoverable regression:

```bash
# 1. Instant kill switch (no deploy needed)
VITE_GLASS_ENABLED=false pnpm tauri build

# 2. Git revert to last known-good tag
git revert --no-commit <bad-merge-commit>..HEAD
git commit -m "revert: rollback glass UI changes"

# 3. Hotfix branch from stable tag
git checkout -b hotfix/glass-regression v0.x.x-stable
```

---

## Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Tech Lead | | | |
| UI/UX | | | |
| QA Lead | | | |
| Rust Engineer | | | |

---

*Generated from glass-ui-plan-v2.md + skill analysis. Update this document as phases complete.*
