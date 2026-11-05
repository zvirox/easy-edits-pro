# FilmCraft vs EEP: provenance check

Not legal advice. Compared FilmCraft `fafdb22` (cloned from `storytold/filmcraft`) with EEP main `9774742` on 2026-11-05.

## Result in one paragraph
**I found no evidence that EEP copied FilmCraft's code in bulk, and FilmCraft's licence would allow it anyway.** I did find two small things that came from FilmCraft and should be credited: the 701-command reference list, and the shape of one data structure. This does **not** block your dual licence. It needs a `NOTICE` entry.

## 1. FilmCraft's licence (verified from its repo)
- `Cargo.toml`: `license = "MIT OR Apache-2.0"`; files `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `ATTRIBUTION.md` exist.
- Its `NOTICE` says the **ArtCraft name, wordmark and logos are trademarks** and are *not* covered by the licence. Do not use them.
- Bundled fonts are OFL-1.1; some assets are public-domain film frames.
- Permissive code can be included in a GPL/AGPL project (Apache-2.0 is compatible with GPLv3), and a commercial licence can be sold over a product that includes it, as long as you keep the copyright and licence notices.

## 2. What I compared
| Test | Result |
|---|---|
| Exact 20-token code runs, all EEP Rust vs all FilmCraft Rust (367,591 windows) | 1,308 match (**0.36%**) |
| Exact 40-token runs | **35 match (0.01%)** |
| Byte-identical files over 200 bytes (any type) | **1**: `assets/fonts/OFL.txt` (the standard SIL font licence for the Inter font, which is correct to include) |
| Identical Markdown lines of 70+ characters | **0** |
| Identifier-blind structural match (names replaced) | 15% at 20 tokens, 1.6% at 40 tokens; this is mostly shared Rust boilerplate (`serde` derives, struct field lists), so I treat it as noise |

The longest exact matches, inspected by hand:
- **`CommandSpec` struct** (`saturn-core/src/catalogue/mod.rs`) has the same fields as FilmCraft's `crates/engine/src/commands.rs` (`id`, `label`, `menu`, `shortcut`, `params`). 53-token run. Small data-structure shape.
- **HSL colour conversion** (`hsl.rs`) and the **`.cube` LUT struct** (`lut.rs`): textbook algorithm and the file-format fields, so any implementer would write nearly the same thing.
- **XML escape function** (`xml_utils.rs`): the standard five replacements.
- Serde derive lines, `fmt` boilerplate, and a test-pixel array.

## 3. The one real finding: the command inventory
`docs/reference/reference-editor-commands.json` has **701 command entries** (id, label, menu, shortcut). **All 701 ids appear verbatim as strings in FilmCraft's Rust sources**, including camelCase ids like `audio.toggleScrubbing` that nobody would invent identically. FilmCraft's own docs table (`crates/engine/src/keyboard.rs`) lists the same label and shortcut for the entries I could parse (4 of 4 labels identical; 3 of 4 shortcuts).
- This is a list of **names, menu paths and shortcuts**: facts and short phrases, so low copyright risk. FilmCraft's licence also allows it.
- **But it should be credited.** MIT/Apache ask you to preserve the notice when you copy material.
- Your `check-parity.mjs` header calls it "the recorded inventory of that product's command catalogue" without naming FilmCraft. Name it.

## 4. What to do
1. Add to `NOTICE` (draft):
   > Portions of the command-name inventory (`docs/reference/reference-editor-commands.json`) and the shape of `CommandSpec` are derived from FilmCraft (https://github.com/storytold/filmcraft), Copyright (c) 2026 ArtCraft Team and the FilmCraft contributors, licensed MIT OR Apache-2.0. Their licence texts are reproduced in `licenses/FilmCraft-MIT.txt` and `licenses/FilmCraft-Apache-2.0.txt`.
2. Copy FilmCraft's `LICENSE-MIT` and `LICENSE-APACHE` into a `licenses/` folder.
3. Add the same line to `docs/PARITY.md`'s source note and `check-parity.mjs`'s header.
4. Tell agents: **study is fine, but do not paste FilmCraft code; where you do reuse something, log it in `NOTICE`.** Add this to `AGENTS.md`.
5. Do not use the names ArtCraft, FilmCraft's logo or app icon.

## 5. A related risk that matters more for selling
FilmCraft deliberately mirrors **Adobe Premiere Pro's** menu names and labels (for example "Essential Graphics", "Essential Sound", "Lumetri Presets" appear in its screenshots), and EEP's UI uses names from Adobe and Blackmagic products: **"Essential Graphics"** panel, **Fusion**, **Fairlight**, **Color/Deliver** workspaces (as in your screenshots). Product names are **trademarks**, which are separate from code licences. They are fine for private study and a hobby build; for a paid product they can draw complaints.
- Before you launch the commercial offer, rename the labels you borrowed (for example "Graphics", "Node compositor", "Mixer", "Grade"), and keep a note of the change.
- EEP already did a first sweep (commit `7785792`, "replace other-editor product names with generic wording"); the UI still shows some.
- Ask your lawyer about names in the interface, in docs and in marketing.

## 6. Limits of this check
- I compared **Rust against Rust**, plus all file hashes and Markdown lines. EEP's TypeScript UI cannot be matched against FilmCraft's Rust UI, so I cannot rule out ported logic or layout ideas there.
- I cannot see what the agents were shown. Copying an *idea* or a *table of facts* is different from copying *expression*; I only tested the second.
- A match score is evidence, not proof either way.
- Method and scripts: `sim.py` and `show.py` in this folder (token n-gram comparison).
