# AF0 dependency and package inventory

**Captured:** 2026-11-08. **Result:** baseline inventory recorded; licence-policy
and package-remediation gates remain open. This is evidence for review, not an
approval of every listed dependency or a complete release SBOM.

Companion: [reviewed actor design](../planning/af0-actor-design.md).
Progress authority: [AF0 checklist](../AI_NATIVE_PLAN_CHECKLIST.md#af0--baseline-housekeeping-and-actor-design).

## 1. Baselines and coverage

| Input | Exact baseline / scope |
|---|---|
| Current source | `62ac896584913c3eb3236d80d53f24cc7ab07367`, clean tracked source before this audit. |
| Root Rust workspace | Locked default-feature metadata, all workspace members, with and without `--filter-platform x86_64-unknown-linux-gnu`. Includes development/build dependencies, not just shipped code. |
| Desktop Rust workspace | Separate `apps/saturn-tauri/Cargo.toml` and lockfile; same two metadata queries. Root metadata does not cover this workspace. |
| JavaScript | Installed pnpm licence reports, full and production graphs on Linux x86_64. Not an all-platform optional-package inventory or proof of tree-shaken bundle contents. |
| Python | Declared harness requirements plus metadata from the local interpreter. No lockfile or built wheel was audited. |
| Actual distribution payloads | All five assets from [v0.1.2-alpha.2](https://github.com/zvirox/easy-edits-pro/releases/tag/v0.1.2-alpha.2), published 2026-11-07; tag resolves to `fe00632076dbb3112d59a4c47ff3d9c2e31ce512`. Every asset's SHA-256 matched the GitHub API digest. |
| Native/runtime | ELF direct `DT_NEEDED`, selected vendored/build-fetched source licences, Flatpak metadata and payload. No execution/install of the downloaded application; no recursive runtime/plugin closure or recovered static-link map. |

**Source and release are different baselines.** The release predates the plugin
worker feature and later edits. Current dependency versions cannot be assigned to
that binary from current lockfiles. Rebuild/current-release inspection remains a
packaging exit criterion. Absence of a dynamic library entry alone does not prove
what was statically linked or embedded in a binary.

Local tools: Cargo/rustc 1.99.1, cargo-deny 0.20.2, pnpm 12.17.0, Python 3.14.9,
GNU readelf 2.47. The package workflow specifies pnpm 10; this local pnpm report
is not evidence of the exact release frontend install. Prowl discovery failed
twice (`command not found`); source inspection used targeted reads.

### Durable evidence

All tables are under [af0-evidence/](af0-evidence/). They contain observed metadata,
not licence selections. Empty Cargo scope membership means absent from that
resolved graph; `root`/`desktop` are Linux-filtered, `*-all` are unfiltered.

| File | Contents |
|---|---|
| [cargo.csv](af0-evidence/cargo.csv) | 710 unique name/version/source rows, declared licence, first-party/path classification, repository URL, native link label, membership and resolved features for all four graphs. |
| [javascript.csv](af0-evidence/javascript.csv) | 36 installed package versions and production-graph membership (11). |
| [python-installed.json](af0-evidence/python-installed.json) | Installed versions/metadata and explicit unavailable packages. |
| [release-assets.csv](af0-evidence/release-assets.csv) | Five assets, download URLs, byte counts and verified SHA-256. |
| [package-files.csv](af0-evidence/package-files.csv) | 107 regular-file/symlink entries across extracted deb, rpm and Flatpak, including sizes, SHA-256 and symlink targets; directories omitted. |
| [elf-needed.csv](af0-evidence/elf-needed.csv) | Direct shared-library names for each packaged app binary. |
| [deny-root.txt](af0-evidence/deny-root.txt), [deny-desktop.txt](af0-evidence/deny-desktop.txt) | Successful current-policy licence runs; repository prefix normalized. |
| [capture-hashes.csv](af0-evidence/capture-hashes.csv) | SHA-256 of raw metadata/API/log captures retained locally in `/tmp/opencode/af0-inventory/`. Raw Cargo metadata includes machine-specific paths and is not checked in. |

Audited lock/policy inputs (SHA-256):

```text
1bfabd7255e008f45d2dc6abd6561cf6102dedd61c745d96dd147cf62e4438ad  Cargo.lock
e265dce8d026b10396a7ac115a8845e2bd16ed4471e2dd43db3cd56b8cd706f4  apps/saturn-tauri/Cargo.lock
f7f0d40c1bc1ea443a814e4f0698d7deaf0bdb604eaeffd13eeb0a5750c4a027  apps/saturn-tauri/pnpm-lock.yaml
ab944f34f74c73e5864a3936be83c84224917e141ec7af7db80576353b752030  agent-harness/pyproject.toml
e0d8e63ea22d032abfa8ab7f31101f2f350b6455f1ab93574110d66a0133e161  deny.toml (original broad policy, audited above)
860497c4ad335f66571fa7f80726a88d1a05b0fd0c3723c6a97388b6a5b0c0e1  deny.toml (scoped AF0 policy with first-party exceptions)
b6a962b6d308c5eb1938bc4510a08327e94fcfec4ab875dc5f1d396354a95e26  crates/saturn-audio-dsp/Cargo.toml (licence corrected to AGPL)
06dd43ed49f109ea34caa87a48731d916196116b1a9df81ce57b4ab1a018c35c  crates/saturn-plugin-worker/Cargo.toml (licence corrected to AGPL)
2da838d0ecfc0696b4c941ce089c392c0fd9ab7d998135430a203bb2069e4806  crates/saturn-plugin-fixture/Cargo.toml (licence corrected to AGPL)
c1ad9db972ab9faf3d871284950c84f1eaf6a6e67146c45625e9e02d93d2280e  apps/saturn-tauri/package.json (licence field added)
```

## 2. Rust and ownership

| Resolved graph | Packages | First-party paths | Third-party |
|---|---:|---:|---:|
| Root Linux | 386 | 14 | 372 |
| Desktop Linux | 525 | 6 | 519 |
| Root all targets | 446 | 14 | 432 |
| Desktop all targets | 661 | 6 | 655 |

These are package-version graph counts, not binary-component counts. Default
features were resolved; this does not enumerate all possible feature combinations.
No resolved Cargo package had a missing declared licence expression.

The first-party set is the repository's exact manifest paths, not any package
whose name starts with `saturn-`. Root contains `saturn-core`, `saturn-cli`,
`saturn-render`, `saturn-commands`, `saturn-mcp`, `saturn-control`, `saturn-timeline`,
`saturn-graphics`, `saturn-project`, `saturn-text`, `saturn-perception`,
`saturn-audio-dsp`, `saturn-plugin-worker` and `saturn-plugin-fixture`. Desktop
contains `saturn-tauri` plus core/render/perception/text/audio-dsp.

**Metadata contradiction — resolved 2026-11-08:** audio-dsp, plugin-worker and
plugin-fixture had declared `MIT OR Apache-2.0` in their manifests while their
source headers declared `AGPL-3.0-or-later` plus commercial availability. The
maintainer chose the least-rights correction: manifest metadata now declares
`AGPL-3.0-or-later` for those three crates (matching every other first-party Rust
manifest, the applied SPDX headers and NOTICE); the commercial entitlement stays
in `LICENSING.md` and the header comment. The private UI package also gained
`"license": "AGPL-3.0-or-later"`. Third-party material in first-party artifacts
keeps its own terms.

### Expressions needing explicit treatment

| Component | Observed expression / implication |
|---|---|
| `cssparser 0.37.0`, `cssparser-macros 0.7.1`, `dtoa-short 0.3.5`, `option-ext 0.2.0`, `selectors 0.38.0` | MPL-2.0-only entries in the desktop graph, absent from root. Review exact roles and obligations rather than assuming root CI covers them. |
| `self_cell 1.3.0` | Apache-2.0 OR GPL-2.0-only; a permissive alternative exists. |
| `r-efi 5.3.0` and `6.0.0` where present | MIT OR Apache-2.0 OR LGPL-2.1-or-later; a permissive alternative exists. |
| `jpeg-encoder 0.6.1` | `(MIT OR Apache-2.0) AND IJG`; the AND obligation cannot be dropped. |
| `ring 0.17.14`, `dpi 0.1.2`, `brotli 9.0.0`, `matchit 0.7.3`, `unicode-ident 1.0.26` | Compound AND expressions; preserve all required terms when generating notices. See CSV for exact expressions. |
| Unicode tables, `webpki-root-certs` | Unicode-3.0 and CDLA-Permissive-2.0 data licences already occur; do not reduce the inventory to only MIT/Apache names. |

No third-party GPL/AGPL/LGPL-**only** Cargo expression was found in these graphs.
That statement does not cover native payloads, codecs, model weights or arbitrary
plugins. MPL-only dependencies remain an explicit policy-review item.

Both `cargo deny` runs ended `licenses ok` under the original broad policy, which
globally allowed AGPL, GPL, LGPL and MPL with an empty exceptions list and ran in
CI on the root workspace only.

**Scoped policy (this change, AF0):** `deny.toml` no longer allows AGPL/GPL/LGPL
identifiers globally. AGPL-3.0-or-later is granted only through per-crate
exceptions naming the 15 first-party workspace members, which cargo-deny matches
by crate name. Because a foreign crates.io crate with a colliding name would
inherit such an exception, `scripts/check-licence-policy.mjs` additionally
asserts, for both resolved graphs, that every AGPL-declared package's manifest
resolves inside this repository and that the name set matches exactly
(14 root, 6 desktop). Production JavaScript dependencies are checked against a
deny-by-default allow-list (11 packages today). The negative controls self-test
both directions in throwaway scratch workspaces (never shipping dependencies):
a third-party GPL-3.0-only dependency fails cargo-deny, and a foreign crate named
`saturn-core` passes cargo-deny by name but is refused by the repository-path
assertion. CI now runs cargo-deny on both workspaces plus this policy script
(`.github/workflows/ci.yml`, `licences` job). MPL-2.0 remains allow-listed under
the dual-licensing plan's file-level-copyleft rationale, and the MPL-only crates
above stay an item for the notice work to document. A remaining honest limit:
Python/native coverage is not part of this automated gate yet.

## 3. JavaScript and Python

Production JS versions: `@tauri-apps/api 2.12.1`, `@tauri-apps/plugin-dialog 2.8.1`,
`framer-motion 14.2.0`, `motion 14.2.0`, `motion-dom 14.2.0`, `motion-utils 14.1.1`,
`react 19.3.0`, `react-dom 19.3.0`, `scheduler 0.28.0`, `lucide-react 1.57.0`,
`tslib 2.8.1`. Their reported licences are MIT/Apache alternatives, MIT, ISC and
0BSD respectively; `tslib` specifically needs the 0BSD expression retained.

`lightningcss` and `lightningcss-linux-x64-gnu 1.33.0` report MPL-2.0 and occur only
in the full/build-side installed graph. Their presence does not prove that their
code is shipped in the app. Neither pnpm report inspects uninstalled cross-platform
optional binaries or licence notices embedded inside native build tools.

`agent-harness/pyproject.toml` requires `click>=8.1.0`, `tabulate>=0.9.0`,
`setuptools>=77` and `wheel`; development adds pytest. Local metadata reports
click 8.5.0 (BSD-3-Clause), tabulate 0.10.0 (MIT), pytest 9.2.0 (MIT). The harness
distribution, setuptools and wheel are not installed in this interpreter. These
are local observations, not a reproducible wheel dependency resolution. An isolated
build/install inventory and its transitive/environment-marker dependencies remain
needed before distributing the harness as an audited package.

## 4. Native, embedded, assets and model boundary

Cargo wrapper licences do not inventory all native code. Source paths in this
table are relative to the named crate's downloaded registry directory unless
explicitly repository paths. These are current-source/local-cache observations.

| Component | Evidence / role | Follow-up |
|---|---|---|
| OpenSSL 3.6.3 through `openssl-src 300.6.1+3.6.3` | Desktop explicitly enables vendored OpenSSL; resolved `openssl-sys` has `vendored`. Native `openssl/LICENSE.txt` is Apache-2.0, separate from wrapper MIT/Apache files. Root's default graph differs. | Include actual built OpenSSL notices and build identity; do not describe all native code as system-loaded. |
| ONNX Runtime 1.22.0 via `ort-sys 2.0.0-rc.10` | Default `download-binaries`/`copy-dylibs`; `build.rs` has static-link paths. CPU Linux archive URL/digest below; matching local cache contains only `onnxruntime/lib/libonnxruntime.a`. Bindings have MIT/Apache licences; runtime is recorded as MIT in the model ledger. | Obtain version-matched runtime and embedded third-party notices plus actual release link/build evidence. The cached `.a` alone supplies no complete component/licence inventory. |
| whisper.cpp/ggml via `whisper-rs-sys 0.13.1` | Vendored C/C++ inference source; `whisper.cpp/LICENSE` is MIT (ggml authors). Binding crate reports Unlicense. | Bundle native notice, including copyright; distinguish inference engine from downloaded weights. |
| SQLite through `libsqlite3-sys 0.35.0` | `rusqlite` enables bundled SQLite. Crate wrapper is MIT; native SQLite is public domain as recorded in perception manifest. | Record actual amalgamation/build version; do not apply the unused SQLCipher subdirectory's licence to the selected SQLite build. |
| Oniguruma through `onig_sys 69.9.3` | Wrapper reports MIT; vendored `oniguruma/COPYING` contains a two-clause BSD-style notice (K. Kosako). Tokenizer uses `onig`. | Preserve native notice as well as wrapper licence. |
| `ring 0.17.14` | Native crypto source; `LICENSE`, `LICENSE-BoringSSL`, `LICENSE-other-bits`; graph expression Apache-2.0 AND ISC. | Generate from actual included notices, not just a single chosen identifier. |
| GTK/WebKit/GStreamer/GES and platform libraries | Direct ELF dependencies are retained in `elf-needed.csv`. Debian/RPM rely on system packages; Flatpak provides a runtime plus bundled GES. | Inventory recursive runtime and runtime-loaded plugins per supported distribution; sys-crate licence is not native-library licence. |
| Bundled GES | Flatpak includes `libges-1.0.so.0.2611.0`, GES/NLE plugins and GES `COPYING`/`COPYING.LIB`. Current manifest pins 1.26.11. | Preserve its shipped texts and review source/redistribution requirements for the selected build. The general system-library statement in NOTICE is incomplete for this artifact. |
| UI font | `assets/fonts/InterVariable.ttf` + `assets/fonts/OFL.txt` recorded in NOTICE; UI uses `apps/saturn-tauri/ui/public/fonts/InterVariable.woff2` (`01-editor.css:469`). | Verify converted/distributed font provenance and include OFL/copyright for the actual bundled file. |
| Models | `docs/model-licences.md` records approved fetch-on-first-use weights and rejected/restricted models. No standalone weight files found in inspected payloads. | Keep engine/runtime downloads separate from weights; current package listing is not proof of every embedded binary byte. Cinema stems remain model/licence-blocked. |
| Plugins | First-party worker hosts externally supplied CLAP plugins via permissive bindings; plugin licences are independent of host licence. | Worker packaging plus plugin-root/sandbox acceptance is still pending; do not bundle unreviewed plugins. |

ONNX CPU archive specified by the installed rc.10 crate's `dist.txt:19`:

```text
https://cdn.pyke.io/0/pyke:ort-rs/ms@1.22.0/x86_64-unknown-linux-gnu.tgz
expected SHA-256: ED1716DE95974BF47AB0223CA33734A0B5A5D09A181225D0E8ED62D070AEA893
```

This is the expected upstream build-artifact digest, not a freshly verified
download or proof that the historical release linked this exact archive. Selected
native-source inspection is not exhaustive for every build helper/embedded data
set; the remaining link-map/notice reconciliation is part of package remediation.

## 5. Actual package contents

| Release asset | SHA-256 | Payload observation |
|---|---|---|
| `Easy.Edits.Pro_0.1.2-alpha.2_amd64.deb` | `682e6c7d859cc507fc9af02f366d170f6d4cd4c34dfc5312ed8832428ce747b3` | Six regular files: app binary, desktop entry, four icons. |
| `Easy.Edits.Pro-0.1.2-alpha.2-1.x86_64.rpm` | `a5d877a3b0e8f469b7a65b7e4e4499e3aeba58f486413567bb76ee1fd6b45cb3` | Same six-file layout. |
| `easy-edits-pro-linux-x86_64.flatpak` | `1517de85ba229c356ff5cc4881228735208d1d27697a16dd0ff35a29a9f99909` | App/assets, GES libraries/plugins, headers/pkgconfig and GES licence texts. |

Deb/RPM binaries are 71,927,208 bytes each; Flatpak's is 55,409,480 bytes. No
standalone EEP/third-party notice bundle or worker appears in deb/RPM payloads.
`bundle.licenseFile` configuration is not evidence that a file was installed.
RPM/header or deb control metadata is distinct from the installed payload.

Flatpak **does** contain licence files:
`files/share/licenses/io.github.zvirox.EasyEditsPro/gst-editing-services/COPYING`
and `COPYING.LIB`. No comprehensive EEP/dependency notice bundle was observed.
Its OSTree commit is
`bd0eb6d557849e302f63c64968d7d87527ed2ffdb4b13ecdd336c04ae645c3c0`.
Metadata selects `org.gnome.Platform/x86_64/49`, with network/IPC, X11/Wayland,
all-device, home and `/run/user/1000` access. This identifies the runtime ref,
not a pinned installed runtime commit or a tested plugin permission contract.

The worker's absence in this **older release** is expected. The current tree
closes that gap: `apps/saturn-tauri/tauri.conf.json` now ships
`saturn-plugin-worker` at `/usr/bin` (the sibling target of
`crates/saturn-audio-dsp/src/sandbox.rs:490-560`) and a generated notice bundle
at `/usr/share/doc/easy-edits-pro/third-party-notices/` into deb and RPM; the
Flatpak module installs both from the deb payload (see §9). Installed
worker/plugin-hosting sandbox acceptance is still an open follow-up.

Direct ELF names include GTK/WebKit/GStreamer/GES and platform libraries, with no
OpenSSL/ONNX `DT_NEEDED` entry. Runtime-discovered codecs/plugins and static native
components are outside this direct list. Debian dependency configuration requests
good/bad/ugly/libav plugin sets; package groups are not a per-codec licence audit.
No x264 library was found as a standalone file in the application payload; this
does not prove absence from a user's system/Flatpak runtime or plugin discovery.

## 6. Remediation and gate criteria

| Owner | Work | Status / required evidence |
|---|---|---|
| Lane 4 | Resolve first-party manifest/header contradictions and UI package metadata. | **Done 2026-11-08** (maintainer decision: least-rights correction to AGPL-3.0-or-later; see §2). NOTICE/provenance records unchanged; no third-party code was relicensed. |
| Lane 4 + Lane 3 | Scope the deny policy to the approved inventory and add negative controls. | **Done 2026-11-08 for the Cargo graphs and production JS** (§2: scoped `deny.toml`, `scripts/check-licence-policy.mjs`, CI on both workspaces; controls A/B pass). Still open: Python and native/runtime licence coverage in the automated gate. |
| Lane 3 | Generate notices/licence texts from resolved build inputs, actual native archives, fonts and selected licence alternatives. | **Implemented 2026-11-09 for the Cargo+JS shipped set** (§9: `scripts/generate-notice-bundle.py` generates from both Cargo.locks and the pnpm prod closure; 526 cargo packages incl. the worker closure's extra `clack-plugin 0.2.0`, 11 JS packages, 974 verbatim texts; AND conjuncts must map to bundled texts, OR branches resolved and recorded; 11 crates without upstream files receive canonical SPDX texts from `licenses/third-party/canonical/`; explicit unresolved gaps carried everywhere and fail `--check`; machine-readable `inventory.json` ships beside the readable doc). Still open in this row: actual native archives (ONNX Runtime — carried as a declared gap; SQLite/GES build identities) and Python distribution coverage. |
| Lane 3 | Wire notices into deb/RPM/Flatpak and package `saturn-plugin-worker`. | **Done locally 2026-11-09, CI pending** (§9: `beforeBundleCommand` + `bundle.license` + deb/rpm `files` maps; Flatpak module installs worker + notices from the deb payload). Local deb/RPM extraction verified: worker at `usr/bin` mode 755 byte-identical (SHA-256 `d4bd258b…7724d94bd5f`), `copyright`/`LICENSE` installed, 521 text directories, lint suite of `installed_package_workflow.py` passes on both packages; repackaged worker smoke (no args → exit 1, `usage:` on stderr) proves the binary runs. No container install or installed-worker lookup/plugin smoke yet; remote CI (deb install proof, rpm dnf proof, flatpak proof) pending. |
| Lane 3 + Lane 4 | Reconcile native/runtime/plugin and Python distribution coverage. | Open. Runtime commits/build provenance, embedded dependency notices and isolated Python build resolution, with unknowns explicitly blocking unsupported release claims. |
| Lane 4 | Actor design and historical UI reconciliation. | **Done 2026-11-08** — [design contracts reviewed](../planning/af0-actor-design.md); execution priority/product defaults still pending. |

Do not close AF0 based on today's successful broad-policy runs or this older
release audit. Its gate requires the negative control, corrected package-bundle
evidence and reviewed design. No policy/packaging changes are made by this slice.

## 7. Reproducing the evidence

Use the named source commit and its lockfiles in a separate checkout when
reproducing the historical baseline. From that checkout, capture:

```bash
mkdir -p /tmp/opencode/af0-inventory
cargo metadata --locked --format-version 1 --filter-platform x86_64-unknown-linux-gnu > /tmp/opencode/af0-inventory/cargo-root.json
cargo metadata --locked --format-version 1 > /tmp/opencode/af0-inventory/cargo-root-all.json
cargo metadata --locked --format-version 1 --manifest-path apps/saturn-tauri/Cargo.toml --filter-platform x86_64-unknown-linux-gnu > /tmp/opencode/af0-inventory/cargo-desktop.json
cargo metadata --locked --format-version 1 --manifest-path apps/saturn-tauri/Cargo.toml > /tmp/opencode/af0-inventory/cargo-desktop-all.json
pnpm --dir apps/saturn-tauri licenses list --json > /tmp/opencode/af0-inventory/pnpm-licenses.json
pnpm --dir apps/saturn-tauri licenses list --prod --json > /tmp/opencode/af0-inventory/pnpm-prod-licenses.json
cargo deny --locked check licenses > /tmp/opencode/af0-inventory/deny-root.log 2>&1
cargo deny --manifest-path apps/saturn-tauri/Cargo.toml --locked --config deny.toml check licenses > /tmp/opencode/af0-inventory/deny-desktop.log 2>&1
gh release view v0.1.2-alpha.2 --json tagName,targetCommitish,assets,publishedAt,url > /tmp/opencode/af0-inventory/release.json
gh release download v0.1.2-alpha.2 --dir /tmp/opencode/af0-inventory/packages
```

Record installed Python metadata with `importlib.metadata.distribution` for
`cli-anything-eep`, `click`, `tabulate`, `setuptools`, `wheel`, `pytest`; retain
version, License-Expression, License, classifiers and requirements, or explicit
`unavailable: true`. A fresh environment may differ; do not replace an unavailable
entry with an inferred installed version. The captured JSON gives the exact shape.

```bash
python3 - <<'PY'
import importlib.metadata as metadata
import json
from pathlib import Path
result = {}
for name in ['cli-anything-eep', 'click', 'tabulate', 'setuptools', 'wheel', 'pytest']:
    try:
        dist = metadata.distribution(name)
        result[name] = {
            'version': dist.version,
            'license_expression': dist.metadata.get('License-Expression'),
            'license': dist.metadata.get('License'),
            'classifiers': dist.metadata.get_all('Classifier'),
            'requires': dist.requires,
        }
    except metadata.PackageNotFoundError:
        result[name] = {'unavailable': True}
Path('/tmp/opencode/af0-inventory/python-installed.json').write_text(json.dumps(result, indent=2) + '\n')
PY
```

Verify downloaded bytes against `release.json` before extracting. Extract deb's
`data.tar.*` using `ar` and `bsdtar` into `deb/`, and RPM with `bsdtar` into `rpm/`.
These are payload reads, not installation. Use a new temporary OSTree repository:

```bash
ostree --repo=/tmp/opencode/af0-inventory/flatpak-repo init --mode=archive
flatpak build-import-bundle /tmp/opencode/af0-inventory/flatpak-repo /tmp/opencode/af0-inventory/packages/easy-edits-pro-linux-x86_64.flatpak
ostree --repo=/tmp/opencode/af0-inventory/flatpak-repo checkout --user-mode bd0eb6d557849e302f63c64968d7d87527ed2ffdb4b13ecdd336c04ae645c3c0 /tmp/opencode/af0-inventory/flatpak
python3 scripts/af0-inventory.py /tmp/opencode/af0-inventory /tmp/opencode/af0-normalized
```

The report-only normalizer (Python >=3.11) verifies asset sizes/digests, graph membership, successful
licence logs and generates the tables. It needs captures from the same checkout
path for first-party classification; run the script from that checkout. It does
not run cargo-deny, install packages, choose licence alternatives or generate a
redistribution licence bundle. Compare normalized results and investigate version
drift instead of silently overwriting this dated snapshot.

## 8. Verification of this inventory/design slice

Recorded 2026-11-08:

- Nine normalized evidence files reproduced byte-for-byte in a separate output
  directory. Graph/ownership counts, package digests, payload notice/worker
  observations and path normalization were checked independently.
- Both current-policy cargo-deny runs passed, as recorded above. The proposed
  policy's negative control remains unimplemented.
- `cargo build --locked -p saturn-plugin-fixture -p saturn-plugin-worker`, then
  `cargo test --workspace` passed: 583 workspace tests plus four cold-process
  scenarios; one existing ignored test. The first attempt lacked the fixture
  shared library and failed eight chain-smoke cases before proceeding. Building
  the prerequisites prescribed by `.github/workflows/ci.yml:108-116` resolved
  that setup failure; no product/test source was changed.
- Frontend build, 174 unit tests, `check:glass`, `check:commands` (465 entries),
  `check:source-size --max-lines=800` and SPDX headers passed.
- All nine mocked E2E areas passed with zero browser errors on the isolated
  rerun. The first attempt timed out at a geometry menu click while Rust was
  compiling; it did not reproduce without the concurrent build. This observation
  does not establish the timeout's root cause or certify native editor behavior.
- Local document links/anchors, new-file character/whitespace sweep and
  `git diff --check` passed. Product source, lockfiles and generated catalogue
  files were not changed by this slice.

Local verification logs are retained in `/tmp/opencode/af0-inventory/`:
`workspace-tests.log` (initial failure), `workspace-rerun.log` (prerequisite build
and pass), `ui-gates.log` (five passes and initial E2E timeout), and
`e2e-rerun.log` (pass). These checks validate the current repository baseline;
they do not close the policy/package-remediation or future actor-implementation
gates described above.

**Licence-policy slice (2026-11-08, later the same day):** the scoped-policy
change (manifest correction, `deny.toml` scoping, `scripts/check-licence-policy.mjs`,
CI wiring) was verified with `node scripts/check-licence-policy.mjs`: JS gate
(11 production packages, allow-listed), cargo-deny on both workspaces under the
scoped policy, first-party AGPL assertions (14 root / 6 desktop, all manifests
inside the repository) and both negative controls. All evidence captures were
regenerated after the correction and the normalized tables reproduce
byte-for-byte in a separate output directory; graph counts are unchanged
(710 rows, 386/525/446/661). Unlicensed Cargo packages continue to fail the
policy by construction (no wildcard allow, no clarify entries), though that
direction has no dedicated scratch control. The first run of negative control B
failed for a script bug (the lookalike assertion ran after scratch cleanup); the
fix moved the assertion inside the scratch lifecycle, and both controls pass.
CI cannot be executed locally; the workflow wiring follows the action's
documented inputs and mirrors the existing packages-workflow pnpm setup.

## 9. Notice-bundle and worker-packaging slice (2026-11-09)

This slice implements the two open Lane 3 remediation rows in §6 without
dropping any declared gap from §4 or the older payload observations in §5.

New/generated material (uncommitted at time of writing):

| Item | SHA-256 (prefix) | Note |
|---|---|---|
| `scripts/generate-notice-bundle.py` | `e2ca4769…e604b8130cd` | Generator + CI gate; `--check` green |
| `scripts/prepare-bundle-inputs.sh` | `8f72170e…91c5bf686d` | Tauri `beforeBundleCommand` hook |
| `licenses/third-party/canonical/` (17 texts) | — | Canonical SPDX identifier texts from spdx.org/licenses (provenance in `NOTICE`) |
| `target/release/saturn-plugin-worker` | `d4bd258b…4d94bd5f` | Release-built worker; byte-identical inside both packages |
| local deb `Easy Edits Pro_0.1.0_amd64.deb` | `d2df2b32…151a7da5` | Extracted + verified below |
| local rpm `Easy Edits Pro-0.1.0-1.x86_64.rpm` | `1638172e…b52ffb44` | Extracted + verified below |
| shipped `inventory.json` (deb) | `7b237e19…dc259a60` | 526 cargo / 11 JS entries + unresolved gaps |
| shipped `NOTICE-DEPENDENCIES.md` (deb) | `4dbcc1b0…61e96d2d8` | Readable doc, same content as inventory flags |

Wiring (`apps/saturn-tauri/tauri.conf.json`):

- `beforeBundleCommand` → `scripts/prepare-bundle-inputs.sh`: builds
  `saturn-plugin-worker` (root workspace, `--locked --release`) and generates
  the bundle into `target/third-party-notices`, refusing thin bundles.
- `bundle.license: AGPL-3.0-or-later` + `licenseFile` (deb installs it as
  `usr/share/doc/easy-edits-pro/copyright`; rpm as `…/easy-edits-pro/LICENSE`).
- deb/rpm `files` maps ship the worker at `/usr/bin/saturn-plugin-worker` and
  the notice directory at `/usr/share/doc/easy-edits-pro/third-party-notices/`
  (directory map entry; `tauri-bundler` `copy_custom_files` recurses).
- `packaging/flatpak/build-module.sh` installs the worker beside
  `saturn-tauri` in `/app/bin/` and copies `usr/share/doc/easy-edits-pro`
  from the deb payload (the payload comes via `make-payload.sh`), failing
  loudly when the payload lacks either.
- CI: the `licences` job runs `generate-notice-bundle.py --check`; the
  containers in the deb/rpm install proofs assert the worker, the installed
  notice bundle and the usage smoke inside real installs; the deb/rpm lints
  assert the same facts at package level.

Verification performed locally (same host as §8):

- `pnpm tauri build` completed end to end: beforeBundleCommand ran
  (`generate-notice-bundle: cargo 526 (520 third-party), js 11, texts 974`;
  `521 text directories` staged) and produced the deb and rpm above
  (version 0.1.0; the tree is untagged, so the tag-version step in CI would
  render a different filename there).
- `cargo test --workspace`: 587 passed, 0 failed (65 suites reporting ok;
  one existing ignored cold-process test unchanged).
- All nine mocked E2E areas passed on the isolated rerun with zero browser
  errors (`tests/e2e`, `pnpm test:e2e`).
- Deb extraction (`ar p … data.tar.gz | tar -xz`): `usr/bin/saturn-tauri` +
  `usr/bin/saturn-plugin-worker` (both mode 755; worker byte-identical to the
  release build by SHA-256), `usr/share/doc/easy-edits-pro/copyright` (AGPL
  text, mode 644), `third-party-notices/{NOTICE-DEPENDENCIES.md,
  inventory.json}` and 521 `texts/` directories.
- RPM extraction (`bsdtar`): same worker + notices + `LICENSE` + 521 text
  directories; desktop entry and icons unchanged; the two warnings from the
  older audit remain exactly the same two known ones (desktop filename
  space; no appstream metainfo) — no new warnings.
- `tests/native/installed_package_workflow.py` `deb_lint` and `rpm_lint`
  (their updated forms, run in-process on the local artifacts) both pass:
  521 text dirs, 526 cargo packages declared, worker present + executable.
- Worker executable smoke on the extracted deb payload:
  `saturn-plugin-worker` with no arguments exits 1 and prints
  `usage: saturn-plugin-worker <control-socket-path> <shm-path>` on stderr —
  the binary runs outside the cargo tree.

Not proven here (honest limits, unchanged responsibilities):

- No real dpkg/dnf container install and no installed-worker lookup or
  plugin-hosting smoke in a packaged layout (the container commands were
  extended above; they must run remotely — this host has no container
  daemon and a non-deb host cannot dnf-install).
- Flatpak install-proof is still blocked by the earlier isolated-session
  hang (preserved report); the module script's new installs are
  payload-verified only by the next CI Flatpak proof, not by a local build.
- Cargo licences describe compiled-in crates only; ONNX Runtime native
  notices and GES/SQLite build identities stay explicitly unresolved and
  are carried in the bundle's own `unresolved_gaps` list (both rendered
  into `NOTICE-DEPENDENCIES.md` and serialized into `inventory.json`);
  nothing in this slice closes them.
- Remote CI has not yet executed the new rules.
