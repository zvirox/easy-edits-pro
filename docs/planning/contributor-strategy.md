# Getting contributors for Easy Edits Pro: where to find them, and how to get the repo ready

Repo state checked: `main` @ `b633ed1` (2026-11-09). Platform facts below are from my own knowledge unless a source link is given; I did not check each site live, so confirm a site is active and read its submission rules before you post.

---

## 1. Is there a platform where you submit a project and contributors come?

No single site does the whole job. Nobody is "assigned" to your project; people find it through lists and search, then decide if it is worth their time. What you can do:

### A. Discovery sites (free, list your repo and its beginner issues)

| Platform | What it does | How you get listed |
|---|---|---|
| **GitHub itself** | Topics, search, Explore, "good first issue" and "help wanted" label pages. This is where most people find work | Add repo topics (`video-editor`, `rust`, `tauri`, `gstreamer`, `linux`, `mcp`, `ai-agents`), a clear description and website, and label issues |
| **Good First Issue** (goodfirstissue.dev) | Indexes projects by their `good first issue` label | Submit your repo; keep the label in use |
| **Up For Grabs** (up-for-grabs.net) | Directory of projects with beginner-friendly tasks | Pull request adding a small project file that names your label |
| **CodeTriage** | Emails volunteers open issues from repos they subscribe to | Sign in with GitHub and add the repo; it needs open issues to send |
| **First Timers Only** (firsttimersonly.com) | Guidance and an issue-label convention for true first-timers | Label a few tiny, fully-specified issues |
| **Hacktoberfest** | October event with a big surge of newcomers | Add the `hacktoberfest` topic in September; next one is October 2027 |
| **Google Summer of Code / Outreachy** | Paid student/intern programmes | Need an accepted org or mentor application; apply under a host organisation first (e.g. a Rust or multimedia umbrella) |

Check each site's current instructions: they change.

### B. Communities where Rust, Linux and video people hang out (you post; they come)

- **Reddit:** r/rust (project showcase), r/linux, r/opensource, r/VideoEditing, r/linuxmasterrace for demos. Read each subreddit's self-promotion rules first.
- **Hacker News:** a "Show HN" post when you have a download and a short demo video.
- **This Week in Rust:** accepts project updates and "call for participation" entries listing issues that need help.
- **Rust Users forum** (users.rust-lang.org) and the Rust community Discord/Zulip: appropriate in showcase channels.
- **GStreamer community** (discourse / Matrix): ask about GES or pipeline problems; this also finds the exact experts you need.
- **Tauri Discord:** same for the shell and plugins.
- **Fediverse and YouTube/Linux creators:** a short "alpha editor for Linux, help wanted" demo reaches users who become contributors.
- **Product Hunt / AlternativeTo:** good for users and testers more than for coders.

### C. Your own place for conversation

- **GitHub Discussions** (switch on in repo settings): Q&A, ideas, show-and-tell, "introduce yourself". Keeps issues clean.
- **A Discord or Matrix room:** only if you can answer in it daily; an empty chat repels people. Start with Discussions, add chat when there are about 10 regular contributors.

### D. Money for contributors (optional later)

GitHub Sponsors and Ko-fi are already in `FUNDING.yml`. Open Collective, bounty platforms (Algora, IssueHunt) and grants (Sovereign Tech Fund-style programmes, NLnet) can fund specific features. Treat these as phase 2; unpaid-contributor onboarding comes first.

---

## 2. How ready the repo is now

**Already good**
- Real `README` with status, known limitations, install and dev setup.
- `CONTRIBUTING.md` with prerequisites per distro, build commands, repo map, PR checklist.
- `CODE_OF_CONDUCT.md`, `SECURITY.md` with private reporting, `LICENSE`, `NOTICE`, `LICENSING.md`, `CLA.md`, `TRADEMARKS.md`.
- Two issue forms (bug, feature) and a CI pipeline (tests, lint, licence checks, UI gates).
- `AGENTS.md` with hard rules for AI agents and path ownership; this is unusually good practice.
- Architecture, command catalogue, parity and plan docs.

**Gaps that will cost you contributors**

| # | Gap | Why it matters | Fix |
|---|---|---|---|
| 1 | No `good first issue` / `help wanted` issues or labels in use; no roadmap | Newcomers cannot see where to start | Create 10–15 small issues (see §4) and a short `ROADMAP.md` |
| 2 | No PR template | PRs arrive without tests, DCO or CLA info | Add `.github/PULL_REQUEST_TEMPLATE.md` (draft in §5) |
| 3 | CLA friction: "comment exactly `I accept…`" with a maintainer recording it by hand; entity not named | Many drive-by contributors leave at a legal form; manual tracking does not scale | Turn on CLA Assistant (or similar) before you promote; name who the CLA is with; keep tiny doc fixes exempt |
| 4 | Contact details are inconsistent: `info@zvirox.in`, `zvirox@proton.me`, `zvirox.in@gmail.com` (security), `team@easy-edits-pro.org` (pyproject) | Contributors don't know where to write; security reports might go to a dead address | Choose one project address per purpose (general, security, licensing) and use it everywhere |
| 5 | `AGENTS.md` still says "Prowl project context" | Confuses humans and agents; looks unfinished | Rename to Easy Edits Pro |
| 6 | `CONTRIBUTING.md` PR checklist numbers two items "5." and is dense | First impression is a wall of rules | Add a 10-line "Your first contribution" at the top (§5) |
| 7 | No `SUPPORT.md`, no Discussions, no `CODEOWNERS`, no `GOVERNANCE.md` | People don't know where to ask or who reviews | Add short files (drafts in §5) |
| 8 | Docs folder mixes user docs, internal plans, audits and agent task cards (about 30 files) | A newcomer cannot tell what to read | Add `docs/README.md` index with three groups: Start here, Reference, Internal planning (not required reading) |
| 9 | `CONTRIBUTING` says licence headers/SPDX needed in practice (CI enforces) but does not explain | First PR fails on a check the guide never mentioned | One paragraph and the one-line command to add headers |
| 10 | No policy on AI-generated contributions | The project is largely agent-built; maintainers need a rule | State it plainly (§3) |
| 11 | README says Linux only, alpha, with known limitations; good, but no "what help we need" list | People can't match skills to tasks | Add "Where we need help" list with areas and difficulty |
| 12 | Heavy build prerequisites (GStreamer dev packages, GES, WebKitGTK) | Biggest drop-off for first-timers | Provide a devcontainer or a tested `scripts/setup-dev-ubuntu.sh`, and a "check your setup" command |
| 13 | Licence issues from the earlier audit may still be open (deny.toml allow list, packaging notices, codec packages) | A contributor or company lawyer will check | Close them before promoting; recent commits suggest notice bundling is in progress |

---

## 3. What to say, and what not to say

**Say (in README and CONTRIBUTING)**
- What EEP is in two sentences, current status (alpha), supported platform (Linux), and a screenshot or 60-second demo.
- Licence in plain words: AGPL-3.0-or-later plus a commercial option; **contributors keep their copyright and grant a licence through the CLA, and why** (so the project can offer both licences). Honesty here builds trust; hiding it causes drama later.
- The real known limitations (you already list them).
- Where help is needed, ranked by difficulty, with named maintainers or areas.
- The exact "first PR" path, how long reviews usually take, and what a good PR looks like.
- That AI agents contribute to the codebase, which rules they follow (`AGENTS.md`), and that humans are accountable for what they submit.
- Policy for AI-assisted contributions: allowed, you must understand and test what you submit, disclose significant AI use in the PR, no pasted third-party code, same DCO/CLA rules apply. Maintainers may close low-effort bulk PRs.
- Response expectations that you can keep (for example "we try to reply within 7 days"). Do not promise what you cannot keep.

**Do not say or include**
- "Production ready", "Hollywood-grade", unsourced benchmarks or comparisons with Resolve/Premiere. Your known-limitations section is stronger.
- Feature promises with dates. Use "planned" and "help wanted".
- Revenue plans, pricing talk or internal monetisation strategy in contributor-facing files. Keep a short "How the project is funded" paragraph only if asked.
- Other companies' product names as your UI labels or marketing (earlier audit flagged Blackmagic/Adobe-style names).
- Secrets, personal phone numbers or personal emails in docs; use project addresses.
- Long internal agent task cards or audit reports as "required reading". Mark them internal.
- Anything that implies you will review code instantly, 24/7.

---

## 4. Onboarding flow (what a new contributor should experience)

1. **Land on README** → sees demo, status, "Where we need help", link "Start contributing".
2. **Set up in under 30 minutes:** `scripts/setup-dev-*.sh` or devcontainer, then `scripts/doctor.sh` (checks Rust, Node, GStreamer, GES, runs one smoke test). Document known failures per distro.
3. **Pick an issue:** filter by label. Each starter issue has: context, files to look at, acceptance test, estimated size, mentor name.
4. **Claim by comment** ("I'd like to work on this"); maintainer assigns within 48 h; unclaimed after 14 days returns to the pool.
5. **Open PR** using the template; bot checks DCO and CLA, CI runs; one comment explains failures in plain words.
6. **Review in one round for small PRs.** Be specific, kind, and fast on the first PR; first-time experience decides whether there is a second.
7. **Merge, thank, credit.** Changelog entry, contributors list, "next good issue" suggestion.
8. **Grow:** after 3 merged PRs, invite to the triage team or give an area to own (`CODEOWNERS`).

**Starter issue ideas drawn from the repo (examples; verify each is still open work)**
- Add a missing icon or fix a truncated label (UI).
- Add tests for a command lacking one (`check:commands` shows gaps).
- Improve an error message in an importer.
- Translate UI strings or docs.
- Add a keyboard shortcut to the cheat sheet.
- Write a "how to profile a render" doc.
- Test the package on a distro not yet verified and report.
- Add an `ingest` or `review` MCP recipe to the agent harness once profiles exist.

Label scheme: `good first issue`, `help wanted`, `needs design`, `needs repro`, `area: ui|engine|render|audio|agent|docs|packaging`, `difficulty: easy|medium|hard`, `blocked`, `question`.

---

## 5. Ready-to-adapt text

**CONTRIBUTING.md: add at the very top**

```markdown
## Your first contribution in 6 steps
1. Read the README "Known limitations", then pick an issue labelled `good first issue`.
2. Comment "I'd like to work on this". We will assign it.
3. Run `scripts/setup-dev-ubuntu.sh` (or see Development Prerequisites) and `scripts/doctor.sh`.
4. Make a small change on a branch; run the checks in the PR checklist.
5. Commit with `git commit -s` (DCO). Accept the CLA once when the bot asks.
6. Open a PR using the template. We aim to reply within 7 days.
Not sure? Ask in Discussions first. No question is too small.
```

**`.github/PULL_REQUEST_TEMPLATE.md`**

```markdown
## What and why
<!-- One or two sentences. Link the issue: Fixes #123 -->

## How I tested it
- [ ] `cargo test --workspace`
- [ ] `pnpm --dir apps/saturn-tauri build` (and `test`, `check:glass`, `check:commands` if UI/commands changed)
- [ ] Manual check (describe)

## Checklist
- [ ] Commits are signed off (`git commit -s`)
- [ ] I accept the CLA (first non-trivial PR)
- [ ] No third-party code pasted; any new dependency is in NOTICE and passes `cargo deny`
- [ ] Source files stay under 800 lines
- [ ] AI-assisted? Tool used and what I verified:
```

**`SUPPORT.md`:** where to ask (Discussions for questions, Issues for bugs, security via the private advisory, licensing/commercial via the single licensing address).

**`CODEOWNERS`:** map `crates/saturn-render/` etc. to maintainers so reviews route themselves (start with you everywhere, add owners as they appear).

**`GOVERNANCE.md` (half a page):** who decides, how someone becomes a maintainer, how disputes and licence changes are handled.

**Announcement post (for r/rust, Show HN, This Week in Rust)**

```markdown
**Easy Edits Pro: an open-source Linux video editor in Rust (alpha), built so AI agents can drive it too**
Rust core, Tauri 2 + React UI, GStreamer/GES rendering, MCP server for AI agents.
Alpha .deb/.rpm packages are available. It's early: [link to known limitations].
We're looking for help with: GStreamer/GES performance, timeline UI, packaging for other distros, tests, translations.
Starter issues: [link]. Licence: AGPL-3.0-or-later + commercial option (CLA, you keep your copyright).
Repo: github.com/zvirox/easy-edits-pro
```

---

## 6. Two-week launch checklist

| Days | Do |
|---|---|
| 1–2 | Fix contact addresses; rename "Prowl" in `AGENTS.md`; fix CONTRIBUTING numbering; set repo topics, description, social preview image |
| 3–4 | Add PR template, SUPPORT, CODEOWNERS, labels; enable Discussions; add `docs/README.md` index |
| 5–6 | Turn on CLA bot; add AI-contribution policy; check the remaining licence fixes from the audit |
| 7–9 | Write `setup-dev` script or devcontainer and `doctor` script; test on a clean Ubuntu 24.04 VM or container and a Fedora one |
| 10–11 | Write and label 10–15 starter issues with acceptance tests; add `ROADMAP.md` |
| 12 | Record a 60–90 s demo; update README top section |
| 13 | Submit to Good First Issue, Up For Grabs, CodeTriage |
| 14 | Post to r/rust, This Week in Rust call for participation; Show HN once a fresh-machine install works |

Afterwards: answer every first PR within a week, keep a public "merged this week" note, and watch the first five contributors closely, since their experience sets your reputation.

---

## 7. Cautions

- **Review capacity is the real limit.** Ten contributors will produce more PRs than one maintainer can review. Start small and grow reviewers from contributors.
- **Do not merge outside code until the CLA flow is working**, as in the dual-licensing plan.
- **Keep AI-agent PRs and human PRs under the same rules** and the same CI.
- **Expect low-effort PRs** after promotion (AI-generated, drive-by). The template, the claim system and a written policy let you decline politely.
- **Legal:** the CLA text and the commercial arrangement still need a lawyer; I am not one.

Sources: [GitHub: setting up your project for healthy contributions](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions), [GitHub: encouraging helpful contributions with labels](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/encouraging-helpful-contributions-to-your-project-with-labels).
