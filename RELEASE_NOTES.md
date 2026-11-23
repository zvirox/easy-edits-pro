# Easy Edits Pro v0.1.3-beta.2

This is the second beta, following `v0.1.3-beta.1`. It adds the authenticated
long-lived `saturn-control` daemon, the Task Sheet web stack (daemon API, Web
Desk, runner and enterprise SSO), and a larger agent harness with real-agent
evaluation mode. The desktop UI has had a craft pass: sentence-case labels,
line icons instead of emoji, and quieter chrome, with the locked plum/magenta
glass palette unchanged.

---

### Breaking risks (read before upgrading)

- **Daemon authentication is now mandatory.** `saturn-control` starts with a
  master token and a project root. `GET /health` is unauthenticated and
  deliberately returns no command inventory; `/commands`, command execution and
  `/ws` require a token, and the WebSocket handshake checks Origin. Scripts that
  called these endpoints without a token will fail. Evidence: commit `a3fb0c8`
  and `crates/saturn-control/tests/security_hardening_tests.rs`.
- **Project schema.** Current schema version is **19** (`PROJECT_SCHEMA_VERSION`
  in `crates/saturn-core/src/project/mod.rs`). Older projects auto-migrate on
  load; opening a newer file with an older build is rejected.
- **GStreamer version floor.** CI package builders use Ubuntu 22.04 (GES 1.20)
  and Debian 12 (GES 1.22); both have recorded install-to-render proofs. Older
  GStreamer/FFmpeg plugin sets can render black or fail at pipeline link. No
  non-development-machine report exists for this beta yet.
- **Proxy format.** Proxies remain a single 1280x720 VP9 WebM profile attached
  per asset; there is no preset system. Proxy playback and cache regeneration
  are still unfinished, so attachments survive an upgrade but are not used for
  playback.
- **Absolute media paths.** Projects embed absolute paths at import. Moving
  media shows clips offline until relinked. Unchanged from alpha, restated
  because beta users carry projects across machines.
- **New, unused dependency.** `@ui-sdk/react` and `@ui-sdk/core` are in the
  lockfile but are not imported anywhere in the source. pnpm reports
  `Agent skills awaiting approval: @ui-sdk/core`. They are kept at the
  maintainer's explicit decision; review that supply-chain flag before using
  this build in production. They are not recorded in `NOTICE` because no code
  from them is reused.
- **Web Desk accounts.** v1 uses local accounts; enterprise OIDC/SAML and RBAC
  are new in this beta and should be evaluated in a non-production deployment
  first.

### Install and launch evidence

- Tagged evidence for earlier releases is recorded verbatim in
  [docs/distribution.md](docs/distribution.md) (install proofs for `.deb`,
  `.rpm` and Flatpak in CI containers and inside the Flatpak sandbox). Those
  rows describe those releases, not this one.
- **Tagged proof for `v0.1.3-beta.2`: PASS** — workflow run
  [45211424805](https://github.com/zvirox/easy-edits-pro/actions/runs/45211424805)
  at tag commit `f98ca5a`, all four jobs green. This is a CI-container proof,
  not a report from a non-development machine:
  - `.deb`, real dpkg install in debian:12: `PASS - container install: h264
    1920x1080, 6.02s, 1465938 bytes`.
  - `.rpm`, real dnf install in fedora:latest: `PASS - container install: vp8
    1920x1080, 6.03s, 251161 bytes`.
  - Flatpak, full Builder build + local install + in-sandbox acceptance drive:
    `PASS - flatpak spike of io.github.zvirox.EasyEditsPro: h264 render verified,
    6.02s, 1477119 bytes`. The bundle and its checksum were attached to the
    release by the proof job.
- Pre-tag CI run [45207688108](https://github.com/zvirox/easy-edits-pro/actions/runs/45207688108)
  was green on the tagged commit (all three jobs). Local gates: `cargo test --workspace`, `pnpm build`, `pnpm test`
  (193/193), `pnpm check:glass`, `pnpm check:commands` (492 shipped),
  `pnpm check:source-size --max-lines=800` and `pnpm test:e2e` (13 areas, 0
  browser errors).

### Commands and parity

- Command catalogue: **492 commands, 492 shipped** (see
  `docs/reference/command-catalogue.json` and generated `docs/COMMANDS.md`).
- Parity (generated, see `docs/PARITY.md`):

  | Tier | Total | na | Effective | Coverage |
  |---|---:|---:|---:|---:|
  | Tier 1 | 443 | 55 | 388 | 100% |
  | Tier 2 | 207 | 84 | 123 | 100% |
  | Tier 3 | 17 | 16 | 1 | 100% |

### Known limits snapshot

Copied from the current README "Known limitations and work in progress" list:

- This is beta software for Linux (v0.1.3-beta.1). Windows and macOS packages
  are not built, and package compatibility still needs reports from
  non-development machines.
- Hardware encoding has not been verified. Export format and codec support
  depend on the GStreamer plugins available on the machine.
- The desktop MCP bridge uses a Unix socket. There is no Windows bridge.
- Text-to-speech and speaker diarization are not implemented. Transcription is
  available; speech separation for dialogue, music, ambience, and effects is
  deferred until a verified, license-cleared model is available.
- Proxy attachments exist, but proxy playback and cache regeneration are
  unfinished.
- EEP-Bench's default run is a scripted deterministic conformance suite. A
  real-agent `--agent` evaluation mode exists, but no real-model baseline has
  been published; scripted results are not model-evaluation results.
- The Task Sheet web API, Web Desk, runner and enterprise SSO are implemented.
  The shared agent-contract crate (TS-X) remains planned until a second product
  consumes it. Cross-platform work (XP1–XP4) and Phase 6 VFX work remain open.
  See the live AI-native progress checklist for current gates and remaining
  work.
