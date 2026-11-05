# Review: EEP AI-Native Architecture Plan, what it achieves, and how to earn from it

Reviewed: `Eep_Ai_Native_Architecture_Plan.md` (952 lines), against repo main `9774742`.
Read-only review. Nothing in your repo was changed. I did not build or run EEP.

---

## 1. Verdict in five lines

1. **The direction is right and the diagnosis is accurate.** "Closed-loop perceptual co-editing" (observe → edit → verify) plus timeline branches is the idea that can set EEP apart. I checked the plan's repo claims; they are true (table below).
2. **The plan is about 3× too big.** It describes Resolve + Fairlight + Fusion + a neural perception stack. Most of the colour, VFX and plugin-host work is for human editors, not for agents.
3. **The "agent MVP" is Phases 0, 1 and part of 2.** Those alone make EEP something no open-source editor is today.
4. **A few design choices will hurt:** the `/dev/shm` frame ring, "Murch score", hard-coded skin-tone ranges by ethnicity, OCIO as a dependency, and tying every agent tool to the desktop app.
5. **Money will come from hosting and trust, not from the editor itself.** Details in §7.

---

## 2. Claims I verified in the repo

| Plan claim | Result |
|---|---|
| `transcribe_audio_speech` returns hard-coded demo sentences | **True**: `apps/saturn-tauri/src/commands/audio.rs:107` ("Welcome to Easy Edits Pro…") |
| `analyze_motion_tracking` is synthetic sine/cosine | **True**: `commands/preview.rs:17,19` use `.sin()` |
| `media_preview` has no still frame | **True**: `mcp_bridge.rs:402` and `saturn-mcp/src/server.rs:756` say "not implemented" |
| `wgpu_surface.rs` is CPU `Vec<u8>` | **True**: `rgba_data: Vec<u8>` at line 43 |
| `saturn-render/src/lib.rs` is 1,483 lines; `server.rs` is 818 | **True** (1,482 and 817 today). Both exceed your own 800-line rule |
| No ONNX/Whisper dependency exists | **True**: no `ort`, `whisper` or `onnx` in any `Cargo.toml` |
| `EbuR128Meter` exists but is not exposed over MCP | **Consistent**: only `loudness.rs` and `lib.rs` reference it, nothing in `saturn-mcp` |
| `docs/security-audit-report.md`, `mcp-command-matrix-report.md`, `agent-task-card.md`, `native-workflow.md` exist | **True** |
| P0.1 asks to set `overflow-checks = true` | **Probably already done**: `Cargo.toml:19` sets it. I did not check which profile. Verify before assigning |

I did not verify the 112 `ui_only` count, the audit findings, or the line numbers inside `captions_cmds.rs` etc.

---

## 3. What is strong (keep exactly as written)

- **Rust owns all state; clients only send commands.** This is what makes agents safe and testable.
- **Branches for AI work** (`project.branch_create/diff/merge`) plus a human accept/reject ribbon. This is "git for timelines" and fits how people will actually trust agents.
- **Atomic batches with rollback**, and before/after telemetry on every batch.
- **Perception tools as the first deliverable** (frames, scopes, loudness), before any fancy editing.
- **§10 security and §11 governance.** Write grants, `0600` sockets, `catch_unwind`, checked tick arithmetic, honest stubs, the `shipped: true` rule.
- **Tier 5 unmocked native tests.** This is the single most valuable habit in the plan.

---

## 4. Problems to fix before agents start building

### 4.1 Scope
About 35 new commands, 7 new MCP tools, 2 new crates, OCIO/ACES, GPU colour nodes, a CLAP/LV2/VST3 host, stem separation, depth compositing, inpainting, morph cuts, film emulation. That is several years of work for a mature team.
**Fix:** adopt the cut line in §5. Move everything else to "later / community / plugin".

### 4.2 The `/dev/shm` zero-copy ring is the wrong tool
A language model cannot read shared memory. It receives images as base64 content or a file path. The ring only helps a local process, and it adds a new attack surface the plan itself then has to harden (§10.2).
**Fix:** return MCP image content blocks (JPEG/PNG, size-capped) or a path under `$XDG_RUNTIME_DIR/easy-edits-pro/previews/` (`0700`). Delete `shm_ring.rs` from the plan. Keep the 1 MiB socket limit but chunk or use paths for big payloads.

### 4.3 "Contact sheet in < 150 ms" is not realistic
Seeking and decoding 16 frames from long-GOP H.264, especially 4K, usually takes far longer. Pre-warming helps but not that much.
**Fix:** target 1–2 s, generate and cache thumbnails/proxies at import, and make the sheet read from the cache. Also add a **token budget** (tile count, tile size, JPEG quality) because every image costs the model's context and your user's money.

### 4.4 `CutContinuityScore` / "Murch score" has no ground truth
`overall_murch_score` 0–100 ("invisible Hollywood cut") is an invented scalar. Eye-trace distance needs a saliency model, 180° rule needs face orientation, and the weights are arbitrary. An agent will over-trust a number that looks precise.
**Fix:** expose the **measurable parts only** (luma jump, ΔE2000, audio boundary pop, optical-flow direction) and call any combined score "heuristic". Before using it as a gate, check it against a small set of human-labelled good/bad cuts. This also fits your own rule 10 ("docs may only claim verified behaviour").

### 4.5 Skin-tone logic is hard-coded and ethnicity-labelled
`skin_tone_ire_mean` comments give different target IRE ranges by ethnicity. That is inaccurate (skin tone varies within groups and with lighting) and a reputational risk.
**Fix:** measure skin tones **relative to a reference shot or a user-set target**, never by ethnicity label. Keep the vectorscope skin-tone line as a guide only. Test face detection (YuNet) and `ai_face` masks across a diverse set before shipping.

### 4.6 OCIO as a dependency
OpenColorIO is a C++ library with weak Rust bindings, and ACES configs are large.
**Fix:** implement a **fixed, small set** of transforms yourself (ACEScg/ACEScct matrices, a handful of camera log curves, Rec.709/sRGB/P3 outputs) in WGSL. Add more only when a user asks.

### 4.7 The export path is the hidden hard part
Preview via `wgpu` in 32-bit float is fine. But export today goes through GStreamer/GES. For "preview and render match bit-for-bit within −90 dBFS" and for graded video, frames must be rendered by the same GPU pipeline and pushed into the encoder (`appsrc`). That is effectively a **new renderer**, not a patch.
**Fix:** treat "one render path for preview and export" as its own milestone with its own tests, before adding more colour nodes.

### 4.8 The agent tools depend on the desktop app being open
The new tools route through `mcp_bridge.rs`, which talks to the running Tauri app. A product "for agents" must run **headless** (servers, CI, containers, cloud).
**Fix:** make the engine, perception and render a library/daemon (`saturn-control`) that the GUI is just a client of. The MCP server talks to the daemon. This one change also unlocks hosting (§7).

### 4.9 Shared viewport state: human and agent will fight
`UnifiedViewportState` puts one playhead and one selection in the engine. If a human and an agent both move them, they overwrite each other.
**Fix:** give each client session its **own cursor/selection**; the human's lives in the GUI session. Keep viewport state **out of undo and out of "dirty" state**. Only mutate the document through commands.

### 4.10 Branch merging
"Merge a tick range from source into target" needs a conflict policy (same clip edited on both branches, media added on one side).
**Fix:** v1 = accept the **whole branch** or reject it. Add range merges later with a written conflict rule and tests.

### 4.11 Smaller technical notes
- **BLAKE3 "head + tail + size" is a fingerprint, not a content hash.** It misses edits in the middle of a file. Name it `fingerprint`, use it to *suggest* relinks, and ask the user to confirm.
- **Whisper:** `whisper-rs` (whisper.cpp) is simpler than ONNX Whisper. **Speaker diarization is not part of Whisper**; it needs a separate model. Treat `diarize` as its own packet.
- **Model weights:** plan the download-on-first-run flow, disk use (large models are over a gigabyte), and offline behaviour.
- **480+ commands for a model to choose from.** Long tool lists make models worse at choosing. Ship a **curated default profile (~20–30 tools)** and let power users opt in to the rest. (A competitor I looked up does exactly this: 20 by default, 205 on request.)
- **Large results:** `doc_inspect` returns the whole document. Add summaries and pagination so long timelines do not flood the model's context.
- **Prompt injection from media.** A spoken line in a video ("ignore previous instructions…") becomes transcript text in the agent's context. §10 covers file writes but not this. Mark all media-derived text as **untrusted data** in tool results, and add **permission tiers** (read-only, edit, export, outside-project writes) with human approval for the risky ones, plus an audit log.
- **Licences of model weights** (from memory, **verify before shipping**): RobustVideoMatting's repo is GPL-3.0; Depth-Anything-V2 Small is Apache-2.0 but the larger sizes are non-commercial (CC-BY-NC); stem-separation weights vary by source. This matters for §7. My search for confirmation returned only titles, so treat these as unconfirmed.

---

## 5. Suggested cut line and order

Time ranges are my rough guess for AI coding agents with a human running real-media tests. Treat them as ±2×.

| Stage | Contents | What it delivers | Rough time |
|---|---|---|---|
| **A. Foundation** | Phase 0: tick-overflow fixes, undo drift, viewport state in Rust (per-session), schema v15 fingerprints, **whole-branch** create/switch/diff/accept | Agents can't brick the editor; nothing is "UI-only"; safe experiments | 3–6 weeks |
| **B. Eyes and ears** | Phase 1 without shm: real `media_preview`, frame/contact sheet with budget, scopes, loudness + spectrum, clip proxy | A multimodal model can see and measure its own edits | 1–2 months |
| **C. Headless daemon** | Move engine/perception/render behind `saturn-control`; MCP talks to it; container image | Runs on a server; the base for hosting | 1–2 months (can overlap B) |
| **D. Understanding** | Phase 2 core: real Whisper (word times), shot detection, embeddings + search, real tracking | "Edit by description", transcript cuts, find-the-shot | 2–4 months |
| **E. Proof** | An **evaluation suite** (see §6) and the scripted closed-loop test | Evidence EEP works; marketing material | ongoing from B |
| **F. Co-edit UI** | Branch ribbon, scopes dock, script panel, telemetry drawer, **built on the new glass-card shell** (after G3 of the UI plan) | Humans can supervise agents | 1–2 months |
| **G. Later / optional** | OCIO and GPU colour graph, film emulation, CLAP/LV2 host, stems, room tone, depth, inpainting, kinetic type | Resolve-class depth | 6–12+ months each; do on demand |

Keep §10 and §11 of your plan in full; they are good.

---

## 6. What EEP will achieve after this plan

### After A + B (the minimum worth shipping)
- Any MCP-capable model (Claude, GPT, Gemini or a local one) can **inspect the whole project, see frames, read scope and loudness numbers, run a batch that rolls back on failure, work on a branch, and hand a diff to a human**.
- Practical jobs: hit a loudness target, fix clipped highlights, tidy silences, assemble clips, add captions, export variants.
- To my knowledge this is not available in an open-source editor today. I only looked at a handful of products (see §7), so treat that as a hypothesis to check.

### After D (the part that makes it useful)
- **"Edit by describing"**: cut a 60-minute interview into a 90-second story, pull every shot of a person or place, make vertical clips from a long video, remove fillers, caption and export.
- Best fit: **talking-head, interview, podcast, lecture, tutorial, meeting and social-clip workflows.** These are structured, repetitive and measurable.

### After G (only if users pull for it)
- Colour-managed grading, plugin-based audio, VFX. Valuable, but a niche for agent users and the largest engineering risk.

### What it will not be
- **Not an autonomous Hollywood editor.** Models are good at assembly and checking, weak at taste. The plan's "Hollywood-grade master" wording should go. The honest promise is: **"the agent does the tedious 70–80% and shows you a diff; you approve."** Measure that claim before you print it.
- **Not an overnight win.** The first real value arrives after stages A–D.

### How to prove it: an agent benchmark
Build a small public suite ("EEP-Bench"): 20–50 tasks such as *cut this interview to 60 s keeping the main point*, *match these two shots*, *master to −14 LUFS*, *remove all "um"s*. Score automatically (duration, loudness, ΔE, caption accuracy, no black frames, no audio pops). Run several models weekly and publish the table. This tells you what to build next, drives your tool design, and is your best marketing.

---

## 7. Landscape and how to make money

### 7.1 What exists (from my web search; not exhaustive)
- **Hosted, closed products with agents/MCP:** Tella, Descript (Underlord), Riverside, invideo, OpusClip, Reap. Several charge by credits or media minutes; Tella's MCP server is on every plan, Riverside's needs a Grow plan or higher ([source](https://www.tella.com/blog/best-agentic-video-editors)).
- **Local open-source MCP servers that wrap FFmpeg:** for example [Kinocut](https://github.com/KyaniteLabs/kinocut) (Apache-2.0, free, 20 default tools, 205 total, guardrails, "video receipts"). Many more are listed on [Glama](https://glama.ai/mcp/servers?query=video+edit).
- **Programmatic video infrastructure:** Remotion, Shotstack, Creatomate, JSON2Video.

**What this means for you:** a free, local, FFmpeg-wrapper MCP server is **already a crowded, commodity space.** EEP's advantages are different: a **real timeline with undo and branches**, **perception and verification**, and a **human GUI to co-edit and approve**. Sell those, not "an MCP for video".

### 7.2 Real pricing data point
Remotion (source-available, not OSI open source) is free for individuals and teams of up to 3, and charges companies $25/month per developer or $0.01 per render, with a $100/month minimum; enterprise starts at a $500/month minimum ([Remotion licence FAQ](https://www.remotion.dev/docs/license/faq)). It shows that developers will pay per seat or per render for video infrastructure.

### 7.3 Ways to earn, ranked for EEP

| # | Model | How it works | Fit |
|---|---|---|---|
| 1 | **Hosted cloud (EEP Cloud)** | You run the headless daemon with GPUs: transcription, indexing, rendering, agent sessions. Charge per processed minute / per render / monthly plan. Free to self-host, pay for convenience | **Best.** Needs stage C. Agents run on Linux servers, so a Linux-first headless engine is an advantage |
| 2 | **Enterprise self-hosted** | Paid licence/support for companies: SSO, audit logs, permission tiers, on-prem install, SLA, priority fixes | **Good.** The safety tiers and audit log in §4.11 are exactly what companies pay for |
| 3 | **Dual licence** | GPL-3 for everyone; a **commercial licence** for companies who want to embed EEP in a closed product | **Good**, but only if you hold the copyright (see §7.4) |
| 4 | **Paid assets and skills** | Look packs, LUTs, title templates, "agent skill" workflow packs (podcast-to-shorts, product-demo cutter), fonts/music partnerships. Content is not GPL code | **Good, small at first**; add a marketplace later |
| 5 | **Support, training, integration** | Pipelines for agencies and studios | **Good early cash** while the product grows |
| 6 | **Sponsorship and grants** | GitHub Sponsors, Open Collective, and public/foundation funds for open-source infrastructure (I believe NLnet and the Sovereign Tech Agency are examples; verify eligibility) | **Useful, not a business** |
| 7 | **Resell model tokens** | Charge for LLM usage inside the app | **Avoid at first.** Let users **bring their own model key**; margins are thin and billing is complex |

### 7.4 Decisions to make early (before outside contributors arrive)
1. **Licence.** GPL-3 stops anyone from closing the code, but a competitor can host your **unmodified or modified server** and sell it without sharing. **AGPL-3** for the server/daemon components closes that gap. Switching later needs every contributor's consent.
2. **Who owns the copyright.** If you want dual licensing or a later licence change, collect a **CLA or DCO** from every contributor now. Keep a record of what AI agents wrote and which tools' terms apply.
3. **Clean provenance.** Parts of the project were ported from FilmCraft. Before you charge anyone, confirm FilmCraft's licence and add a `NOTICE`. Likewise confirm every model-weight licence (§4.11): a non-commercial weight inside a paid feature is a legal problem.
4. **Trademark.** Register or at least document the "Easy Edits Pro" name. The code can be forked; the name and hosted brand are what you actually own.

### 7.5 A realistic path
1. **Months 0–4:** stages A + B. Ship `.deb`/Flatpak, a demo video of an agent editing with a branch diff, and the first EEP-Bench results. Free. Goal: stars, users, issues.
2. **Months 4–9:** stages C + D. Launch **EEP Cloud beta** (waitlist, pay-per-minute) and a **support/integration offer** for early teams.
3. **Months 9–18:** paid skills/asset packs; enterprise self-hosted tier; publish a quarterly benchmark.
4. Revisit the licence, the Windows/macOS bridge and the colour/audio depth only when paying users ask.

### 7.6 Honest expectations
Most open-source projects earn little. Revenue usually comes from a **hosted service or support for companies**, not from individuals who download the app. Do not count on money before you have real users and a working hosted offering. Competitors are funded and moving fast. I have no data on your potential market size; the figures above are models, not forecasts.

---

## 8. Immediate next steps
1. Decide the licence question (§7.4) **before** the next outside contribution.
2. Replace Phase 1's `/dev/shm` ring with image content blocks and cached thumbnails; split `lib.rs` and `server.rs` first.
3. Verify `overflow-checks`, then dispatch task cards P0.1 and P1.1 as written (they are good).
4. Start the EEP-Bench task list now; you need it to know whether any phase worked.
5. Put the co-edit UI (Phase 5.2) **after** the glass-card shell lands, using your existing colours and fixed layout.

---

Sources: [Tella: best agentic video editors](https://www.tella.com/blog/best-agentic-video-editors) · [Kinocut](https://github.com/KyaniteLabs/kinocut) · [Remotion licence FAQ](https://www.remotion.dev/docs/license/faq) · [Glama video MCP list](https://glama.ai/mcp/servers?query=video+edit)
