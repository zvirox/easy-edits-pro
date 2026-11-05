# Dual licensing plan for Easy Edits Pro

**I am not a lawyer and this is not legal advice.** Use this as a checklist and as the brief you give an open-source licensing lawyer before you sell anything. Facts below come from your repo at `9774742` unless marked "from memory".

---

## 1. What dual licensing means for EEP

You publish the same code under two licences:

| Licence | Who it is for | Cost | Obligation |
|---|---|---|---|
| **Open licence** (GPL-3.0-or-later today, or AGPL-3.0-or-later) | Individuals, hobbyists, researchers, open-source projects | Free | Anyone who distributes (GPL) or hosts (AGPL) a modified version must publish their source under the same licence |
| **Commercial licence** (written by you) | Companies that want to embed EEP in a closed product, or host it as a service without sharing code, or want support/warranty | Paid | None of the copyleft duties; the terms in your contract instead |

It works **only if you hold the right to relicense every line you ship**. Right now that looks true, which is your window.

---

## 2. What I found in the repo

| Check | Result | What it means |
|---|---|---|
| Authors | All 251 commits are from one identity (`zvirox`, three email forms). No outside contributors yet | **You can still relicense freely.** After outside code is merged, this gets hard |
| Current licence | `GPL-3.0-or-later` in every crate `Cargo.toml`, plus `LICENSE` (GPL v3) | Past alpha releases stay GPL for everyone who has them. You cannot withdraw that, and it does not block a new commercial licence |
| Contribution terms | `CONTRIBUTING.md` requires a **DCO** sign-off certifying the work can go "under the project's GPL-3.0-or-later license" | **A DCO is not enough for dual licensing.** It says the contributor may submit under GPL; it does not give you the right to sell the code under other terms. You need a CLA (§4) |
| `NOTICE` / `CLA` / `LICENSING` files | None exist | To add |
| SPDX headers in source files | None found in `.rs`/`.ts` files | To add (§5) |
| FilmCraft | Named throughout `CHANGELOG.md` as the parity target ("FilmCraft Tier-1 parity fulfilment") | **Open question for you, see §3.1** |
| x264 / H.264 | Referenced in `saturn-render/src/lib.rs`, `commands/diagnostics.rs`, `commands/synthetic.rs`, README | **Open question**, see §3.4 |

---

## 3. Blockers to clear before you sell a commercial licence

### 3.1 FilmCraft provenance (checked, not a blocker)
FilmCraft is **MIT OR Apache-2.0**, which allows reuse in a GPL/AGPL product and in a commercial licence if you keep its notices. I compared both codebases (see `filmcraft-provenance-check.md`): **no sign of bulk code copying** (0.01% exact overlap at 40 tokens, one identical file that is the standard font licence). Two small items came from FilmCraft and need a `NOTICE` credit: the 701-entry command inventory and the shape of `CommandSpec`. Do not use the ArtCraft name or logos. A bigger commercial risk is borrowed Adobe/Blackmagic product names in the UI (see that file, section 5).

### 3.2 AI-generated code
Most of the code was written by AI coding agents. Whether purely AI-written code is protected by copyright, and who owns it, differs by country and is unsettled in several (from memory; ask your lawyer). It matters because a licence can only be sold over rights that exist.
- Keep a human in the loop: you review, direct, edit and select, and commits are made under your name.
- Check each agent tool's terms about who owns outputs and any restrictions.
- Expect your lawyer to ask for this record.

### 3.3 Dependencies must allow it (the most technical blocker)
A commercial licence cannot override someone else's licence. Rust links crates **statically**, so copyleft dependencies matter.
- Run `cargo deny` or `cargo about` (and `pnpm licenses list` for the UI) and list every licence.
- **Fine:** MIT, Apache-2.0, BSD, ISC, MPL-2.0 (with care), Zlib.
- **Problem:** GPL-only or AGPL-only crates; LGPL crates linked statically (they impose relinking duties). Replace or remove these from the commercial build.
- **GStreamer** is LGPL and loaded dynamically as a system library, which is normally fine. Plugin sets differ (see 3.4).
- Add a CI check that fails the build on a disallowed licence (`cargo deny check licenses`).

### 3.4 Codecs and plugins
- The **x264** encoder is GPL (from memory). If your commercial build **bundles** it, you inherit GPL duties and would need x264's own commercial licence. Safer: do not bundle it; use hardware encoders (VA-API/NVENC) and any codecs the user's system already provides, detected at run time.
- **Patent licensing for H.264/HEVC/AAC is separate from software licences.** Selling a product that ships encoders can create patent-pool obligations. Ask your lawyer; this is common to all video editors.

### 3.5 Model weights (for the AI roadmap)
Weights have their own licences (see the review file). From memory: RobustVideoMatting is GPL-3.0, and the larger Depth-Anything-V2 models are non-commercial. A commercial feature must not depend on a weight you cannot license commercially. Keep a table `docs/model-licences.md` with name, source, licence, and "OK for commercial? yes/no".

---

## 4. The contributor agreement (do this before merging any outside code)

Replace "DCO only" with a **CLA**. Two common forms:

| Form | What contributors give you | Pros | Cons |
|---|---|---|---|
| **Licence-grant CLA** (Apache-style ICLA) | They keep their copyright and grant you a perpetual, worldwide right to use, modify, **sublicense and relicense** their contribution | Friendlier; most contributors accept | Slightly weaker than owning it |
| **Copyright assignment** | They transfer the copyright to you | Simplest for relicensing and enforcement | Many contributors refuse; some countries limit assignment |

**Recommendation:** licence-grant CLA with an explicit "may relicense under any terms, including proprietary" clause, signed through a CLA bot (for example CLA Assistant on GitHub) so every PR is checked automatically.

Until the CLA is live:
1. **Do not merge outside pull requests** that contain code. Documentation or typo fixes are low-risk.
2. Add a notice in `CONTRIBUTING.md` and the PR template.
3. Keep the DCO too; it still documents origin.

If a contributor cannot or will not sign, rewrite their change yourself rather than merging it.

---

## 5. Choosing the open licence: GPL or AGPL

| | **GPL-3.0-or-later** (today) | **AGPL-3.0-or-later** |
|---|---|---|
| Stops closed redistribution of the desktop app | Yes | Yes |
| Stops a company **hosting** your engine/MCP server as a paid service without sharing code | **No** | **Yes** |
| Makes companies buy the commercial licence | Only if they redistribute | **Also if they host over a network** |
| Corporate acceptance | Widely accepted | Some companies ban AGPL (this pushes them toward your paid licence, but loses some adopters) |

**Recommendation for your plan (hosted EEP Cloud, AI agents over a network): AGPL-3.0-or-later for the whole repository, plus the commercial licence.** The change is easy now because you hold all copyright. GPL-3 and AGPL-3 can be combined under their own terms, so mixed-licence dependencies are not a problem on their own. If you prefer to keep the GUI plain GPL, split by crate: GPL for `apps/saturn-tauri`, AGPL for engine/daemon crates. Splitting makes the story more complicated, so I would use one licence.

Your **MCP clients** (Claude, GPT and others) stay separate programs talking over a socket, so they are not covered by the AGPL.

---

## 6. What the commercial licence should say (outline for your lawyer)

1. **Grant:** a non-exclusive, non-transferable licence to use, modify and distribute EEP in a proprietary product, without copyleft duties.
2. **Scope options:** per developer (seat), per product, per hosted deployment, or per render/minute.
3. **Fees and term:** annual or perpetual-for-version, with update rights and a defined support level.
4. **Exclusions:** third-party components keep their own licences (list them); codecs and patents are the customer's responsibility.
5. **Warranty and liability:** as-is with a capped liability; any IP indemnity only if your lawyer advises you can stand behind it.
6. **Trademark:** no right to use the name "Easy Edits Pro" or logo unless agreed.
7. **Audit and termination clauses.**
8. **Governing law and dispute venue** (Indian law/venue, or neutral).
9. A separate **support/SLA schedule**.

Price reference (real data point): Remotion sells company licences at $25/month per developer or $0.01 per render, with a $100/month minimum, and free for individuals and teams up to 3 ([Remotion licence FAQ](https://www.remotion.dev/docs/license/faq)). Use it as a sanity check, not a template. Remotion is "source-available", not open source; yours stays open source.

---

## 7. Files and changes to make in the repo

**Licence layout**
```
LICENSE                      # AGPL-3.0 text (or GPL-3.0 if you keep GPL)
LICENSING.md                 # plain-English: who needs which licence, how to contact you
LICENSE-COMMERCIAL.md        # short pointer + request form; the real agreement is a signed contract
NOTICE                       # third-party licences, FilmCraft record, model-weight table
CLA.md                       # contributor licence agreement text
docs/model-licences.md       # weights, source, licence, commercial OK?
deny.toml                    # cargo-deny allow-list
TRADEMARKS.md                # how the name and logo may be used
```

**SPDX header** at the top of every source file (add with a script, then a CI check):
```
// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-EEP-Commercial
// Copyright (c) 2025-2026 Prakashpuri Gauswami
```
Put `license = "AGPL-3.0-or-later OR LicenseRef-EEP-Commercial"` in each `Cargo.toml` (use the `license-file` field for the custom reference if the registry complains), and the same in `package.json`.

**`LICENSING.md` skeleton**
```
Easy Edits Pro is dual licensed.
1. Open source: AGPL-3.0-or-later (see LICENSE). Free for personal use,
   open-source projects and anyone who shares their source as required.
2. Commercial: for use in closed-source products or hosted services without
   source sharing, and for support/SLA. Contact: <your business email>.
Contributors sign the CLA (see CLA.md) so that we can offer both licences.
```

**`CONTRIBUTING.md` change:** keep the DCO paragraph, add: "By opening a pull request you agree to the CLA in `CLA.md`; a bot will ask you to sign once."

**Copyright line:** decide whether the holder is **you personally** or a **company**. If you plan to sell licences, form a legal entity first (in India that is usually an LLP or private limited company; ask an adviser) and assign the copyright to it, so the business, not your personal name, signs contracts. This also keeps the Ko-fi/Sponsors money separate from product revenue.

---

## 8. Sequence (do in this order)

| Step | Action | Who |
|---|---|---|
| 1 | Answer the FilmCraft question (§3.1); find its licence | You |
| 2 | Pause merging outside code | You |
| 3 | Run `cargo deny`, `pnpm licenses`; list GPL/LGPL/AGPL-only deps | An agent, you review |
| 4 | Pick AGPL vs GPL (§5) | You, with lawyer |
| 5 | Book a one-time consult with an open-source/IP lawyer using this file as the brief | You |
| 6 | Write `CLA.md`, `LICENSING.md`, `NOTICE`, `TRADEMARKS.md`; turn on the CLA bot | Agent drafts, lawyer reviews |
| 7 | Add SPDX headers and CI checks (`cargo deny`, header check) | An agent |
| 8 | Change `LICENSE` and release a new version under the new terms | You |
| 9 | Form the entity; assign copyright; write the commercial agreement | You + lawyer |
| 10 | Publish a pricing page and "contact us" for commercial use | You |

Steps 1–4 take days. Steps 5–10 depend on the lawyer and the entity setup.

---

## 9. Risks and honest limits
- **Dual licensing earns little by itself.** It pays when companies *need* your code and cannot accept copyleft. For EEP the stronger income is hosted cloud and enterprise support (see the review file); dual licensing is the legal frame that protects those.
- **Relicensing later is hard** once outside contributors exist without a CLA, so the CLA must come first.
- **A fork is allowed** under the open licence. Your protection is the name, the hosted service, the roadmap speed and the trust you build.
- **Code you cannot relicense** (copied FilmCraft code, GPL-only dependencies) must be removed or rewritten before the first sale.
- Rules differ by country. Have a lawyer in your jurisdiction review the contract and the CLA.
