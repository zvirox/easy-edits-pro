# Phase 0 — UI Layout & Workspace Consistency Baseline Report

**Date:** 2026-11-12  
**Recon agent:** `@baseline-auditor` (zero product source changes)  
**Remediation plan:** [planning/ui-layout-workspace-remediation.md](../planning/ui-layout-workspace-remediation.md)  
**Scratch probe & evidence:**
- Probe script: `/home/zvirox/.gemini/antigravity/brain/02eae39c-a044-43ea-b20f-d88fba759da9/scratch/ui_baseline_probe.mjs`
- Measurements JSON: `/home/zvirox/.gemini/antigravity/brain/02eae39c-a044-43ea-b20f-d88fba759da9/scratch/ui-baseline-results.json`
- Screenshots (12 PNGs): `/home/zvirox/.gemini/antigravity/brain/02eae39c-a044-43ea-b20f-d88fba759da9/scratch/ui-baseline/` (`welcome-{900,1280,1600}`, `edit-{900,1024,1280,1920}`, `color-{1280,1366}`, `audio-1280`, `keyframes-1280`, `deliver-1280`)

---

## 1. Environment & OS / Native Window Facts

| Property | Observed Value | Source |
|---|---|---|
| OS / Desktop | KDE Plasma on Wayland | `$XDG_CURRENT_DESKTOP=KDE`, `$XDG_SESSION_TYPE=wayland`, `$WAYLAND_DISPLAY=wayland-1`, `$DISPLAY=:0` |
| Browser (Playwright) | HeadlessChrome/156.0.8078.4 | `navigator.userAgent` |
| Scale / Screen | `devicePixelRatio: 1`, `1920×1080`, 24-bit | `window.devicePixelRatio`, `screen.*` |
| Tauri window config | `1280×800` initial, `minWidth: 900`, `minHeight: 620`, `resizable: true` | `apps/saturn-tauri/tauri.conf.json:22-29` |
| Native decorations | **ON** (no `"decorations": false` in `tauri.conf.json`, no `set_decorations(false)` in `apps/saturn-tauri/src/main.rs`) | `tauri.conf.json:21-30`, `main.rs` |

> **OS/KDE vs Browser distinction:** Playwright runs Vite in headless Chromium (no KDE titlebar drawn inside the viewport). Duplicate OS titlebar controls are confirmed from `tauri.conf.json` + `main.rs` (native KDE/Wayland window frame enabled) combined with in-DOM `.window-controls` rendered by `AppHeader.tsx:559-578` (confirmed live in browser and screenshots).

---

## 2. Panel Classification Inventory

| Panel | Layout Classification | Primary Selectors | Key Source File(s) |
|---|---|---|---|
| Welcome Screen | Full-page content | `.welcome-shell`, `.welcome-panel.glass` | `apps/saturn-tauri/ui/src/workspaces/ProjectWelcomeScreen.tsx` |
| App Header (Tier 1 + Tier 2) | Fixed header strips | `.app-header`, `.system-bar-tier1`, `.workspace-bar-tier2` | `apps/saturn-tauri/ui/src/features/menubar/AppHeader.tsx`, `01-editor.css:508-691` |
| Persistent NLE Shell | Resizable 2-row grid | `.workspace-shell[data-workspace]`, `.shell-top`, `.shell-nle-row` | `apps/saturn-tauri/ui/src/workspaces/WorkspaceShell.tsx`, `140-shell.css:11-73` |
| Edit Left Dock (Media Pool / Trim) | Fixed tab header + scroll/flex body | `.shell-dock-left.dock-card`, `.card-h`, `.card-b.dock-card-body` | `EditDocks.tsx:49-73`, `DockCard.tsx`, `120-glass.css:202-251`, `135-dock.css:15-32` |
| Edit Right Dock (10 tabs) | Fixed tab header + stacked scroll/flex body | `.shell-dock-right.dock-card`, `.editor-right-column`, `.studio-panel` | `EditDocks.tsx:75-180`, `InspectorPanel.tsx:491-516`, `03-editor.css:365-396` |
| Program Monitor | Canvas/stage (non-glass) | `.shell-center`, `.monitor-unified`, `.preview-stage` | `PreviewSection.tsx`, `140-shell.css:48-61`, `120-glass.css` |
| Timeline | Horizontal strip (non-glass) | `.shell-nle-row .shell-timeline`, `.timeline` | `TimelineSection.tsx`, `140-shell.css:87-98`, `02-editor.css` |
| Timeline Tool Rail | Vertical strip | `.shell-toolbox .timeline-tool-rail` | `TimelineToolRail.tsx`, `140-shell.css:74-85`, `03-editor.css:113` |
| Master Meter | Fixed vertical strip | `.shell-meter` | `MasterMeter.tsx`, `140-shell.css:100-164` |
| Media Left Dock | Fixed header + scroll body | `.media-dock-body` | `MediaDocks.tsx`, `145-dock-modes.css:88-99` |
| Color Left Dock (Video Scopes) | Canvas/stage + controls | `.color-scopes-stage`, `.scopes-canvas-stage` | `ColorDocks.tsx`, `06-editor.css` |
| Color Right Dock (Wheels/Curves/Looks) | Resizable work area | `.color-wheels-dock-wrapper`, `.color-wheels-grid` | `ColorDocks.tsx:195-260`, `06-editor.css:310-380` |
| Fairlight Left Dock (Mixer / Master Bus) | Horizontal strip inside scroll body | `.fairlight-mixer-dock-body`, `.fairlight-strips-scroll`, `.fairlight-strips-grid`, `.fairlight-channel-strip` | `FairlightDocks.tsx`, `FairlightMixerRack.tsx`, `04-editor.css:291-345`, `145-dock-modes.css:12-29` |
| Fairlight Right Dock (Track DSP / Automation) | Fixed header + scroll body | `.fairlight-automation-dock-body` | `FairlightDocks.tsx`, `145-dock-modes.css:31-36` |
| Fusion Left/Right Docks | Canvas/stage + scroll body | `.fusion-canvas-dock-body`, `.fusion-inspector-dock-body` | `FusionDocks.tsx`, `145-dock-modes.css:46-63` |
| Keyframes/Curves Right Dock | Canvas/stage + scroll body | `.keyframes-curve-dock-body`, `.curve-editor-stage`, `.keyframes-tangent-dock-body` | `KeyframesDocks.tsx`, `145-dock-modes.css:65-86` |
| Deliver (Export) Workspace | Full-page content (3-column grid) | `.export-workspace`, `.export-destinations`, `.export-settings-panel`, `.export-preview-panel`, `.export-preview-stage` | `ExportWorkspace.tsx`, `03-editor.css:165-225`, `17-editor.css:108-110` |
| Status Bar | Fixed footer strip | `.status-bar` | `01-editor.css` / `140-shell.css` |

---

## 3. Observed Measurements & Root Causes Mapped to `UI-01..UI-10`

### UI-01 — Duplicate Native + Custom Window Controls & Header Overflow (P0)
- **Code:** `apps/saturn-tauri/tauri.conf.json:21-30` leaves native decorations enabled (`decorations` omitted; no `set_decorations(false)` in `apps/saturn-tauri/src/main.rs`), while `apps/saturn-tauri/ui/src/features/menubar/AppHeader.tsx:559-578` renders `<div className="window-controls">` with custom fullscreen and `getCurrentWindow().close()` buttons.
- **Live browser (`1280×800` `edit`):** `.window-controls` measures `x:1208, y:1, w:62, h:30` with `scrollW:139, clientW:62` (internal overflow). `.app-header` measures `w:1280` with `scrollW:1347` (`+67px` horizontal overflow); `.menu-bar` measures `w:207, scrollW:679` (`hOverflow: true`, clipping `Markers Color Graphics Audio Effects View Window Help` out of view).

### UI-02 — Hidden / Unscrollable Dense Panel Content (P0)
- **Code:** `apps/saturn-tauri/ui/src/features/inspector/InspectorPanel.tsx:491-516` stacks `.inspector-dock-panel` + `.audio-monitor-mixer-panel` inside `.editor-right-column`. `03-editor.css:365-386` sets `.editor-right-column { height: 100%; min-height: 0; overflow: hidden; }` while `.audio-monitor-mixer-panel` enforces `min-height: 260px`. `02-editor.css:238-247` sets `overflow-x: hidden` on `.studio-panel-body.is-scrollable`.
- **Live browser (`edit`):**
  - `1366×700`: right dock height `293px`; `.audio-monitor-mixer-panel` (`min-height: 260px`) crushes Inspector `.studio-panel-body` to **`296×20px` (`scrollH: 124`, `vOverflow: true`)**.
  - `1600×900`: `.studio-panel-body` crushed to **`352×20px` (`scrollH: 124`)**.
  - `1920×1080`: `.studio-panel-body` is **`429×111px` (`scrollH: 124`)**; bottom `V1` track mixer row is half-clipped with no scrollbar on `.editor-right-column`.
  - Left dock `edit` @ `900×620`: `Media pool` sub-tabs (`asset-tab-btn`) overflow by `+61px..+315px`; at `1280×800`, 4th tab icon and `Replace Clip` / `Bin` clip horizontally.

### UI-03 — Color Wheels Horizontal Clipping, Raw Inline Hex & Fairlight Mixer Strip Clipping (P0)
- **Color Wheels (`ColorDocks.tsx:195-254`, `06-editor.css:310-367`):**
  - `.color-wheels-grid` hardcodes `grid-template-columns: repeat(4, 1fr); gap: 10px;` inside `.color-wheels-dock-wrapper { overflow: hidden; }` with no wrap or `overflow-x: auto`.
  - **Live (`color`):** `.color-wheel-item` overflows right dock by **`+186px`** at `1366×700` (`.wheel-disc` `+32px`; `GAIN`/`OFFSET` wheels cut off horizontally, `LIFT`/`GAMMA` bottom halves cut off vertically), **`+93px`** at `1600×900`, and **`+110px`** at `1920×1080` (`hOverflow: true`). Left dock `Video Scopes` `.scope-tabs-bar` overflows `+91px..+194px` and `.scopes-canvas` overflows bottom `+52px..+170px`; `Ceiling: 100 IRE` overlaps scope tabs at `1366×700`.
  - **Token compliance note:** `ColorDocks.tsx:195-254` also uses raw inline hex colours (`#462d40`, `#9f1249`, `#100810`, `#ffffff`, `#a0aec0`).
- **Fairlight Mixer (`04-editor.css:291-322`, `columnRatios.ts`):**
  - `.fairlight-channel-strip` is fixed `width: 108px; min-width: 108px; height: 100%;` inside `.fairlight-strips-grid { height: 100%; min-height: 480px; width: max-content; }`.
  - **Live (`audio-1280.png` at `1280×800`):** Left dock (`303×344`) clips `V1`/`A1` strips vertically right below `PAN` (faders and meters cut off) due to `min-height: 480px`, and `OFF REA TOU LAT WRI` automation pills overflow horizontally past the `108px` strip (`WRI` clipped).

### UI-04 — Small / Constrained Program & Deliver Preview Monitor (P1)
- **Deliver (`03-editor.css:165-175, 213-218`, `17-editor.css:108-110`):** `.export-workspace` hardcodes `grid-template-columns: 260px minmax(380px, 1fr) 380px;`, locking `.export-preview-stage` inside the fixed `380px` right column.
- **Live (`deliver`):** `.export-preview-stage` stays stuck at **`320×180`** (`900×620`, `1024×768`) and **`322×181`** (`1280×800` through `1920×1080`), never scaling with viewport growth. (`ENGINE PRESET BANK` also shows `Resolves to undefinedxundefined at undefined/undefined, undefined` in `deliver-1280.png`.)
- **Program Monitor (`140-shell.css:16`):** `1.05fr / 1fr` split leaves `.shell-center` at `676×252` (`900×620`) and `632×293` (`1366×700`) for header + video stage + transport combined.

### UI-05 — Unclear Workspace Identity During Persistent-Shell Switches (P1)
- Persistent header, center monitor, and timeline remain visually uniform across `Media`, `Edit`, `Fusion`, `Color`, `Fairlight`, `Effects`, and `Curves` apart from the active Tier-2 tab pill and dock tab set (`edit-1280.png` vs `audio-1280.png`).

### UI-06 & UI-07 — Oversized Insets / Margins & Vertical Competition (P1)
- **Live (`edit`):** `1.05fr / 1fr` vertical split (`140-shell.css:16`) + card/toolbar chrome restricts `.shell-center` height to `252px` (`900×620`) and `293px` (`1366×700`), while `.shell-toolbox` (`140-shell.css:75`, `overflow: hidden`) + `03-editor.css:113` clip lower tool-rail buttons even at `1920×1080`.

### UI-08 — Welcome Recovery Status & Path Truncation (P1)
- `ProjectWelcomeScreen.tsx:215-256` renders `<small>{recoveryStatus.projectName ?? "Autosaved project"} · autosave found</small>` next to `Recover` and `<small title={item.path}>{item.path}</small>` in recent items (default mock state starts with 0 recovery/recent items, so populated-row overflow needs dedicated fixture coverage in Phase 5). At `900×620`, `.welcome-shell` measured `900×650` (`+30px` taller than `620px` viewport).

### UI-09 — Inconsistent Control / Panel Density (P2)
- `EditDockRight` (`EditDocks.tsx:94-178`) packs 10 tabs into a single `38px` `.card-h` row (`Inspector`, `Sheet`, `Agent`, `Review`, `Script`, `Telemetry`, `Markers`, `Captions`, `Essential Graphics`, `Nodes`), overflowing horizontally even at `1920×1080` (`Telemetry` cut mid-word; `Markers`, `Captions`, `Essential Graphics`, `Nodes` off-screen in `edit-1920.png`).

### UI-10 — Abrupt Dock Hiding & Hard Minimums on Narrow / Short Windows (P2)
- **Code (`140-shell.css:170-188`):** `@media (max-width: 1280px)` forces `.shell-dock-right { display: none; }`.
- **Live (`edit`):**

| Viewport | `.shell-dock-left` | `.shell-center` | `.shell-dock-right` | `.workspace-shell` | `.timeline` |
|---|---|---|---|---|---|
| `900×620` | `200×252` | `676×252` | `display:none (0×0)` | `900×516` | `724×240` (`hOverflow`) |
| `1024×768` | `225×328` | `775×328` | `display:none (0×0)` | `1024×664` | `848×312` (`hOverflow`) |
| `1280×800` | `303×344` | `953×344` | `display:none (0×0)` | `1280×696` | `1104×328` (`hOverflow`) |
| `1366×700` | `379×293` | `632×293` | `324×293` (`flex`) | `1366×596` | `1190×279` (`hOverflow`) |
| `1600×900` | `444×395` | `744×395` | `380×395` (`flex`) | `1600×796` | `1424×377` (`hOverflow`) |
| `1920×1080` | `533×488` | `898×488` | `457×488` (`flex`) | `1920×976` | `1744×464` (`hOverflow`) |

---

## 4. Prioritized Root-Cause Map for Phase 1–3 Workers

1. **Phase 1 (Window chrome & root shell — `UI-01`):** `apps/saturn-tauri/tauri.conf.json:21-30`, `apps/saturn-tauri/src/main.rs`, `apps/saturn-tauri/ui/src/features/menubar/AppHeader.tsx:559-578`, `apps/saturn-tauri/ui/src/styles/01-editor.css:475-691` (duplicate controls + header/menu-bar horizontal overflow at `1280px`).
2. **Phase 2 (Panel scroll containment — `UI-02`):** `apps/saturn-tauri/ui/src/styles/03-editor.css:365-396`, `apps/saturn-tauri/ui/src/features/inspector/InspectorPanel.tsx:491-516`, `apps/saturn-tauri/ui/src/styles/02-editor.css:238-247`, `apps/saturn-tauri/ui/src/styles/140-shell.css:74-85` + `03-editor.css:113`.
3. **Phase 3 (Color wheels & Fairlight unclipping — `UI-03`):** `apps/saturn-tauri/ui/src/styles/06-editor.css:310-367`, `apps/saturn-tauri/ui/src/workspaces/docks/ColorDocks.tsx:195-254` (inline hex violation + 4-col clipping), `apps/saturn-tauri/ui/src/styles/04-editor.css:291-322` (Fairlight `min-height: 480px` & `108px` strip width).
