# Easy Edits Pro — Roadmap

This is the single roadmap for the project. Other documents describe the
*how* (architecture, plans) and link here; this file owns the *what and
when*. Status is **not** hand-written here: current coverage lives in
[`docs/PARITY.md`](PARITY.md), generated from the command catalogue by
`pnpm check:parity`.

---

## 1. Principles

1. **Green `main`, always.** Every change lands as a small, verified packet.
2. **Verified before "done".** A feature is shipped only when it has a UI
   affordance, a test, a catalogue entry marked `shipped`, and (for media
   features) a pass on the real-media corpus. Docs never claim more.
3. **Measure before polishing.** Performance and compatibility claims need
   numbers from more than one machine.
4. **One source of truth for status.** Generated from the catalogue and CI
   (`docs/PARITY.md`), not hand-written percentages.
5. **Small, disjoint work packets** so agents can run in parallel.

The full work-packet table (WS-A stabilise, WS-B distribution, WS-C glass,
WS-D parity tooling, WS-E parity features, WS-F real-media verification,
WS-M MCP/AI safety, WS-G process docs) with per-packet goals, owned paths,
and acceptance criteria is maintained in the current implementation plan
(final-implementation-plan.md, mirrored by each packet's task card).

---

## 2. Milestone ladder

| Milestone | Window (est.) | Contents | Exit criteria |
|---|---|---|---|
| **M0 Stabilise** | Week 1 | Markers done, warning-free build, e2e split, parity script + `PARITY.md`, one roadmap, agent task-card rules restored, distribution decision, hardware-matrix started | Green `main`, one roadmap, distribution decision recorded |
| **M1 Trust** | Weeks 2-3 | Hardware matrix + DMA-BUF safety net, Flatpak/release workflow + install matrix, real-media corpus, native workflow test, encoder matrix, recovery + MCP/control security tests | **Alpha 2**: installable artifact, real-media import-edit-export passing, DMA-BUF safe by default or fallback |
| **M2 Finish glass + safety** | Weeks 3-5 | Contrast on real glass, CSS naming, panel scaling/breakpoints, remaining workspaces on the panel engine, layout tests, visual regression, perf budget script, auto-downgrade, accessibility pass, glass docs; MCP permission tiers, source tagging, audit log; parity backlog generator, expose already-working engine features | Glass done and measured; AI actions safe; shipped count rising |
| **M3 Tier-1 parity** | Weeks 5-8 | Tier-1 families (below) with real-engine coverage and project-compatibility fixtures | **Beta 1** |
| **M4 Tier-2 parity** | Weeks 8-11 | Tier-2 families | Beta 2 candidate |
| **M5 Later** | After | Tier-3 families, learning layer, Windows MCP bridge/packaging | - |

---

## 3. Parity tiers

Measured per family in `docs/PARITY.md`. Full parity with FilmCraft's 701
commands is explicitly **not** a Beta goal; Tier 1 plus verified quality is.

- **Tier 1 (Beta 1):** file/project/sequence management, clip/timeline/edit/
  trim/playhead, markers, captions, export queue + presets, media bins,
  shortcuts/preferences, history.
- **Tier 2 (after Beta 1):** graphics beyond basics, effects/colour UI
  (masks, Lumetri-style, LUT, presets), mixer/essential sound, multicam,
  transcript editing.
- **Tier 3 (later):** TTS, media browser, collaboration, scripting, advanced
  interchange.

---

## 4. Phase ledger

### FilmCraft parity phases (per the approved phased plan, 2026-10-21)

| Phase | Scope | Status |
|---|---|---|
| 0 - Command catalogue & foundation | `saturn-commands` catalogue, codegen, gesture-scoped undo, enablement reasons | Done |
| 1 - Surface | Menus, 17 tools, shortcuts, workspaces, command palette | Done |
| 2 - Editing engine | Playhead, edit, timeline, trim gestures, Trim Monitor | Done |
| 3 - Clip & sequence depth | Multi-sequence tabs, speed/duration, razor/context tools, transitions | Done |
| 4 - Media organisation | Bins, labels/metadata, markers (point/range/nav/CSV), pointer workflows | Done |
| 5 - Graphics, text, captions | Caption tracks + editor, timeline graphics entities, text stack, export rasterisation | Done |
| 6 - Audio and mixing | **Slice 1 done**: automation curves (interpolation modes, per-sample render, Fairlight Latch/Touch/Write recording, lane editor). Remaining: mixer channel depth, clip gain/fades, essential-sound presets, ducking, insert-slot DSP | In progress |
| 7 - Source, multicam, proxies, speech | Angle sync/switch/flatten, transcript editing, TTS, proxy manager | Planned (M3-M4) |
| 8 - Export, interchange, project manager | Export queue + presets over the job system, EDL/FCPXML/XML/OTIO, project manager | Planned (M3) |
| 9 - Platform | Preferences depth, colour depth (lumetri/lut/masks), scripting/plugins | Planned (M4+) |

### Original 16-phase plan

Phases 0-6 (foundations through export MVP) are complete; phases 7-14 were
advanced substantially through the increment ledger in
[`planning.md`](planning.md). That 16-phase numbering is retired in favour of
the milestone ladder above; `planning.md` remains the architecture decision
record. See its Part 1 for the preview-architecture rationale.

### Glass UI

Phases 0-3 (WebKitGTK spike, tokens + contrast gates, glass surface system,
animation foundation) have landed. Phases 4-6 (panel polish, remaining
workspaces, measurement + a11y) are tracked in
[`GLASS_UI_PHASED_PLAN.md`](GLASS_UI_PHASED_PLAN.md) and fold into milestones
M1-M2 above.

---

## 5. What is next

### Decisions recorded

| Packet | Decision | Where |
|---|---|---|
| B1 distribution | **Flatpak** is the supported path; `.deb`/`.rpm` become secondary best-effort artifacts; AppImage stays dropped | [`docs/distribution.md`](distribution.md) |
| C7 workspaces on the panel engine | **Fusion and Fairlight migrate to floating panels** (multi-pane colour/audio work benefits); **Export stays docked** (linear form-like surface) | decision table here; execution in M2 |
| C1 hardware matrix | Matrix formalised in the glass-spike addendum; this machine's row (Intel Iris Xe, Wayland/niri) is filled from the phase-0 spike; NVIDIA/AMD/X11/older-WebKitGTK rows need real machines | [`docs/glass-spike.md`](glass-spike.md) |

1. Finish M0: agent task-card template and hard agent rules, distribution
   decision (Flatpak vs deb/rpm), start the glass hardware matrix.
2. M1: real-media corpus + native workflow test; Flatpak; DMA-BUF watchdog.
3. M2: glass measurement/a11y; MCP safety tiers.

Current coverage snapshots live in [`docs/PARITY.md`](PARITY.md); command
reference in [`docs/COMMANDS.md`](COMMANDS.md); evidence base and parity
method in [`docs/filmcraft-parity-roadmap.md`](filmcraft-parity-roadmap.md).
