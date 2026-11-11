# EEP: Linux-first with Windows and macOS, and how to position it as an AI-first editor

Repo checked: `main` @ `ecd3041` (2026-11-10). Market facts come from two competitor blog posts (linked in §8), which are vendor-written and unbenchmarked, so treat them as a map of who exists, not as proof of quality. Anything marked "(general knowledge)" I have not verified live.

---

## 1. Did the community patch land?

Yes. It is on `main` as `b7e1d51`. Checked on the repo: all 10 new files present; only `info@zvirox.in` and `zvirox@proton.me` remain as email addresses (plus the `your.email@example.com` placeholder and the `128x128@2x.png` filename); the CONTRIBUTING checklist numbers run 1–10; the SPDX header check passes (628 files); both new shell scripts parse. `main` has also moved on with AF1/AF2 (command profiles, actor isolation, durable locks, atomic batches) and AF4 (rationale, confidence, brief), so the agent-first plan is being built.

---

## 2. Can EEP be "Linux first, Windows and macOS supported"?

**Yes, and the Rust and Tauri stack suits it.** It is real work, not a rebuild. What is portable and what is not, from the code:

| Area | Today | Portability |
|---|---|---|
| Core engine, project format, commands, perception maths, colour pipeline (wgpu) | Pure Rust | Portable. wgpu runs on Vulkan, Metal and DirectX 12 |
| Desktop shell | Tauri 2 + React | Tauri supports all three OSes (general knowledge). Web engine differs: WebKitGTK (Linux), WebView2 (Windows), WKWebView (macOS). The glass look (`backdrop-filter`) and video behaviour need testing on each |
| Rendering | GStreamer + GES (`gstreamer-editing-services`) | GStreamer runs on all three, but plugin sets differ. Confirm GES is in the official Windows/macOS binaries you plan to bundle (not verified here) |
| Hardware encode | VA-API paths in `encoders.rs`, GPU detection in `gpu_detect.rs` | Linux-specific. Needs NVENC/AMF/QSV (Windows) and VideoToolbox (macOS) equivalents |
| MCP/control bridge | Unix domain sockets (`mcp_bridge.rs`, `local_control.rs`, `saturn-mcp/server.rs`) | README already says "no Windows implementation". Needs named pipes or authenticated loopback on Windows |
| Plugin sandbox | Unix socket + `/dev/shm` rings (`saturn-audio-dsp/shm_ring.rs`, `sandbox.rs`, `saturn-plugin-worker`) | Unix-specific. Needs a per-OS shared-memory and sandbox layer |
| CLAP plugin trust paths | `$HOME/.clap`, `/usr/lib/clap` | Add the Windows and macOS standard CLAP directories |
| Preview cache | `$XDG_RUNTIME_DIR/easy-edits-pro/previews` | Use a cross-platform dirs library with an owner-only mode per OS |
| DMA-BUF GPU import | Linux only | Fall back to a copy path elsewhere |
| ONNX Runtime, whisper.cpp | `ort` and `whisper-rs` | Both have Windows/macOS builds (general knowledge); your glibc floor workaround does not apply there |
| Packaging | `.deb`, `.rpm`, Flatpak | Add Windows installer (MSI/NSIS) and macOS `.dmg` |
| CI | Ubuntu runners only | Add Windows and macOS jobs |

**Costs people forget**
- **Code signing:** Apple Developer Program and notarisation, and a Windows signing certificate to avoid SmartScreen warnings. Both are paid and take setup time (general knowledge).
- **Licensing:** shipping GStreamer DLLs and dylibs means you distribute LGPL (and possibly GPL) components yourself, with notices and relinking rights. Check which plugins the official installers include; avoid GPL-only ones to keep your commercial licence clean. macOS and Windows have OS-provided codecs (VideoToolbox, Media Foundation), which can reduce what you must bundle, but patent questions remain for your lawyer.
- **Test hardware:** you need real Windows and Mac machines or reliable CI runners.
- **Support load:** each OS multiplies bug reports.

### Recommended approach: tiers, a platform layer, and headless first

1. **Declare support tiers in the README**
   - **Tier 1 Linux:** full CI, packages, release gates.
   - **Tier 2 macOS and Windows:** builds in CI, packages published, "supported with known gaps", maintained with community help.
   - A tier is promoted only when its CI and install proof are green.
2. **Add a `saturn-platform` crate** that owns everything OS-specific: IPC (socket or pipe), shared memory, sandbox, data/cache/runtime dirs, plugin directories, GPU/encoder detection, open-in-file-manager. Rule: no new `cfg(unix)` or Linux paths outside it (enforce with a CI grep).
3. **Cheap first step (about a day):** add `cargo check --workspace` jobs on `windows-latest` and `macos-latest`. They show how much breaks without any porting work.
4. **Headless first.** The engine, CLI and MCP server are what agents use. Port `saturn-cli` + `saturn-mcp` (the daemon) to macOS and Windows before the GUI. That delivers the agent story to those users early, and also lets CI and cloud servers run it.
5. **Then the desktop app,** one OS at a time.
   - **macOS first** if your first users are developers driving the editor from Claude Code, Codex or similar (many use Macs, and Metal/VideoToolbox are strong).
   - **Windows first** if your first users are creators (largest market, more hardware variety).
   - My lean: macOS second, Windows third, but this is your call (see §7).
6. **Contributor strategy:** recruit one platform champion per OS; publish a "Porting guide" and platform status page.

**Rough sizes (small, medium, large), not time estimates:** cross-platform paths and IPC = medium; GStreamer bundling and encoder paths = medium to large; sandbox/shm on Windows = large; signing/notarisation/installers = medium; web-engine UI fixes = small to medium; CI and test matrix = medium.

---

## 3. The market you are entering

AI video editing is crowded in three ways. From the two roundups:

| Group | Examples (as named) | What they do |
|---|---|---|
| Hosted creator tools with agents | Descript, Tella, Riverside, Kapwing, invideo, Cardboard | Chat or agent inside a hosted/web or Mac editor; most expose a hosted MCP server; credits and paid tiers |
| Local timeline editors with MCP | ChatCut, Palmier Pro (macOS), DaVinci Resolve (community MCP bridge; Studio needed), Premiere community bridges | Agents edit real timelines; some local; some fragile after app updates |
| Open-source agent editors | OpenChatCut (AGPL-3.0), FableCut, Cartcut; FFmpeg-based servers (ffmpeg-skill, Kinocut, Cutbolt) | Open, but younger and often lighter-weight or file-in/file-out |
| Clip and generation services | OpusClip, Reap, HeyGen, Runway | Auto-clipping or generating video, not editing your timeline |

Takeaways: "AI video editor" alone is not a position; every vendor says it. Agent access over MCP is becoming standard. The gaps the roundups themselves hint at: local control, trust ("local servers run with your permissions"), fragile bridges to closed apps, plan-gated MCP, and "free code but paid credits".

---

## 4. Positioning: what EEP can honestly own

**Category:** an *agent-operated* video editor, not "an AI video editor". The agent works inside a real editor, on a branch, and you review it.

**One-line positions (pick one to test)**
1. "The open-source video editor your AI agent can actually drive, and you can actually trust."
2. "Agents edit. Humans approve. Every cut is measured, attributed and reversible."
3. "A real video editor built for AI agents: local, open, reviewable."

**Five pillars (each needs proof you can show)**

| Pillar | What it means | Proof asset |
|---|---|---|
| **Open** | AGPL source, inspectable, no lock-in, interchange exports | Licence, repo, XML/OTIO export demo |
| **Local-first** | Media and models run on your machine; bring any agent | Demo with network off; local whisper/SigLIP |
| **Measured** | The agent checks its own work with scopes, loudness, continuity, `verify.run`, not guesses | EEP-Bench results; a "caught the clipped highlight" clip |
| **Reviewable** | Branches, diff, per-edit reason and confidence, accept/reject, agent-only undo | Review-queue screen recording |
| **Agent-agnostic** | Any MCP client (Claude, GPT, local models), curated profiles | Same task run with three agents |

**Who it is for (in this order)**
1. **Developers and automation builders** replacing FFmpeg scripts and render pipelines with an editable, reviewable timeline (podcast clip factories, course production, release videos).
2. **Privacy-sensitive teams** (newsrooms, education, enterprise on-premises, legal/medical) that cannot upload footage to hosted tools.
3. **Creators who already use coding agents** and want them to cut footage.
4. **Linux creators**, who are currently underserved.

**Where not to compete:** generative video (Runway-type), templated social clippers (OpusClip-type), or "replaces Premiere/Resolve". You do not need to win those.

**How to handle "built by AI"**
- It is a strong, true story: the editor was built by agents under rules (`AGENTS.md`, CI gates, provenance checks), and it ships the same verification tools that made that possible.
- But "built by AI" alone sounds cheap or risky to many buyers. Say instead: **"Built with AI agents, under human accountability, and every change tested and attributed."** Let the benchmark and the release gates carry the trust.
- Legal caution: copyright over AI-written code is unsettled in several countries, and your commercial licence depends on you holding rights. Do not put "written entirely by AI" in licence documents or sales contracts; keep that wording for the story page, and ask your lawyer.

**Claims discipline:** only claim what a test shows. Today that is strong on perception tools, branches and measured honesty; the agent UI and `verify.run` are still being built. Do not market "autonomous editing" until EEP-Bench shows it.

---

## 5. Messaging do and don't

| Do | Don't |
|---|---|
| "Open source, local, reviewable" | "Fully autonomous", "replaces editors" |
| Show a 60 s demo: brief → agent edits on a branch → review queue → accept | Show only polished generative output |
| Publish benchmark numbers with failures | Quote unreproducible speed-ups |
| Say which agents and models you tested | Imply it works with every model equally |
| Be clear that you bring your own model (the editor is free; model costs are yours) | Hide that agent plans/credits cost money |
| Say "Linux first; macOS and Windows coming/supported with known gaps" | Promise dates |
| Use your own product names and labels | Use other editors' trademarks (Blackmagic, Adobe) in UI or marketing |

---

## 6. Launch sequence

1. **Foundation (now → AF3/AF6):** finish profiles, actors, `verify.run`, Agent and Review cards. Without these the story is only "MCP over an editor", which competitors already offer.
2. **Proof:** EEP-Bench v0 with 10 tasks, a demo video, a comparison page ("how EEP differs") that is honest about trade-offs.
3. **Public alpha announcement** when a fresh-machine install works and the CLA bot is live: r/rust, Show HN, This Week in Rust, Linux communities, MCP/agent communities. Lead with the demo, not the philosophy.
4. **Cross-platform headless** (CLI + MCP daemon) → announce "runs on macOS and Windows for agents".
5. **Desktop on macOS, then Windows,** each with a signed installer and a known-gaps list.
6. **Commercial:** after the licence cleanup is closed, offer hosted/on-prem support and commercial licences to the teams in segment 1 and 2.

**Naming and tagline note:** "Easy Edits Pro" reads as a generic consumer editor. Consider a sub-brand or tagline that carries the agent idea ("Easy Edits Pro: the editor for agents"), and check trademark availability before investing in a name.

---

## 7. Decisions recorded (2026-11-11)

1. **Support tiers:** Adopted **Tier 1 (Linux)** and **Tier 2 (macOS and Windows)**. Tier 1 has full release gates, local packages, and CI; Tier 2 has CI builds and published packages with documented known gaps.
2. **Headless-first cross-platform:** Confirmed. Port `saturn-cli` and `saturn-mcp` (the daemon) via `saturn-platform` to macOS and Windows before tackling the Tauri GUI desktop shell.
3. **Desktop OS second:** **macOS first** (serving developers and automation builders driving EEP from Claude Code, Codex, or other coding agents), then **Windows** (addressing the broad creator base).
4. **Position line:** Will decide later. The three candidates in §4 remain available to test.
5. **How loudly to say "built by AI":** Story page only, under human accountability and tested gates; avoid using "written entirely by AI" in sales contracts or legal notices.
6. **Maintenance:** Recruit platform champions during the Tier 2 rollout.

## 8. Sources and limits

- [ChatCut: best video editing MCP servers](https://chatcut.io/blog/best-video-editing-mcp-servers) (vendor blog)
- [Tella: best agentic video editors](https://www.tella.com/blog/best-agentic-video-editors) (vendor blog)
- Repo evidence: `README.md` ("no Windows implementation"), `docs/distribution.md` (Windows/macOS "Not built"), `tauri.conf.json` bundle targets, CI runners, and the Unix-specific files listed in §2.
- Not verified: current feature sets of competitors beyond those posts; availability of GES and each plugin in Windows/macOS GStreamer installers; Windows/macOS behaviour of every dependency. Do a one-day spike (§2 step 3) before committing to a schedule.
