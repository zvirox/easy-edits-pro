# Easy Edits Pro — Glass UI Phased Implementation Plan v2.1

> **Source:** `glass-ui-plan-v2.md` + skill analysis (glassmorphism, progressive-blur, baseline-ui, reverse-engineer-anything) + risk mitigation strategies
> **Status:** Ready for execution
> **Total estimated effort:** 2.5–3 weeks (13–18 work days)

---

## Phase Overview

| Phase | Duration | Focus | Gate |
|-------|----------|-------|------|
| **0** | 0.5–1 day | Feasibility spike (WebKitGTK performance) | **Go/No-go** |
| **1** | 1–2 days | Token cleanup, contrast, CI gates | All contrast ≥4.5:1 |
| **2** | 1–2 days | Glass surfaces, background, rename CSS | 0 ad-hoc backdrop-filter |
| **3** | 1–2 days | Animation hygiene, motion/react | 0 `transition: all` |
| **4** | 3–5 days | Floating panel engine | Drag/resize/snap/dock/persist |
| **5** | 2–3 days | Responsiveness, container queries | Breakpoints work |
| **6** | 2 days | Performance, a11y, QA, CI | `test:e2e` green |

**Total:** ~12–15 work days + 1–2 days buffer = **2.5–3 weeks**

---

## Phase 0 — Feasibility Spike (½–1 day) ⚡ **CRITICAL GATE**

### Objective
Validate `backdrop-filter` performance in Tauri's WebKitGTK **before committing to glass**.

### Tasks
- [ ] Create throwaway branch `feat/glass-spike`
- [ ] Implement minimal test: `--bg-stage` background + 3 `.glass` panels (L1/L2/L3) + 1 draggable panel + playing `<video>`
- [ ] Build AppImage: `pnpm tauri build`
- [ ] Measure frame time while dragging + video playing on:
  - [ ] Your machine (high-end)
  - [ ] Low-end laptop (Intel UHD / AMD Vega / integrated graphics)
- [ ] Test **inside Tauri window** (WebKitGTK), not Chrome
- [ ] Document results in `docs/glass-spike.md`

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
- [ ] Pre-build **shader fallback** (CSS gradient + shadow, zero `backdrop-filter`) for slow paths
- [ ] Rust-side GPU detection (`src-tauri/src/gpu_detect.rs`) to inject `[data-webkit-gtk-slow]` class at startup
- [ ] Feature flag `VITE_GLASS_ENABLED` for instant kill switch

---

## Phase 1 — Token Cleanup & Contrast (1–2 days) 🎨 **FOUNDATION**

### Objective
Establish token-only color system with WCAG AA contrast; eliminate all raw literals.

### Work Packets

#### G1a — Glass Token File
- [ ] Create `styles/00-tokens-glass.css` (imported right after `01-editor.css`)
- [ ] Define tokens from plan + accessibility additions:
  ```css
  --accent-text: #f472b6;           /* 7.15:1 on surface */
  --accent-text-hover: #f9a8d4;
  --text-muted: #a58aa0;            /* 6.08:1 on surface */
  --glass-l1..l4, --glass-border, --glass-highlight, --glass-shadow
  --glass-saturate: 140%;
  --blur-l1..l4, --ease-spring, --ease-out
  --dur-fast: 120ms, --dur-med: 220ms, --dur-slow: 360ms
  --bg-stage (radial glows + canvas)
  ```
- [ ] Add `[data-glass="lite|off"]` and `@media (prefers-reduced-transparency/motion)` fallbacks

#### G1b — Contrast Fixes
- [ ] Switch ~29 rules using accent (`#9f1249`, `#6b21a8`) as **text** → `--accent-text`
- [ ] Switch ~139 rules using `--text-muted` (`#7d6377`) → lifted `--text-muted`
- [ ] Verify: no small text uses dark rose/violet

#### G1c — Contrast CI Gate
- [ ] Create `scripts/check-contrast.mjs`:
  - Parses all token blocks (dark + light)
  - Computes contrast for every text/background pair
  - Fails CI if any <4.5:1 (body) or <3:1 (large text/icons)
- [ ] Add to CI: `pnpm check:contrast` runs on every PR

#### G1d — Literal Elimination
- [ ] Batch replace raw hex/rgba in `styles/` with tokens (by file)
- [ ] Delete all 50 `[data-theme="light"]` overrides in `16-editor.css` once light tokens cover them
- [ ] Target: **0 raw hex/rgba in `styles/`** (except token files)

#### G1e — Lint Gates
- [ ] Add ESLint/Stylelint rules (fail CI):
  - `no-raw-colors-in-styles` — raw hex/rgba outside token files
  - `no-transition-all` — catches `transition: all`
  - `max-lines-per-file` — 1000 hard, 700 warning

### Acceptance Criteria
- [ ] `pnpm check:contrast` passes
- [ ] 0 raw hex/rgba in `styles/` (grep count = 0)
- [ ] 0 `transition: all` in codebase
- [ ] 0 `[data-theme="light"]` overrides in `styles/`
- [ ] Light theme works via tokens only

### Risk Mitigation (NEW)
- [ ] **Contract tests** for token values: `tests/tokens/contrast.test.ts`
- [ ] **Living docs**: `docs/glass-tokens.md` auto-generated from token files; CI fails if code ≠ docs

---

## Phase 2 — Glass Surfaces & Background (1–2 days) 🪟

### Objective
Apply single `.glass` system; remove all ad-hoc glass; establish elevation scale.

### Work Packets

#### G2a — CSS Rename (Rename-Only Commit)
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
- [ ] Create `styles/120-glass.css` with `.glass` + `data-elev="1-4"` from plan
- [ ] Apply to components:
  - [ ] `StudioPanel` (L1)
  - [ ] `AppHeader` menus, popovers (L3)
  - [ ] `BatchQueueDrawer` (L3)
  - [ ] `OllamaChatDrawer` (L3)
  - [ ] `features/dialogs/*` modals (L4)
  - [ ] Tooltips, toasts (L5)
  - [ ] Status bar (L1)
- [ ] **Delete 11 ad-hoc `backdrop-filter` rules** (menus, banners, overlays, job drawer, welcome, scopes...)

#### G2c — Background & Gaps
- [ ] Put `--bg-stage` on app root (`#root` or `body`)
- [ ] Add 8–12px gaps between docked panels (CSS gap in panel container)
- [ ] Verify background glows visible

#### G2d — Media Surface Protection
- [ ] **Audit**: Ensure NO glass on:
  - [ ] Preview frame (`PreviewFrame.tsx`)
  - [ ] Timeline clip area (`TimelineSection.tsx`)
  - [ ] Scopes (`ScopesWorkspace.tsx`)
  - [ ] Waveforms (`WaveformDisplay.tsx`)
- [ ] Add ESLint rule: `no-glass-on-media-components`

### Acceptance Criteria
- [ ] Single `.glass` class + `data-elev` used everywhere
- [ ] 0 ad-hoc `backdrop-filter` rules (grep = 0)
- [ ] Screenshots: dark/light × Full/Lite/Off for each workspace
- [ ] Media surfaces render sharp, no blur

### Risk Mitigation (NEW)
- [ ] **Architectural boundary**: `GlassPortal` component wraps all glass UI; media renders outside
- [ ] **Visual regression thresholds** per component (PreviewFrame = 0% tolerance)

---

## Phase 3 — Animation Foundation (1–2 days) ✨

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
- [ ] Use `motion/react` (already in `package.json`) for:
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
