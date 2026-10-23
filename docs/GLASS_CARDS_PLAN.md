# EEP UI Correction Plan v4: Fixed Glass Cards + One Consistent Workspace Shell

> **v4 changes:** (1) **Colours are locked.** No existing colour token changes. (2) Merged the findings from your research PDF (`Easy_Edits_Pro_Overhaul_Plan.pdf`); see §13. (3) Per-workspace column presets are allowed (FilmCraft does this). Timeline, toolbox, meters and bars stay identical.

Replaces v3, `glass-ui-plan.md`, `glass-ui-plan-v2.md` and the floating-panel parts of `final-implementation-plan.md` (WS-C).  
Visual target mock: `eep-glass-cards-mock.html` (open it in a browser, click the workspace tabs, resize the window).

---

## 0. The Corrected Brief

1. **Fixed layout. Nothing moves.** No dragging, no close, minimise or pin buttons, no bottom dock for hidden panels, no saved panel layouts.
2. **"Floating" is a visual style, not behaviour.** The window has a frosted, tinted, colourful backdrop. Every block (header, media, program monitor, inspector, toolbox, timeline, meters, status bar) is a rounded glass card that looks lifted off the backdrop by depth: border, soft shadow, inner highlight. This is the Motrix look.
3. **Colours stay exactly as they are.** Same plum/magenta surfaces, same accent, same text colours, same stage gradient. Only spacing, shape and elevation change.
4. **Size-responsive.** Cards reflow and shrink with the window. They never overlap each other.
5. **One shell for every workspace (FilmCraft).** Program monitor, toolbox, timeline and meters sit in the same slots in every workspace. A workspace only changes the *tab content* of the side cards, and may pick a different column ratio from a small fixed table.
6. **Organised, not cluttered.** Grouped toolbar, compact track headers, consistent spacing.

The prior drag/dock/snap/minimise panel engine is to be **deleted**, not tuned.

---

## 1. What Is Wrong in Current EEP Screenshots

| # | Observation | Cause | Fix in this plan |
|---|---|---|---|
| 1 | Looks flat, little depth. | Panels touch or overlap (0 px gap), so the stage never shows between them, and there is no consistent edge highlight. The stage itself is very dark by design. | §3: gaps, radius, inner highlight, layered shadow, with **existing colours**. Optional one-variable glow boost |
| 2 | Captions and Essential Graphics panels sit on top of the Media pool, Inspector and Program monitor. The monitor is half hidden. | `PanelHost` floats panels over the work area | §4: delete floating engine; panels become fixed tabs |
| 3 | Bottom-right "PANELS / Markers / Reset layout" strip covers the status bar. "Native GPU…" is clipped. | Panel dock is positioned over the footer | Removed with the engine |
| 4 | Every workspace is a different page. Color has a huge black monitor, Fairlight has its own mini timeline, Effects has its own monitor with a panel on top, Curves has its own graph page. Preview and timeline move or vanish. | Eight separate `*Workspace.tsx` files, each with its own layout | §5: one `WorkspaceShell`, workspaces become content presets |
| 5 | Timeline toolbar is about 30 same-looking icons in one row. | No grouping, and edit actions mixed with view controls | §6.1 grouped toolbar; undo/redo/delete move to menus and shortcuts |
| 6 | Track header is huge: "Track", "Subtitles", V1 button, lock, M, S, VOL slider, PAN slider. | Gain and pan sliders live inside every header | §6.2: header = target, lock, name, M/S. Gain/pan go to the Inspector or a hover popover |
| 7 | Toolbox is tall icons scattered in a block with extra labels. | Unfocused block layout | §6.3: slim single card, grouped, with dividers |
| 8 | Top bar: "Help" is overlapped by icons. | Fixed widths, no overflow handling | §7 bug list |
| 9 | Several buttons are empty squares (Captions "Cue", the square before SRT, Essential Graphics "Text"). | Missing icon names in `Icon.tsx` | §7 |
| 10 | Fairlight: "OFF REA TOU LAT WRI" truncated, "MASTER (S" truncated, Master bus and channel strips cut off. | Fixed widths and clipped overflow | §7 |
| 11 | Program monitor is black with "Offline File". | Test media offline | §7: proper offline state card, not a black hole |

---

## 2. Design Rules

* **3 surface levels only:**
  * **Stage:** the tinted backdrop. It is the only thing with colour.
  * **Card (L1):** glass block on the stage.
  * **Inset (L2):** fields, chips, thumbnails inside a card.
* **One gap** (12px) between cards and around the window. **One radius** (16px for cards, 10px for insets).
* **One header style** per card: tabs on the left, underline on the active tab, 38px high, hairline under it.
* **Accent used sparingly:** active tab underline, selected tool, playhead, primary button.
* **Labels** are small caps (10.5px, wide tracking). Numbers are tabular.
* **No overlap, no z-index games** except menus, context menus and modals.

---

## 3. Tokens: Reuse Existing, Add Only Shape and Elevation

**Do not edit any colour value.** Uses only these existing tokens:

| Role | Existing token (file) | Value |
|---|---|---|
| Stage | `--bg-stage` (`00-tokens-glass.css`) | existing 3 radial glows + dark linear gradient |
| Card | `--glass-l1` | `rgba(24,13,23,.56)` |
| Card border | `--glass-border` | `rgba(244, 215, 235, 0.14)` |
| Top highlight | `--glass-highlight` | `rgba(255, 255, 255, 0.08)` |
| Shadow | `--glass-shadow` | `0 18px 48px rgba(0, 0, 0, 0.45)` |
| Blur / saturate | `--blur-l2`, `--glass-saturate` | `14px`, `145%` |
| Text | `--text-primary / -secondary / -muted` | `#faf2f7 / #d4bccf / #a890a0` |
| Accent | `--accent-text`, `--accent-primary`, `--accent-text-violet` | `#f472b6`, `#9f1249`, `#c084fc` |

Light theme and Lite/Off modes keep their existing values too.

**New tokens (shape only, no colour):**

```css
:root {
  --gap: 12px;
  --radius: 16px;
  --radius-sm: 10px;
  --card-inset: rgba(255, 255, 255, 0.045);
}
.glass {
  background: var(--glass-l1);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius);
  box-shadow: inset 0 1px 0 var(--glass-highlight), var(--glass-shadow);
  backdrop-filter: blur(var(--blur-l2)) saturate(var(--glass-saturate));
  -webkit-backdrop-filter: blur(var(--blur-l2)) saturate(var(--glass-saturate));
}
```

Rules for the agent:
* Keep Full / Lite / Off modes and `data-webkit-gtk-slow` fallback. In Lite and Off, drop `backdrop-filter`.
* Keep `prefers-reduced-transparency` and the contrast gates (`check:contrast`).
* `check:tokens` must show **zero changed colour values** in the PR diff.
* Never place `backdrop-filter` over the video surface; the Program card's video area is opaque.

---

## 4. Delete the Floating-Panel Engine

Remove, in one PR, together with their tests:

```
features/panels/FloatingPanel.tsx      features/panels/usePanelDrag.ts
features/panels/PanelHost.tsx          features/panels/usePanelResize.ts
features/panels/LayoutPresetsDialog    features/panels/snapping.ts (+ test)
features/panels/docking.ts (+ test)    features/panels/zOrder.ts (+ test)
features/panels/presets.ts, presetStore.ts (+ test), layouts.ts, usePanelLayout.ts
styles/140-panels.css, styles/171-layout-presets.css
```

* Delete `save_panel_layout` / `load_panel_layout` Tauri commands, their catalogue entries, `docs/COMMANDS.md` rows, and JSON catalogue. Run `pnpm catalogue:sync` and `check:commands`.
* Remove `PanelHost` from `EditWorkspace`, `ColorWorkspace`, `EffectsWorkspace`, and `OllamaChatDrawer.tsx`.
* Remove `.is-floating-panels`, `editor-panel-layer`, `panel-rail`, "Reset layout", and "PANELS" dock.
* Keep the panel **contents** (`MediaPanel`, `InspectorPanel`, `MarkersPanel`, `CaptionsPanel`, `GraphicsPanel`, `TrimMonitorPanel`). Only the window chrome goes.
* Clear stale localStorage layout keys on launch.

Replacement primitive:

```tsx
<DockCard id="left" tabs={[{id:"project", title:"Project", render}, ...]} />
```

---

## 5. One Shell, Many Workspaces (FilmCraft Model)

```
┌───────────────────────── Top bar card: brand · menus · workspace tabs · actions ────────────────────────┐
├─ Dock L card ──────┬──────── Program monitor card ────────┬─ Dock R card ─────┤   row 1  (≈ 52%)
│ tabs (per workspace)│  video · timecodes · scrub · transport │ tabs (per workspace)│
├──┬─────────────────┴───────── Timeline card ───────────────┴───────────┬──────┤   row 2  (≈ 48%)
│Tb│ sequence tabs · grouped toolbar · ruler · track headers · lanes     │Meter │
├──┴──────────────────────── Status bar card ─────────────────────────────┴──────┤
```

* **Identical in every workspace (same slot, same size):** top bar, toolbox, timeline, meters, status bar.
* **Column ratio preset table:** `edit: [28, 48, 24]`, `audio: [40, 36, 24]`, `color: [34, 42, 24]`.
* **Changes per workspace:** which tabs appear in Dock L and Dock R.

| Workspace | Dock L tabs | Dock R tabs |
|---|---|---|
| Edit | Project · Media Browser · Effects | Inspector · Titles · Markers |
| Assembly | Project · Markers | Source · Inspector |
| Color | Scopes · Project | Color (wheels, curves, HSL, sliders) |
| Audio | Mixer · Project | Essential Sound · Inspector |
| Captions | Transcript · Project | Captions · Graphics |
| Effects | Project | Effects · Effect Controls |

Wide content handling:
* **Curves / Keyframes:** Dock L "Curves" tab, or timeline tab (`Timeline | Curves`). Interpolation in Dock R.
* **Fairlight:** Dock L "Mixer" with wide preset; channel strip scrolls inside card. Automation lane in timeline.
* **Fusion node graph:** Tab in centre card next to Program. Parameters in Dock R.
* **Deliver / Export:** Delivery presets and export settings in Dock L, queue/interchange in Dock R, Program monitor for In/Out ranges, timeline kept. Synthetic "Sequence clips reel" deleted.

---

## 6. Timeline, Tracks and Toolbox Organization

### 6.1 Timeline Toolbar (40px)
* **Group 1 (Edit modes):** Snap · Linked selection · Ripple
* **Group 2 (Marking):** Add marker · Mark in · Mark out
* **Group 3 (Tracks):** Add track
* **Group 4 (Right side):** Zoom out · zoom slider · Zoom in

### 6.2 Track Header (44px Rows)
`[target V1/A1] [name …………] [lock] [eye or M S]`
* Coloured target buttons (video blue, audio green).
* Gain and pan move out to Inspector or Audio Mixer.
* Video and audio groups separated by thin divider.

### 6.3 Toolbox (52px Single Card)
* Left of the timeline, single vertical column.
* Groups separated by hairlines: **Select / Track select** · **Ripple / Razor / Slip** · **Hand / Text**.

### 6.4 Program Monitor & Meters
* Program card: tab, Fit, quality, video viewport, timecode left and duration right with scrub bar between, transport row below.
* Meters card: dual vertical peak bars, M / I / TP readout, 92px wide.

---

## 7. Bug Fix Checklist

1. Top bar: reserve space for menus; collapse into "…" menu below 1300px.
2. Add missing icons in `Icon.tsx`.
3. Status bar is a normal grid row and must never be covered.
4. Fairlight: automation modes as segmented control; channel strip scrolls inside card.
5. Offline media: Program monitor shows "Offline media – Relink" card, not a black box.
6. Curves card: "0 keyframes" styled as clean label.
7. Avoid `overflow: hidden` without scroll fallbacks on card bodies.

---

## 8. Implementation Phases

* **G0. Freeze:** Safety tag `pre-glass-cards`.
* **G1. Tokens & Glass Card:** New tokens (§3), `.glass`, `.card-h`, `.tab`, `.chip`, `.field`.
* **G2. Delete Floating Engine & Add DockCard:** Purge MDI engine files; run `check:commands`.
* **G3. WorkspaceShell:** Implement persistent 2-row grid and dock tabs.
* **G4. Timeline & Toolbox Rework:** 44px track headers, 52px toolbox, grouped toolbar.
* **G5. Special Modes:** Dock Fairlight, Fusion, Color, Deliver inside shell without full-page unmounting.
* **G6. Polish & Motion:** Clean transitions (220ms), bug fixes, reduced-motion compliance.
* **G7. Tests & Acceptance:** Geometry invariance and e2e smoke verification.

---

## 9. Functional Workstream F (Parallel to G)

| Card | Item | Files | Action |
|---|---|---|---|
| **F1** | Single shared clip-selection store | `app/useTimelineOperations.ts`, `features/inspector/*` | Auto-select first V1 clip on project load; unify selection across all docks |
| **F2** | Offline media relink | `features/media-pool/MediaPanel.tsx`, `app/useMediaOperations.ts` | One-click relink action; verify media server initialization |
| **F3** | Program monitor transport | `features/preview/PreviewSection.tsx`, `app/usePlayback.ts` | 10 transport buttons + blue playhead timecode + Fit zoom |
| **F4** | Keyframes in render export | `crates/saturn-core/src/job.rs`, `saturn-render` | Bake transform, opacity, and audio automation into MP4 output |
| **F5** | Hardware encoder probe | `job.rs`, `app/useJobs.ts` | Probe once asynchronously on startup and cache |
| **F6** | Fairlight cleanup | `workspaces/Fairlight*` | Remove placeholder slots; bind active tracks to EQ/Compressor |

---

## 10. Master Agent Directives

1. **Sequential Phase Locking:** Phases must be executed in order and verified before moving forward.
2. **Zero MDI Window Controls:** Never render 📌, -, or X window buttons on panel headers.
3. **Zero Overlapping Popups:** No `position: absolute` floating feature panels. Only dialogs in `features/dialogs/` may be modals.
4. **GPU-Safe Frosted Glass:** `.glass` applied only to cards over the static ambient stage; never over live video.
5. **Persistent Core NLE Skeleton:** Never unmount Program, Timeline, Toolbox, or Meters when switching workspace tabs.
6. **Compact 44px Track Headers:** Never place horizontal VOL/PAN sliders in track headers.
