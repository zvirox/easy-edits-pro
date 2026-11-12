# UI Layout & Workspace Consistency Remediation Plan

**Date:** 2026-11-12  
**Status:** Adopted — active remediation track (`UI-P0..UI-P7`)  
**Checklist:** [AI_NATIVE_PLAN_CHECKLIST.md](../AI_NATIVE_PLAN_CHECKLIST.md#ui-layout--workspace-consistency-remediation--ui-p0ui-p7)  
**Design baseline:** [GLASS_CARDS_PLAN.md](../GLASS_CARDS_PLAN.md) · [design-qa.md](../design-qa.md)

---

## 1. Scope

This plan remediates layout, overflow, window-chrome, workspace-identity, density, and responsive-shell defects across the Easy Edits Pro desktop frontend without altering core video/audio rendering, timeline semantics, DSP, or project serialization:

- **React / TypeScript UI:** `apps/saturn-tauri/ui/src/` (shell, workspaces, docks, panels, preview, timeline, welcome screen, and stylesheets under `ui/src/styles/`).
- **Tauri window chrome:** `apps/saturn-tauri/tauri.conf.json`, `apps/saturn-tauri/src/main.rs`, and `apps/saturn-tauri/ui/src/features/menubar/AppHeader.tsx`.
- **Workspace shell & routers:** `WorkspaceShell.tsx`, `WorkspaceRouter.tsx`, `columnRatios.ts`, `SpecialModeShells.tsx`, `ExportWorkspace.tsx`, `ProjectWelcomeScreen.tsx`.
- **Panels & stages:** `StudioPanel.tsx`, Inspector, Telemetry/Scopes, `AudioMonitorMixer.tsx`, `FairlightMixerRack.tsx`, `ColorWheels.tsx`, `RgbCurvesEditor.tsx`, Agent/Review/Sheet docks, Program/Deliver preview monitor, and Timeline.
- **Responsive behavior:** Window sizes from the Tauri minimum (`900x620`) up to `1920x1080`.

---

## 2. Issue Register (`UI-01..UI-10`)

| ID | Priority | Area | Defect Summary | Primary Files |
|---|---|---|---|---|
| **UI-01** | **P0** | Window chrome | Duplicate native + custom window controls: `AppHeader.tsx` renders custom minimize/maximize/close controls while `tauri.conf.json` creates a decorated native window (`900x620` min, default `decorations: true`). | `tauri.conf.json`, `AppHeader.tsx`, `main.rs` |
| **UI-02** | **P0** | Panel scrolling | Dense panel content in Inspector, Telemetry/Scopes, and Mixer becomes hidden or unscrollable because nested flex/grid containers lack `min-height: 0` / `min-width: 0` and fixed-header + scroll-body containment. | `StudioPanel.tsx`, `02-editor.css`, `145-dock-modes.css` |
| **UI-03** | **P0** | Horizontal clipping | Color wheels (`ColorWheels.tsx`) clip horizontally in constrained left docks, while Fairlight (`FairlightMixerRack.tsx`, `AudioMonitorMixer.tsx`) renders narrow cramped mixer strips inside a wide empty rack. | `ColorWheels.tsx`, `FairlightMixerRack.tsx`, `AudioMonitorMixer.tsx`, `145-dock-modes.css` |
| **UI-04** | **P1** | Preview monitor | Program and Deliver preview monitors are small or over-constrained by fixed margins, surrounding headers, and dock/timeline proportions, wasting available stage area. | `02-editor.css`, `03-editor.css`, `140-shell.css`, `ExportWorkspace.tsx` |
| **UI-05** | **P1** | Workspace identity | Persistent-shell workspace switches keep identical outer chrome, making active workspace identity unclear at a glance (beyond subtle header tab styling). | `WorkspaceShell.tsx`, `AppHeader.tsx`, `140-shell.css` |
| **UI-06** | **P1** | Spacing / insets | Oversized outer insets, card margins, and inner padding consume disproportionate space on laptop (`1280x800`, `1366x700`) and minimum (`900x620`) viewports. | `01-editor.css`, `02-editor.css`, `140-shell.css`, `ProjectWelcomeScreen.tsx` |
| **UI-07** | **P1** | Vertical budget | Vertical competition between the Program monitor, tool rail, timeline header/ruler/tracks, and status bar squeezes preview height and track lanes on short viewports (`620px`–`768px` height). | `140-shell.css`, `150-timeline-refresh.css`, `02-editor.css` |
| **UI-08** | **P1** | Welcome screen | Crowded Welcome recovery status banner and abrupt project-path truncation obscure recent-project identification and recovery actions. | `ProjectWelcomeScreen.tsx`, `01-editor.css` |
| **UI-09** | **P2** | Panel density | Inconsistent padding, header heights, font sizes, and control spacing across Edit, Color, Effects, Fairlight, Keyframes, and Agent/Review/Sheet panels. | `02-editor.css`..`18-editor.css`, `145-dock-modes.css`, `180-review-rounds.css` |
| **UI-10** | **P2** | Responsive docks | Hard `minmax(260px, ...)` dock tracks and abrupt `@media (max-width: 1280px)` hiding of `.shell-dock-right` (`display: none`) make Inspector/Agent/Review/Sheet unreachable without warning on narrow or short windows. | `140-shell.css`, `columnRatios.ts`, `WorkspaceRouter.tsx` |

---

## 3. Target Layout Principles & Shared CSS Contracts

### Ten Target Layout Principles

1. **Single window chrome authority:** Exactly one set of window controls exists per runtime platform (native OS titlebar by default on Linux/Tauri unless custom frameless chrome is explicitly enabled and verified without duplication).
2. **Viewport-locked root shell:** The application fills `100dvh` with `overflow: clip` on `.app-shell`; the document body never scrolls.
3. **Bounded persistent workspace skeleton:** `.workspace-shell` owns row/column tracks (`shell-top` and `shell-nle-row`) where every grid and flex child declares `min-height: 0` and `min-width: 0` so no child forces track blowout.
4. **Fixed-chrome, scrollable-body panels:** Every panel uses one `.panel-shell` / `StudioPanel` contract: non-shrinking header/toolbar/footer (`flex: 0 0 auto`) and an independently scrollable body (`flex: 1 1 auto; min-height: 0; overflow: auto`).
5. **Media-first stage, glass-free canvases:** `.preview-stage` maximizes available space and renders video/canvas with `object-fit: contain`. Never apply `.glass` or `backdrop-filter` over video, scopes, or audio meter canvases.
6. **Intrinsic horizontal strips & adaptive racks:** Color wheels and Fairlight channel strips size from intrinsic minimums with clean responsive wrapping or intentional horizontal scroll containers (`overflow-x: auto`) — never silent clipping or dead whitespace.
7. **Clear workspace identity:** Switching modes updates unmistakable mode cues (header switcher, workspace badge/title context, and focused dock presets) while preserving persistent monitor/timeline/meter geometry.
8. **Proportional density & token-only spacing:** Insets, margins, gaps, and control sizes scale via existing CSS tokens without ad-hoc pixel bloat on `1280x800` or `900x620` screens.
9. **Graceful compact-window access:** Narrow (`<=1280px`) and short (`<=720px`) windows provide explicit dock toggle/drawer/tab access so Inspector, Agent, Review, and Sheet remain usable rather than silently disappearing.
10. **Matrix-verified invariance:** Every layout change is verified across all 6 viewports and 9 workspaces in live Playwright runs and visual inspection before acceptance.

### Illustrative Shared CSS Contracts

These contracts illustrate the target layout structure using existing design tokens (no hard-coded colours or global `!important`):

```css
/* Root window shell: viewport-locked, zero document scroll */
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  width: 100%;
  min-width: 0;
  min-height: 0;
  overflow: clip;
}

/* Persistent 2-row workspace shell */
.workspace-shell {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  display: grid;
  grid-template-rows: minmax(0, 1.05fr) minmax(0, 1fr);
  overflow: hidden;
}

/* Fixed header/toolbar/footer + independently scrollable body */
.panel-shell,
.studio-panel {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.panel-shell__header,
.panel-shell__toolbar,
.panel-shell__footer,
.studio-panel-header {
  flex: 0 0 auto;
  min-width: 0;
}

.panel-shell__body,
.studio-panel-body.is-scrollable {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  overflow: auto;
}

/* Media stage: aspect-safe containment, raw surface (no .glass) */
.preview-stage {
  position: relative;
  display: grid;
  place-items: center;
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.preview-stage > video,
.preview-stage > img,
.preview-stage > canvas {
  max-width: 100%;
  max-height: 100%;
  width: 100%;
  height: 100%;
  object-fit: contain;
}
```

---

## 4. Phased Remediation Plan (`UI-P0..UI-P7`)

| Phase | Name | Tickets | Scope & Deliverable |
|---|---|---|---|
| **UI-P0** | Baseline, reproduction & layout inventory | `UI-01..UI-10` | Read-only audit across source + live Playwright probe (`docs/audit/ui-layout-baseline.md`): panel classification inventory, bounding-rect/overflow measurements across viewport & workspace matrix, and baseline screenshots. Zero product code changes. |
| **UI-P1** | Window chrome & root shell authority | `UI-01` | Remove duplicate native+custom window controls (`AppHeader.tsx` vs `tauri.conf.json` / `main.rs`) and lock `.app-shell` to `100dvh` with `overflow: clip` and zero root scrollbar leakage. |
| **UI-P2** | Panel shell & scroll containment | `UI-02` | Enforce `.panel-shell` / `StudioPanel` `min-height: 0` + `min-width: 0` + fixed header/toolbar/footer + `overflow: auto` body across Inspector, Telemetry/Scopes, Mixer, and Agent/Review/Sheet docks. |
| **UI-P3** | Horizontal strip, Color wheels & Fairlight rack unclipping | `UI-03` | Fix `ColorWheels.tsx` horizontal clipping in left/narrow docks and rebalance `FairlightMixerRack.tsx` / `AudioMonitorMixer.tsx` strip widths, rack fill, and horizontal scroll behavior. |
| **UI-P4** | Preview stage, vertical proportions & insets | `UI-04`, `UI-06`, `UI-07` | Maximize Program & Deliver `.preview-stage` with clean `object-fit: contain` framing; trim oversized outer insets/margins; balance monitor vs tool rail vs timeline vertical allocation on short screens. |
| **UI-P5** | Workspace identity & Welcome screen clarity | `UI-05`, `UI-08` | Add unmistakable active workspace identity cues during persistent-shell transitions; declutter Welcome recovery status and fix path truncation (`ProjectWelcomeScreen.tsx`). |
| **UI-P6** | Density harmonization & responsive dock degradation | `UI-09`, `UI-10` | Standardize token-driven control/panel density across suites; replace abrupt `display: none` dock hiding at `<=1280px` with responsive drawer/tab access so all dock tools stay reachable down to `900x620`. |
| **UI-P7** | Full matrix regression hardening & sign-off | `UI-01..UI-10` | Lock automated Playwright geometry/overflow/visibility tests across the full viewport × workspace matrix, complete visual & accessibility inspection, and finalize docs/QA evidence. |

---

## 5. Verification Matrices

### Viewport Matrix

Every UI phase must verify zero unintended document/container overflow, zero overlapping interactive controls, and reachable primary actions across all six viewports:

| Viewport | Profile | Key Risk Covered |
|---|---|---|
| `900x620` | Tauri `minWidth` × `minHeight` | Tightest vertical & horizontal competition; dock access without overlap |
| `1024x768` | Compact XGA / split desktop | Narrow-width dock degradation (`UI-10`) and Color/Fairlight clipping (`UI-03`) |
| `1280x800` | Default Tauri window / 13" laptop | Right-dock boundary (`1280px` breakpoint) and preview-vs-timeline ratio |
| `1366x700` | Short HD laptop | Vertical squeeze (`UI-07`), timeline track visibility, header/status bar budget |
| `1600x900` | Mid desktop | Proportional dock widths, Fairlight rack fill, and density consistency |
| `1920x1080` | Full HD desktop | Large-canvas `object-fit: contain` scaling and rack/panel alignment |

### Workspace Matrix

Every verification run covers:
1. `Welcome` (initial + recovery/recent project states)
2. `Edit` (Media Pool, Program Monitor, Inspector / Agent / Review / Sheet tabs, Timeline)
3. `Media`
4. `Color` (Wheels, Curves, Qualifier, Scopes)
5. `Effects`
6. `Fusion`
7. `Audio` / `Fairlight` (Channel strips, Master/Submix rack, Automation)
8. `Keyframes` / `Curves`
9. `Deliver` (Export settings, preview monitor, queue)

---

## 6. Automated & Visual Browser Verification Rules

Before any phase `UI-P0..UI-P7` can be ticked `[x]` in `docs/AI_NATIVE_PLAN_CHECKLIST.md`:

1. **Automated Playwright geometry & overflow check** across the Viewport Matrix × Workspace Matrix:
   - Document root has zero unintended scroll (`documentElement.scrollWidth <= innerWidth`, `documentElement.scrollHeight <= innerHeight`).
   - All `.panel-shell` / `.studio-panel` headers remain visible and non-overlapping; scrollable bodies actually scroll when content exceeds container height.
   - Zero horizontally clipped interactive controls (Color wheels, mixer strips, transport buttons, header menus).
   - `.preview-stage` maintains non-zero bounding area and `object-fit: contain` without overflowing its parent.
   - Zero console/page errors.
2. **Live visual / browser inspection:** Rendered screenshots across key viewports (`900x620`, `1024x768`, `1280x800`, `1366x700`, `1920x1080`) and workspaces are inspected to confirm visual hierarchy, contrast, glass framing, and no truncated/overlapping labels.
3. **Repository frontend & safety gates:**
   - `pnpm build` (TypeScript check + Vite build)
   - `pnpm test` (unit tests)
   - `pnpm check:glass` (`check:contrast`, `check:styles`, `check:tokens`, `check:token-freeze`)
   - `pnpm check:source-size --max-lines=800`
   - `pnpm test:e2e` (all mocked-backend E2E areas green, 0 browser errors)

---

## 7. Definition of Done (12 Criteria)

1. Single non-duplicated window chrome (`UI-01`) verified on Linux/Tauri and browser harness.
2. Zero root document scrollbars across `900x620`..`1920x1080` (`100dvh` + `overflow: clip`).
3. All panels enforce fixed header/toolbar/footer + `overflow: auto` body with `min-height: 0` / `min-width: 0` (`UI-02`).
4. Color wheels and Fairlight mixer strips render unclipped and proportional across all supported widths (`UI-03`).
5. Program and Deliver preview stages maximize usable canvas with `object-fit: contain` and zero glass blur over media (`UI-04`).
6. Active workspace identity is immediately clear across persistent-shell switches (`UI-05`).
7. Outer insets, card gaps, and panel padding match compact token scales without crowding or wasted gutters (`UI-06`).
8. Monitor, tool rail, and timeline maintain balanced usable height down to `900x620` and `1366x700` (`UI-07`).
9. Welcome recovery status and project paths display cleanly without cramped overlap or unreadable truncation (`UI-08`).
10. Control and panel density is consistent across all 9 workspaces and right-dock tabs (`UI-09`).
11. Compact windows (`<=1280px`) provide accessible dock switching/drawers so no panel functionality is silently lost (`UI-10`).
12. All automated Playwright matrix checks, visual inspection, `pnpm build`, `pnpm test`, `pnpm check:glass`, `pnpm check:source-size --max-lines=800`, and `pnpm test:e2e` pass with zero browser errors.

---

## 8. Guardrails

- **AGENTS.md UI & code rules remain binding:** design tokens only (`check:styles` / `check:token-freeze`), never `transition: all`, `.glass` only via `.glass` classes, never glass over video/scope/meter surfaces, and `<= 800` lines per source file (`check:source-size --max-lines=800`).
- **Rust core owns project state:** UI layout fixes must never fork project/timeline/viewport state client-side or alter engine serialization, rendering, or DSP behavior.
- **No global `!important` overrides:** Fix cascade and flex/grid sizing at the component/shell contract level.
- **Provenance & naming:** No third-party code pasting, no third-party product names or trademarks in UI labels/identifiers, and never use ArtCraft or FilmCraft names/logos.
- **One lane per packet:** UI/CSS/visual test work executes in Lane 1; coordinate any `tauri.conf.json` / `main.rs` window-chrome change cleanly with Lane 3/4.
