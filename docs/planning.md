# Easy Edits Pro — Master Delivery & UI Overhaul Plan

**Updated:** 2026-10-23  
**Status source:** Command catalogue generates [PARITY.md](PARITY.md); GitHub Actions is the source for CI and packaging status.

This document serves as the authoritative, forward-looking plan for Easy Edits Pro (EEP). It incorporates the **Executive Architectural Diagnosis**, the **Target Hybrid Architecture**, the **UI Correction Plan v4 (Fixed Glass Cards & Persistent 2-Row Viewport)**, and the **Functional Reliability Workstreams (F1-F6)** while resolving all prior roadmap and styling conflicts.

---

## 1. Scope, Design Decisions & Conflict Resolutions

### Core Architectural Decisions

| Decision | Rationale | Resolution / Conflict Elimination |
|---|---|---|
| **Fixed Glass Cards; Zero Window Dragging** | "Floating" is purely visual glass elevation (borders, inner highlight, soft shadows over a frosted, tinted stage). Prior automated development mistakenly implemented a free-floating Multiple Document Interface (MDI) window manager. | **Delete the MDI floating engine** (`FloatingPanel.tsx`, `PanelHost.tsx`, `docking.ts`, `snapping.ts`, `zOrder.ts`, restore pill bar). Panels never detach, drag, pin, minimize, or close. |
| **Existing Colors Are Final & Locked** | The existing plum/magenta glass palette, ambient stage gradients, text, and accent tokens are fully locked and calibrated to WCAG AA contrast gates. | **Reject the PDF's proposed slate color shift** (`rgba(22, 22, 28, 0.78)` / `#110d18`). All surfaces use existing tokens from `00-tokens-glass.css`. Only shape, gap, and elevation tokens are added (`--gap: 12px`, `--radius: 16px`, `--radius-sm: 10px`, `--card-inset`). |
| **Persistent 2-Row NLE Skeleton Across Workspaces** | Switching workspace tabs previously unmounted the editor and mounted isolated full-screen pages with synthetic "SEQUENCE CLIPS REEL" boxes, breaking timeline context. | **Implement a single unified `WorkspaceShell`**. Top-Center Program Monitor, Bottom-Left Toolbox, Bottom-Center Multi-Track Timeline, and Bottom-Right Audio Meter stay permanently locked across all workspaces. Workspaces only swap contextual dock tabs (Dock L / Dock R) and column ratio presets. |
| **Compact 44px Track Headers** | Track headers were bloated to 150px height by horizontal VOL and PAN range sliders, restricting the timeline to a single visible track. | **Standardize on the reference editor's compact 44px track headers** (Source Patch pill, Lock, Target pill, Name, M/S, Visibility eye). Move per-track gain/pan sliders into the Inspector and Audio Mixer. |
| **Validate on Development Laptop First** | Local testing on the development laptop is the only verified baseline. Avoid cross-hardware claims without local reproduction. | Host path uses software H.264/x264 fallback as ground truth; hardware acceleration detection (nvenc, vaapi, videotoolbox, qsv) is cached asynchronously. |
| **Authoritative Rust Engine Core** | Project state, history, undo/redo, timeline mutations, and export pipelines remain strictly in Rust (`crates/saturn-*`). | React UI and MCP services submit structured commands and render snapshots; client state is never forked. |

---

## 2. Locked Design System & Token Architecture

All UI components must consume the existing design tokens in `00-tokens-glass.css` and `01-editor.css`. **No color value may be modified or added.**

### 2.1 Final Surface & Color Tokens

```css
:root {
  /* STAGE BACKGROUND: Locked radial glow + linear backdrop */
  --bg-stage:
    radial-gradient(48vw 42vh at 18% 22%, rgba(255, 92, 148, 0.2), transparent 62%),
    radial-gradient(42vw 46vh at 82% 74%, rgba(88, 132, 255, 0.18), transparent 64%),
    radial-gradient(36vw 32vh at 62% 18%, rgba(255, 184, 92, 0.1), transparent 60%),
    linear-gradient(160deg, #170c16 0%, #0b060a 54%, #120a14 100%);

  /* GLASS SURFACES: Locked existing tokens */
  --glass-l1: rgba(24, 13, 23, 0.56);
  --glass-border: rgba(244, 215, 235, 0.14);
  --glass-highlight: rgba(255, 255, 255, 0.08);
  --glass-shadow: 0 18px 48px rgba(0, 0, 0, 0.45);
  --blur-l2: 14px;
  --glass-saturate: 145%;

  /* TYPOGRAPHY: Locked existing text tokens */
  --text-primary: #faf2f7;
  --text-secondary: #d4bccf;
  --text-muted: #a890a0;

  /* ACCENT TOKENS */
  --accent-text: #f472b6;
  --accent-primary: #9f1249;
  --accent-text-violet: #c084fc;
  --ring: rgba(244, 114, 182, 0.55);

  /* TRACK & STEM TOKENS */
  --video: #7ad0ff;
  --audio: #6be0a8;
  --gfx: #ffb35c;

  /* SHAPE & ELEVATION: New structural tokens only (No new colors) */
  --gap: 12px;
  --radius: 16px;
  --radius-sm: 10px;
  --card-inset: rgba(255, 255, 255, 0.045);
}

/* Glass Card Primitive (Desktop-Kit Pattern) */
.glass {
  background: var(--glass-l1);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius);
  box-shadow: inset 0 1px 0 var(--glass-highlight), var(--glass-shadow);
  backdrop-filter: blur(var(--blur-l2)) saturate(var(--glass-saturate));
  -webkit-backdrop-filter: blur(var(--blur-l2)) saturate(var(--glass-saturate));
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
```

> [!IMPORTANT]
> **GPU Compositor Safety Rule:** `backdrop-filter` is applied strictly to non-overlapping `.glass` cards over the static root stage. It must **never** be placed over the active `<video>` canvas or wgpu preview surface, ensuring zero blur-kernel re-evaluations during 60 FPS playback.

---

## 3. Persistent 2-Row NLE Viewport Architecture

```
┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                AppHeader & Global Menus (46px)                                         │
├───────────────────────────────┬────────────────────────────────────────┬───────────────────────────────┤
│   Top-Left Contextual Dock    │   Top-Center Persistent Program        │   Right Contextual Dock       │
│   minmax(260px, 24% - 28%)    │   Monitor (1fr)                        │   minmax(260px, 24%)          │
│   (Swappable Tabs)            │   (GPU Canvas + Scrub + Transport)     │   (Swappable Tabs)            │
├─────┬─────────────────────────┴────────────────────────────────────────┴───────────────────────┬───────┤
│Tool │  Bottom-Center Multi-Track Timeline (1fr)                                                │Master │
│Strip│  (Compact 44px Track Headers, Sequence Tabs, Grouped Toolbar, Playhead, Lanes)          │Meter  │
│52px │                                                                                          │92px   │
├─────┴──────────────────────────────────────────────────────────────────────────────────────────┴───────┤
│                                StatusBar & Telemetry (30px)                                            │
└────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Contextual Dock Tab Mapping per Workspace

The 4-zone lower row (Toolbox, Timeline, Audio Meter, Status Bar) and the Top-Center Program Monitor **never move or unmount**. Clicking workspace tabs changes only the docked tabs in Dock L and Dock R, along with column width presets:

| Workspace Mode | Top-Left Contextual Dock | Top-Center Monitor | Right Contextual Dock | Bottom Row (Tools + Timeline + Meter) |
|---|---|---|---|---|
| **Edit** | Project · Media Browser · Effects | Persistent Program Monitor | Inspector · Titles · Markers | Persistent 4-Zone Bottom Row |
| **Assembly** | Project · Markers | Persistent Program Monitor | Source Monitor · Inspector | Persistent 4-Zone Bottom Row |
| **Color** | Scopes · Project Media | Persistent Program Monitor | Color Wheels · Curves · HSL · Primary Sliders | Persistent 4-Zone Bottom Row |
| **Audio (Fairlight)** | Track DSP Mixer (4-Band EQ, Comp) · Project | Persistent Program Monitor (Picture Sync) | Essential Sound · Channel Strip Rack | Persistent Bottom Row + Expanded Automation Lanes |
| **Captions & Graphics** | Text (Transcript, Captions, Graphics) · Effect Controls | Persistent Program Monitor + Live `CanvasGraphicsLayer` | Essential Graphics Template Browser · Title/Subtitle Inspector | Persistent 4-Zone Bottom Row |
| **Effects** | Effect Controls (Active Stack, AI Matting) · Source | Persistent Program Monitor | Effects Library (Presets, Audio FX, Video FX) · Essential Graphics | Persistent 4-Zone Bottom Row |
| **Curves (Keyframes)**| Keyframe Bezier Curve Editor · Source | Persistent Program Monitor | Interpolation Style & Tangent Handles (Linear, Ease, Bezier) | Persistent 4-Zone Bottom Row |
| **Fusion** | Node Graph Palette & Node Inspector | Fusion Node Graph Canvas (Tabbed with Program Monitor) | Node Parameters & Routing Sockets | Persistent 4-Zone Bottom Row |
| **Deliver (Export)** | 1-Click Delivery Presets · Export Settings | Program Monitor (In/Out Render Range Scrub) | Batch Render Queue · Timeline Interchange (FCPXML, EDL, OTIO) | Persistent 4-Zone Bottom Row |

---

## 4. Teardown of MDI Window Manager

The following legacy window-management files, state stores, and styles are to be completely removed:

* `apps/saturn-tauri/ui/src/features/panels/FloatingPanel.tsx`
* `apps/saturn-tauri/ui/src/features/panels/PanelHost.tsx`
* `apps/saturn-tauri/ui/src/features/panels/docking.ts` (and `docking.test.ts`)
* `apps/saturn-tauri/ui/src/features/panels/snapping.ts` (and `snapping.test.ts`)
* `apps/saturn-tauri/ui/src/features/panels/layouts.ts`
* `apps/saturn-tauri/ui/src/features/panels/presets.ts`
* `apps/saturn-tauri/ui/src/features/panels/presetStore.ts` (and `presetStore.test.ts`)
* `apps/saturn-tauri/ui/src/features/panels/usePanelLayout.ts`
* `apps/saturn-tauri/ui/src/features/panels/usePanelDrag.ts`
* `apps/saturn-tauri/ui/src/features/panels/usePanelResize.ts`
* `apps/saturn-tauri/ui/src/features/panels/zOrder.ts` (and `zOrder.test.ts`)
* `apps/saturn-tauri/ui/src/features/panels/LayoutPresetsDialog.tsx`
* `apps/saturn-tauri/ui/src/styles/140-panels.css` and `171-layout-presets.css`

### Replacement: Stationary `<DockCard>` Primitive

```tsx
export interface TabDefinition {
  id: string;
  title: string;
  render: () => React.ReactNode;
}

export function DockCard({ id, tabs, activeTabId, onSelectTab }: DockCardProps) {
  const active = tabs.find(t => t.id === activeTabId) ?? tabs[0];
  return (
    <section className="glass" id={id}>
      <div className="card-h">
        {tabs.map(tab => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={tab.id === active.id}
            className="tab"
            onClick={() => onSelectTab(tab.id)}
          >
            {tab.title}
          </button>
        ))}
      </div>
      <div className="card-b">{active.render()}</div>
    </section>
  );
}
```

* All 📌 (Pin), - (Minimize), and X (Close) buttons are deleted from panel headers.
* The bottom-right floating restore pill bar (`PANELS | Markers | Video Scopes | Reset layout`) is removed.
* Stale localStorage keys (`easy-edits-pro.panel-layout.*`) are pruned on startup.

---

## 5. Timeline, Track Headers & Toolbox Specification

### 5.1 Compact 44px Track Headers

* Track header height is locked to **44px** (eliminating the 150px bloat).
* **Video Row Anatomy (44px):** `[Source Patch V1] [Lock 🔒] [Target V1] [Track Name] [Sync Lock] [Visibility 👁]`
* **Audio Row Anatomy (44px):** `[Source Patch A1] [Lock 🔒] [Target A1] [Track Name] [Sync Lock] [Mute M] [Solo S] [Voiceover 🎙] [Keyframe ◆]`
* Master Mix footer terminates the audio block with `Mix 0.0`.
* Horizontal VOL and PAN sliders are completely removed from the track header and relocated to the Inspector and Audio Clip Mixer.

### 5.2 Grouped Timeline Toolbar (40px)

Divided by hairlines into 4 clear functional groups:
1. **Edit Modes:** Snap (🧲) · Linked Selection · Ripple Edit
2. **Marking:** Add Marker (M) · Mark In (I) · Mark Out (O)
3. **Tracks:** Add Track (+)
4. **Zoom Navigation:** Zoom Out · Zoom Range Slider · Zoom In

### 5.3 Vertical Tool Strip (52px Card)

Single-column vertical card positioned between the Media Browser and Timeline:
* **Group 1:** Selection (`V`) · Track Select Forward (`A`)
* **Group 2:** Ripple Edit (`B`) · Razor / Blade (`C`) · Slip (`Y`)
* **Group 3:** Hand / Pan (`H`) · Text (`T`)

### 5.4 Master Audio Meter (92px Card)

Flanks the timeline on the far right:
* Dual-channel vertical peak level bars (0 to -57 dB) with green/amber/red grading.
* Live EBU R128 loudness telemetry: Momentary (M), Short-Term (S), Integrated (I), True Peak (TP).

---


## 6. Completed Workstreams — Condensed Ledger

Full narratives, batch plans, and acceptance details for everything below live in
git history and the per-release CHANGELOG entries; this section records only that
each stream closed, when, and in which release.

**UI Workstream G (Done 2026-10-23):** glass-card token freeze and primitives
(G1), MDI engine teardown + `<DockCard>` (G2), the unified `WorkspaceShell`
2-row grid (G3), 44px track headers/tool strip/meters (G4), contextual dock
routing for the color/audio/caption/deliver workspaces (G5), UI polish (G6),
and geometry-invariance + Playwright validation (G7).

**Functional Workstream F (Done 2026-10-23):** single reactive selection store
(F1), media ingestion/relink fixes (F2), program-monitor transport + telemetry
(F3), keyframe render baking into the export loop (F4), once-per-startup
hardware-encoder probe with cache (F5), Fairlight plugin-host cleanup (F6).

**Parity batches (Done 2026-10-24 through 2026-10-26):**

* P1 — clip + timeline Tier-1 audit and flip (Done, `2e74b35`, alpha.5)
* P2 — clip + timeline closure (Done, alpha.6)
* P3 — track/bus/media/project/prefs/shortcuts batch (Done, alpha.7)
* P4 — interop importers + UI affordance correction (Done, alpha.8)
* P5 — small singulars (Done, alpha.9)
* P6 — speech trio + dead-flow repair (Done, alpha.10)
* P7 — caption track management (Done, alpha.11)
* P8 — live multicam editing (Done, alpha.12)
* P9 — compositing Nodes panel (Done, alpha.13)
* P10 — media metadata re-probe (Done, alpha.14)
* P11 — caption styling + engine shortcut sync (Done, alpha.15)
* P12 — workspace purge; planned list at zero (Done, alpha.16)
* P13 — per-id semantic parity metric + tier audits (Done, infrastructure)
* P14 — Tier-1 batch 1: playhead/prefs/history, captions 21/22 (Done, alpha.17)
* P15 — Tier-1 batch 2: media 15/17 (Done, alpha.18)
* P16 — Tier-1 batch 3: shortcuts 16/16 (Done, alpha.19)
* P17 — Tier-1 batch 4: export 19/19 (Done, alpha.20)
* P18 — Tier-1 batches 5+6: trim 24/24, markers +18 (Done, alpha.21)
* P19 — Tier-1 closure (sequence + clip; Done, alpha.22-alpha.24)

## 6b. Tier-1 Milestone Release Rule

DONE (2026-10-26): the rule released Tier-1 batches on coverage milestones
(200 -> alpha.21, 250 -> alpha.22, 300 -> alpha.23, 350 -> alpha.24) rather
than one alpha per family; all milestones were passed and Tier 1 closed.

## 6c. Alpha.24 Batch Plan — Sequence (32) + Clip (52), milestone 350

DONE (shipped in alpha.24, 2026-10-26): C.1-C.8 as planned - clip schema +
render hookup, new ui_only surface with App.tsx kept under the 800-line cap,
sequence engine arms, audit equivalents, UI builders, mapped-count target,
honest na entries, and the verification order. Full plan text in git history.

## 7. Master Agentic Execution Rules

All coding agents working on Easy Edits Pro must strictly adhere to these six execution directives:

1. **Sequential Phase Locking:** Execute phases in strict dependency order. Every phase must pass `cargo test --workspace` and `pnpm --dir apps/saturn-tauri build` before advancing.
2. **Zero MDI Window Controls:** Never render 📌, -, or X buttons on workspace panel headers. All panels must mount inside stationary `DockCard` containers.
3. **Zero Overlapping Popups:** Never use `position: absolute` or `position: fixed` to float feature panels over the Program Monitor or Timeline. Only modal dialogs in `features/dialogs/` may center as overlays.
4. **GPU-Safe Frosted Glass:** Apply `.glass` only to fixed cards over the root ambient stage. Never apply `backdrop-filter` or blur shaders over the active video preview canvas.
5. **Persistent Core NLE Skeleton:** Never unmount the Top-Center Program Monitor, Bottom-Left Tool Strip, Bottom-Center Timeline, or Master Audio Meter when switching workspace modes.
6. **Compact 44px Track Headers:** Never place horizontal volume or pan range sliders inside timeline track headers. Enforce the reference editor's 44px micro-icon row standard.

---

## 8. Verification & Acceptance Gates

Before claiming completion of any phase, run and pass all gates:

```bash
cargo test --workspace
pnpm --dir apps/saturn-tauri build
pnpm --dir apps/saturn-tauri test
pnpm --dir apps/saturn-tauri check:contrast
pnpm --dir apps/saturn-tauri check:styles
pnpm --dir apps/saturn-tauri check:tokens
pnpm --dir apps/saturn-tauri check:commands
pnpm --dir apps/saturn-tauri check:source-size --max-lines=800
pnpm --dir apps/saturn-tauri test:e2e
```

### Specialized Acceptance Tests

* **Geometry Invariance Test:** Measure bounding boxes of Top Bar, Program Monitor, Tool Strip, Timeline, Audio Meter, and Status Bar across all workspace tabs at 1280×720, 1600×900, and 1920×1080. Assert bounding box deviations are $\le 1\text{px}$.
* **No-Overlap Test:** Assert no two `.glass` card bounding boxes intersect under any layout state.
* **Zero Color Token Drift:** Assert `git diff` on `styles/00-tokens-glass.css` shows 0 modified or added color hex/rgba values.
* **Source Size Ceiling:** All new and modified `.ts`, `.tsx`, `.rs`, and `.css` files must strictly remain $\le 800$ lines.

---


## 6d. Alpha.25 Batch Plan — File family (51) + Tier-2 multicam audit, milestone 400

DONE (shipped in alpha.25, 2026-10-27): D.1-D.7 as planned - interop export
engine arms + File > Export flows, save group, recovery/autosave reach-through,
frame export, the synthetic-media generator (pure-Rust PNG writer + bars/tone
pipeline), remaining equivalents + documented na set, and the Tier-2 multicam
audit. Full plan text in git history.

## 6e. Beta 1 Plan — Parity Book Closed, Readiness Begins (2026-10-27)

User directive: **no new alpha/beta releases until explicitly told.**
Batches land on `main` with the full gate stack green; tags wait for
authorization.

With alpha.26 the parity campaign ended: Tier 1 388/388 (55 na), Tier 2
123/123 (84 na), Tier 3 1/1 (16 na) - every tier 100% effective, audited
per id. Parity is no longer the organizing metric; Beta 1 readiness is.

### Completed beta-readiness workstreams (details in git history + CHANGELOG)

* **E.1 (Done 2026-10-27):** the scripted acceptance runbook
  `tests/native/installed_package_workflow.py` (+ `container_driver.py`)
  produces real install/launch/edit-to-render evidence; PASS on the dev
  laptop. Container mode awaits a docker/podman daemon.
* **E.2 (Done 2026-10-27):** `tests/native/sample_projects.py` generates two
  maintained sample bundles through the engine with a `--verify` rot-guard
  (render + ffprobe PASS). Samples ship with the app via E.5 tail (open).
* **E.3 (Done 2026-10-27):** documentation freshness pass - README
  quick-start, architecture module map, native workflow evidence,
  `docs/distribution.md` beta delivery matrix, BETA release-notes template.
* **E.4 (Done 2026-10-27, `307dd3a`):** QAT hardening of the five audited
  seams; rule in force - every REPRODUCED quirk gets a regression test.
* **E.5 (Done 2026-10-27, `3accbf9`+):** packaging polish - rpm/deb lint
  pass (PASS; 2 recorded warnings), Flatpak spike **PASS on GNOME 49**,
  decision **FOR Beta 1**; audio-encoder rank fix landed. Spike drive ran
  on a CI dispatch build of main; release-asset re-verification is open.

### E.6 Known-limits review — DONE (2026-10-27)

- Re-verified against code: motion tracking still runs at render (the
  preview-side `analyze_motion_tracking` reports simulated points), proxies
  are still the single 1280x720 VP9 WebM profile, and TTS is absent
  (transcription-only via `speech.transcribe_to_subtitles`; catalogue ships
  zero `tts.*` commands).
- README "Known limitations" gained the user-visible motion-tracking,
  proxy-profile, and no-TTS lines, and the stale install-evidence bullet was
  corrected (Flatpak spike passed; not yet a shipped artifact; rpm lint
  PASS vs never-installed distinction). BETA-TEMPLATE's snapshot rule picks
  the corrected list up automatically.

### Open tails

- Proxy playback (E.4 tail): `proxy_path`/`use_proxies` are persisted but
  nothing consumes them for playback yet; when proxy playback ships, resolve
  regeneration over stale/pruned cache attachments (an attached path can
  dangle after the age-based startup prune).
- Samples bundling / first-run offer (E.2 tail).
- Flatpak: run the manifest in CI against a tagged release (archive source)
  and re-verify the acceptance drive on the release asset; then one
  non-dev-machine report.
- Container-mode .deb install proof + an RPM-host install/launch result
  (see docs/distribution.md delivery matrix).

### Gates

Gate stack unchanged (packet A4); parity scripts include the tier-3 audit -
`check-parity --check` must stay green. A beta tag is released **only** on
explicit user instruction.
