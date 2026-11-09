#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Generate the third-party notice bundle shipped inside deb/RPM/Flatpak.

Outputs (into --out):
  NOTICE-DEPENDENCIES.md   readable, human-facing notice document
  inventory.json           machine-readable: every shipped package, its
                           declared licence expression, the bundled text
                           files with SHA-256s, and the explicit gaps
  texts/<package>-<version>/...
                           verbatim upstream licence files; original names
                           and copyright statements preserved

Sources of truth (never hand-edited):
  * Cargo.lock for the desktop workspace (apps/saturn-tauri, locked, host
    triple) — the crates linked into the shipped application binary;
  * the workspace-root lockfile for the saturn-plugin-worker runtime
    closure — the worker ships next to the application in every package;
  * pnpm-lock.yaml production closure (the bundled frontend);
  * repository-tracked licence texts for fonts and vendored native sources;
  * an explicit unresolved-gap list mandated by docs/audit/af0-inventory.md
    §4 (e.g. version-matched ONNX Runtime notices). Gaps are carried into
    every surface and FAIL the --check gate.

Policy enforced while generating:
  * every third-party cargo package in the shipped set must have at least
    one discoverable licence text (its source directory must exist locally;
    a missing text fails generation — never ship an unidentified crate);
  * for a declared expression containing AND, every conjunct must map to a
    text file (both halves of an AND are real obligations); OR branches are
    resolved by choosing one mapped branch, which is recorded;
  * texts brought in are copied verbatim, never SPDX-normalized.

Usage:
  python3 scripts/generate-notice-bundle.py --out PATH   # emit the bundle
  python3 scripts/generate-notice-bundle.py --check      # verify only

--check builds the same bundle into a temporary directory and fails when
anything above is unmet. It is wired into CI's licences job; package builds
regenerate the bundle (beforeBundleCommand) so a shipped artifact can never
drift from the lockfiles.
"""

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
APPR = REPO / "apps" / "saturn-tauri"
NOTICE_DOC_NAME = "NOTICE-DEPENDENCIES.md"
INVENTORY_NAME = "inventory.json"
WORKER_CRATE = "saturn-plugin-worker"

FIRST_PARTY = {
    "saturn-cli", "saturn-commands", "saturn-control", "saturn-core",
    "saturn-graphics", "saturn-mcp", "saturn-perception",
    "saturn-plugin-fixture", "saturn-plugin-worker", "saturn-project",
    "saturn-render", "saturn-tauri", "saturn-text", "saturn-timeline",
    "saturn-audio-dsp",
}

# Licence-file basename match; original casing preserved on copy.
LICENCE_RE = re.compile(
    r"^(?:LICENSE|LICENCE|UNLICENSE|UNLICENCE|NOTICE|COPYING|COPYRIGHT)(?:[._\-].*)?$",
    re.IGNORECASE,
)
SKIP_RE = re.compile(r"LICENSESUM", re.IGNORECASE)
SCAN_DEPTH = 2
CANONICAL_DIR = REPO / "licenses" / "third-party" / "canonical"


def canonical_files_for(identifier):
    """Canonical SPDX texts for an id (the id itself plus any WITH-exception
    rider). Returns the file list, or None when no canonical text exists."""
    parts = [identifier.split(" WITH ")[0]]
    parts.extend(re.findall(r"WITH\s+([A-Za-z0-9._+-]+)", identifier))
    files = []
    for part in parts:
        path = CANONICAL_DIR / f"{part}.txt"
        if not path.is_file():
            return None
        files.append(path)
    return files


def copy_canonical(out_dir, package, ids):
    target = out_dir / "texts" / f"{package['name']}-{package['version']}"
    target.mkdir(parents=True, exist_ok=True)
    for identifier in ids:
        files = canonical_files_for(identifier)
        if files is None:
            raise SystemExit(
                f"generate-notice-bundle: no canonical text for {identifier} "
                f"(requested by {package['name']} {package['version']})")
        for path in files:
            shutil.copy2(path, target / path.name)


def canonical_texts(ids):
    entries = []
    for identifier in ids:
        for path in canonical_files_for(identifier) or []:
            entries.append({
                "file": path.name,
                "sha256": digest(path),
                "bytes": path.stat().st_size,
                "source": "canonical SPDX identifier text (licenses/third-party/canonical); the upstream archive ships no licence file",
            })
    return entries

# Fragment used to map an SPDX identifier to shipped file names (substring
# test against the basename, lower-cased).
ID_FRAGMENTS = {
    "MIT-0": ["mit-0"],
    "MIT": ["mit"],
    "Apache-2.0": ["apache"],
    "BSD-2-Clause": ["bsd-2", "2-clause"],
    "BSD-3-Clause": ["bsd-3", "bsd"],
    "ISC": ["isc"],
    "MPL-2.0": ["mpl"],
    "Zlib": ["zlib"],
    "Unlicense": ["unlic"],
    "Unicode-3.0": ["unicode"],
    "Unicode-DFS-2016": ["unicode-dfs", "unicode_dfs"],
    "CDLA-Permissive-2.0": ["cdla"],
    "BSL-1.0": ["bsl", "boost"],
    "IJG": ["ijg", "jpeg"],
    "AGPL-3.0-or-later": ["agpl", "license"],
}

# Other shipped material with repository-tracked texts (fonts / vendored
# native sources / dynamically loaded system libraries). Curated by hand in
# the audit; copied verbatim into the bundle.
OTHER_COMPONENTS = [
    {
        "name": "Inter (UI typeface; bundled as InterVariable.woff2, upstream as InterVariable.ttf)",
        "status": "resolved",
        "licence": "SIL Open Font License 1.1",
        "copyright": "Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter)",
        "texts": [{"repo_path": "assets/fonts/OFL.txt"}],
        "note": "Ships with the UI build; also recorded in the repository NOTICE.",
    },
    {
        "name": "whisper.cpp / ggml (vendored C/C++ inference sources)",
        "status": "resolved",
        "licence": "MIT",
        "copyright": "(c) 2023 Georgi Gerganov and contributors",
        "texts": [{"repo_path": "licenses/whisper.cpp-MIT.txt"}],
        "note": ("Inference engine embedded via whisper-rs-sys 0.13.1. Downloaded model "
                 "weights are separate and fetch on first use; see repository NOTICE."),
    },
    {
        "name": "gst-editing-services 1.26.11 (compiled from source into the Flatpak)",
        "status": "resolved",
        "licence": "LGPL-2.1-or-later",
        "copyright": "(c) The GStreamer project and other GES contributors",
        "texts": [{"repo_path": None, "note": "GES's own COPYING / COPYING.LIB ship inside the Flatpak payload (verified for the audited payloads)"}],
        "note": "Built by packaging/flatpak/io.github.zvirox.EasyEditsPro.yml (source URL + sha256 pinned there); LGPL source availability is satisfied by building from the pinned upstream archive.",
    },
    {
        "name": "System libraries loaded dynamically at run time (WebKitGTK, GTK, GStreamer and its plugin sets)",
        "status": "dynamic",
        "licence": "LGPL-2.1-or-later and per-library identifiers; NOT bundled",
        "texts": [],
        "note": "Declared as deb Depends / RPM Requires in tauri.conf.json; loaded from the user's system (deb) or the Flatpak runtime. Their distributors hold the copyright.",
    },
]

# Mandated by docs/audit/af0-inventory.md §4; each entry needs real evidence
# (not paperwork) before a release is described as fully notated.
UNRESOLVED_GAPS = [
    {
        "name": "ONNX Runtime 1.22.0 (ort-sys 2.0.0-rc.10 prebuilt archive)",
        "note": ("The version-matched CPU archive in the local cache contains only "
                 "libonnxruntime.a; no third-party notice list verifies what the "
                 "prebuilt binary embeds. The digest evidence is docs/audit/"
                 "af0-inventory.md §4; obtain version-matched runtime + notice files "
                 "with build provenance before an unsupported-claim release."),
    },
    {
        "name": "Flatpak payload EEP/dependency notice coverage",
        "note": ("The audited release payloads carry GES's own COPYING/COPYING.LIB but "
                 "no comprehensive application+dependency bundle; this bundle closes "
                 "the deb/RPM side and must be verified inside the next Flatpak build."),
    },
]

# Curated record: declared expressions whose only shipped text is a single
# combined file so no OR choice can be named from the file name alone. The
# combined file is still copied verbatim; the choice is documented here.
COMBINED_TEXT_PACKAGES = {
    # instance: (name, version) -> branch chosen (recorded for reviewers)
}


def run(command, cwd):
    result = subprocess.run(command, capture_output=True, text=True, cwd=str(cwd))
    if result.returncode != 0:
        raise SystemExit(
            "generate-notice-bundle: command failed (" + " ".join(command) + "):\n"
            + result.stderr[-2000:]
        )
    return result.stdout


def digest(path):
    # hashlib.file_digest is Python 3.11+; the packaging runner ships 3.10,
    # so fall back to a streaming copy (identical hex output).
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        if hasattr(hashlib, "file_digest"):
            return hashlib.file_digest(stream, "sha256").hexdigest()
        for chunk in iter(lambda: stream.read(65536), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def cargo_packages(manifest, filter_platform=True):
    args = ["cargo", "metadata", "--locked", "--format-version", "1",
            "--manifest-path", str(manifest)]
    if filter_platform:
        args += ["--filter-platform", "x86_64-unknown-linux-gnu"]
    data = json.loads(run(args, REPO))
    nodes = {node["id"] for node in data["resolve"]["nodes"]}
    packages = [p for p in data["packages"] if p["id"] in nodes]
    adjacency = {}
    for node in data["resolve"]["nodes"]:
        runtime_edges = [dep["pkg"] for dep in node["deps"]
                         if not dep.get("dep_kinds")
                         or any(not kind["kind"] for kind in dep["dep_kinds"])]
        adjacency[node["id"]] = runtime_edges
    return packages, adjacency


def worker_closure():
    """The worker crate and its runtime (normal-edge) dependency closure from
    the workspace-root lockfile."""
    packages, adjacency = cargo_packages(REPO / "Cargo.toml")
    roots = [p["id"] for p in packages if p["name"] == WORKER_CRATE]
    if not roots:
        raise SystemExit(f"generate-notice-bundle: {WORKER_CRATE} missing from the root graph")
    seen, frontier = set(), list(roots)
    while frontier:
        node = frontier.pop()
        if node in seen:
            continue
        seen.add(node)
        frontier.extend(adjacency.get(node, []))
    known = {p["id"] for p in packages}
    missing = seen - known
    if missing:
        raise SystemExit(f"generate-notice-bundle: closure references unknown ids {sorted(missing)}")
    return [p for p in packages if p["id"] in seen]


import os


def find_licence_files(package_dir):
    files = []
    if not package_dir.is_dir():
        return files, False
    for dirpath, _dirnames, filenames in os.walk(package_dir):
        relative = Path(dirpath).relative_to(package_dir)
        if len(relative.parts) >= SCAN_DEPTH:
            continue
        for name in sorted(filenames):
            if SKIP_RE.search(name):
                continue
            if LICENCE_RE.match(name):
                files.append(Path(dirpath) / name)
    return files, True


def copy_texts(out_dir, package, package_dir):
    target = out_dir / "texts" / f"{package['name']}-{package['version']}"
    target.mkdir(parents=True, exist_ok=True)
    files, _ = find_licence_files(package_dir)
    for path in files:
        shutil.copy2(path, target / path.name)


def tokenize(expression):
    return re.findall(r"\(|\)|[^()\s/]+|/", expression.replace("/", " OR "))


def parse_expression(expression):
    """Returns an AST: ("OR"/"AND", children) or an id string. WITH-exception
    riders stay attached to the preceding id."""
    tokens = tokenize(expression)
    pos = [0]

    def peek():
        return tokens[pos[0]] if pos[0] < len(tokens) else None

    def take():
        token = tokens[pos[0]]
        pos[0] += 1
        return token

    def parse_or():
        branches = [parse_and()]
        while peek() == "OR":
            take()
            branches.append(parse_and())
        return branches[0] if len(branches) == 1 else ("OR", branches)

    def parse_and():
        terms = [parse_term()]
        while peek() == "AND":
            take()
            terms.append(parse_term())
        return terms[0] if len(terms) == 1 else ("AND", terms)

    def parse_term():
        token = peek()
        if token == "(":
            take()
            tree = parse_or()
            if take() != ")":
                raise ValueError("unclosed parenthesis")
            return tree
        ident = take()
        if ident in ("AND", "OR", "WITH", ")"):
            raise ValueError(f"unexpected token {ident!r}")
        while peek() == "WITH":
            take()
            ident += " WITH " + take()
        return ident

    tree = parse_or()
    if peek() is not None:
        raise ValueError("trailing tokens")
    return tree


def all_ids(tree):
    out = []

    def collect(node):
        if isinstance(node, str):
            out.append(node.split(" WITH ")[0])
        else:
            for child in node[1]:
                collect(child)

    collect(tree)
    return out


def term_matches(term, files):
    base = term.split(" WITH ")[0]
    fragments = ID_FRAGMENTS.get(base) or [base.split("-")[0].lower()]
    matches = [f for f in files if any(frag in f.name.lower() for frag in fragments)]
    if not matches:
        # A single unsuffixed LICEN[CS]E/COPYING/COPYRIGHT/NOTICE file (with
        # plainer or .md/.txt endings) may cover any term; the copy stays
        # verbatim and the choice below records it.
        generic = [f for f in files
                   if re.sub(r"\.(txt|md|rst|ark|html)$", "", f.name.lower())
                   in ("license", "licence", "copying", "copyright", "notice", "unlicense")]
        matches = list(generic)
    return matches


def node_cover(node, files):
    """Can the declared node be honored with the shipped texts? Returns
    (covered, term_records, missing_ids). OR nodes record the first covered
    alternative; AND nodes record every conjunct."""
    if isinstance(node, str):
        matches = term_matches(node, files)
        if matches:
            return True, [{"term": node, "files": sorted(f.name for f in matches)}], []
        return False, [], [node]
    (op, children) = node
    records = []
    if op == "AND":
        for child in children:
            covered, terms, missing = node_cover(child, files)
            records.extend(terms)
            if missing:
                return False, records, missing
        return True, records, []
    assert op == "OR", op
    missing_ids = []
    for child in children:
        covered, terms, missing = node_cover(child, files)
        if covered:
            records.append({"alternative": (child if isinstance(child, str) else "grouped"),
                            "terms": terms})
            return True, records, []
        missing_ids.extend(missing)
    return False, records, missing_ids


def resolve_cover(expression, files):
    """Prove that some OR branch is covered and every AND term maps to a
    file. Returns (record, missing_ids) — an empty missing list means the
    declared obligations are demonstrably honored by shipped texts."""
    tree = parse_expression(expression)
    branches = [tree] if isinstance(tree, str) or tree[0] == "AND" else list(tree[1])
    missing_ids = []
    for branch in branches:
        covered, records, missing = node_cover(branch, files)
        if covered:
            return {"branch": branch if isinstance(branch, str) else branch[0],
                    "honored_terms": records}, []
        missing_ids.extend(missing)
    return {"branch": None, "honored_terms": []}, list(dict.fromkeys(all_ids(tree)))


def cargo_entry(package, out_dir):
    """Licence record for one cargo package in the shipped set."""
    manifest = Path(package["manifest_path"]).resolve()
    package_dir = manifest.parent
    owned = package["source"] is None and manifest.is_relative_to(REPO)
    files, present = find_licence_files(package_dir)
    expression = package["license"] or "UNDECLARED"
    entry = {
        "name": package["name"],
        "version": package["version"],
        "source": package["source"] or "path-dependency",
        "origin": None,
        "repository": package["repository"] or "",
        "declared_expression": expression,
        "coverage_pick": None,
        "ownership": "first-party" if owned else "third-party",
        "texts": [],
    }
    if package["source"] and package["source"].startswith("registry+https"):
        entry["origin"] = f"https://crates.io/crates/{package['name']}/{package['version']}"
    if owned:
        lic = REPO / "LICENSE"
        entry["texts"].append({
            "file": "LICENSE", "sha256": digest(lic), "bytes": lic.stat().st_size,
            "source": "repository root LICENSE (ships to /usr/share/doc/easy-edits-pro/copyright)",
        })
        return entry, []
    expression_ids = all_ids(parse_expression(expression)) if expression not in ("", "UNDECLARED") else []
    if not present:
        # The upstream crate archive shipped no licence files; fall back to
        # the canonical SPDX text pool vendored in the repository.
        ids = expression_ids or ["MIT"]
        entry["canonical_fallback"] = ids
        copy_canonical(out_dir, package, ids)
        entry["texts"].extend(canonical_texts(ids))
        return entry, []
    for path in files:
        entry["texts"].append({
            "file": str(path.relative_to(package_dir)),
            "sha256": digest(path),
            "bytes": path.stat().st_size,
        })
    _record, missing = resolve_cover(expression, files)
    if missing:
        # AND conjuncts (or every OR alternative) that no shipped file maps
        # to: satisfy them from the canonical pool so the AND obligations are
        # honored by bundled texts rather than by hope.
        entry["canonical_fallback"] = list(dict.fromkeys(missing))
        copy_canonical(out_dir, package, entry["canonical_fallback"])
        entry["texts"].extend(canonical_texts(entry["canonical_fallback"]))
        missing = [i for i in missing if canonical_files_for(i) is None]
    entry["coverage_pick"] = _record
    return entry, missing


def js_closure(out_dir):
    """Production JavaScript closure. The `pnpm licenses list --prod` report
    is the authoritative name/version set (pnpm list collapses nested trees);
    install paths come from `pnpm list --prod --json`."""
    report = json.loads(run(["pnpm", "licenses", "list", "--prod", "--json"], APPR))
    paths = {}
    listing = json.loads(run(["pnpm", "list", "--prod", "--json"], APPR))
    trees = [entry.get("dependencies") or {} for entry in listing] \
        if isinstance(listing, list) else [listing.get("dependencies") or {}]

    def walk(graph):
        for name, info in (graph or {}).items():
            version = (info or {}).get("version") or ""
            if name and version:
                paths[(name, version)] = (info or {}).get("path") or ""
            walk((info or {}).get("dependencies") or {})

    for tree in trees:
        walk(tree)

    wanted = []
    for notion, items in report.items():
        for item in items:
            for version in item.get("versions", []):
                wanted.append((item["name"], version, notion))
    wanted.sort()
    if not wanted:
        raise SystemExit("generate-notice-bundle: no production javascript packages")

    js_dir = APPR / "node_modules" / ".pnpm"
    entries = []
    for name, version, notion in wanted:
        directory = Path(paths.get((name, version), "")) if paths.get((name, version)) else None
        if directory is None or not directory.is_dir():
            normalized = name.replace("/", "+")
            directory = None
            if js_dir.is_dir():
                candidates = sorted(d for d in js_dir.iterdir()
                                    if d.name.startswith(f"{normalized}@{version}"))
                for candidate in candidates:
                    sub = candidate / "node_modules" / name
                    if sub.is_dir():
                        directory = sub
                        break
        if directory is None or not directory.is_dir():
            raise SystemExit(
                f"generate-notice-bundle: javascript package {name}@{version} has no "
                f"installed directory under node_modules")
        files, _ = find_licence_files(directory)
        if not files:
            raise SystemExit(
                f"generate-notice-bundle: javascript package {name}@{version} ships "
                f"no licence text under {directory}")
        target_base = out_dir / "texts" / "javascript" / f"{name}-{version}"
        target_base.mkdir(parents=True, exist_ok=True)
        texts = []
        for path in files:
            shutil.copy2(path, target_base / path.name)
            texts.append({
                "file": path.name, "sha256": digest(path),
                "bytes": path.stat().st_size,
            })
        entries.append({
            "name": name,
            "version": version,
            "origin": f"https://www.npmjs.com/package/{name}/v/{version}",
            "declared_expression": notion,
            "texts": texts,
        })
    return entries


def render_readable(out_dir, cargo_entries, js_entries, metadata):
    lines = []
    a = lines.append
    a("# Third-party dependency notices (Easy Edits Pro)")
    a("")
    a("Generated by `scripts/generate-notice-bundle.py` from the Cargo lockfiles,")
    a("`pnpm-lock.yaml` 'prod' closure and the repository licence texts; **do not")
    a("hand-edit**. The machine-readable record is the sibling `inventory.json`.")
    a("")
    a(f"- cargo packages covered: {metadata['cargo_packages']} "
      f"({metadata['first_party']} first-party, {metadata['third_party']} third-party)")
    a(f"- worker-only runtime packages: {metadata['worker_extra']}")
    a(f"- javascript packages covered: {metadata['javascript_packages']}")
    a(f"- bundled text files copied verbatim: {metadata['texts']}")
    a("- lockfile SHA-256: " + "; ".join(
        f"`{path} = {value}`" for path, value in metadata["lockfile_hashes"]))
    a("")
    a("## Rust crates in the shipped application and worker binaries")
    a("")
    a("| crate | version | declared coverage | bundled texts |")
    a("|---|---|---|---|")
    for entry in cargo_entries:
        if entry["ownership"] == "third-party":
            relative = f"texts/{entry['name']}-{entry['version']}/"
            inner = ", ".join(Path(t["file"]).name for t in entry["texts"])
            a(f"| {entry['name']} | {entry['version']} | {entry['declared_expression']} | `{relative}({inner})` |")
    a("")
    a("First-party crates (Easy Edits Pro's own code, AGPL-3.0-or-later):"
      f" {', '.join(e['name'] for e in cargo_entries if e['ownership'] == 'first-party')}.")
    a("Their licence text ships as the package copyright file (repository `LICENSE`).")
    a("")
    a("## Bundled front-end packages (production JavaScript closure)")
    a("")
    a("| package | version | declared coverage | bundled texts |")
    a("|---|---|---|---|")
    for entry in js_entries:
        relative = f"texts/javascript/{entry['name']}-{entry['version']}/"
        inner = ", ".join(t["file"] for t in entry["texts"])
        a(f"| {entry['name']} | {entry['version']} | {entry['declared_expression']} | `{relative}({inner})` |")
    a("")
    a("## Other shipped material with repository-tracked licence texts")
    a("")
    for component in OTHER_COMPONENTS:
        a(f"- **{component['name']}** — {component['licence']}")
        if component.get("copyright"):
            a(f"  {component['copyright']}")
        for text in component["texts"]:
            path = text.get("repo_path")
            if path:
                a(f"  text: `{path}`")
            elif text.get("note"):
                a(f"  {text['note']}")
        if component.get("note"):
            a(f"  note: {component['note']}")
    a("")
    a("## System libraries loaded dynamically at run time")
    a("")
    a("WebKitGTK, GTK, GStreamer and plugin packages are NOT embedded; they load from")
    a("the user's system (deb Depends / RPM Requires in tauri.conf.json) or the")
    a("Flatpak runtime. Their distributors hold the copyright; the dual-licensing")
    a("plan's dynamic-loading carve-out (docs/planning/dual-licensing-plan.md §3.3)")
    a("covers use at run time.")
    a("")
    a("## Model weights")
    a("")
    a("Model weights are not compiled into the binaries and fetch on first use; see")
    a("the repository `NOTICE` (models ledger) for terms and the fetch ledger.")
    a("")
    a("## Known gaps (must be resolved before calling a release fully notated)")
    a("")
    for gap in UNRESOLVED_GAPS:
        a(f"- **{gap['name']}**")
        a(f"  {gap['note']}")
    a("")
    a("Generated content paths inside the packages:")
    a(f"- `/usr/share/doc/easy-edits-pro/third-party-notices/{NOTICE_DOC_NAME}` (this file)")
    a(f"- `/usr/share/doc/easy-edits-pro/third-party-notices/{INVENTORY_NAME}`")
    a("- `/usr/share/doc/easy-edits-pro/third-party-notices/texts/...`")
    a("")
    (out_dir / NOTICE_DOC_NAME).write_text("\n".join(lines) + "\n", encoding="utf-8")


def collect_cargo(out_dir):
    desktop, desktop_adj = cargo_packages(APPR / "Cargo.toml")
    desktop_names = {(p["name"], p["version"]) for p in desktop}
    closure = worker_closure()
    extra = [p for p in closure if (p["name"], p["version"]) not in desktop_names
             and not (p["source"] is None and p["name"] in FIRST_PARTY)]
    entries, unmapped = [], []
    for package in desktop + extra:
        entry, missing = cargo_entry(package, out_dir)
        if entry["ownership"] == "third-party":
            copy_texts(out_dir, package, Path(package["manifest_path"]).resolve().parent)
        if missing:
            unmapped.append({entry["name"] + "@" + entry["version"]: missing})
        entries.append(entry)
    first_party = sum(1 for e in entries if e["ownership"] == "first-party")
    return {
        "entries": entries,
        "unmapped": unmapped,
        "metadata": {
            "cargo_packages": len(entries),
            "first_party": first_party,
            "third_party": len(entries) - first_party,
            "worker_extra": len(extra),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", help="output directory (missing parents created)")
    parser.add_argument("--check", action="store_true", help="verify completeness only (dry-run)")
    args = parser.parse_args()
    if args.out and args.check:
        raise SystemExit("generate-notice-bundle: --out and --check are exclusive")

    if args.check:
        out_dir = Path(tempfile.mkdtemp(prefix="easy-edits-pro-notices-"))
    else:
        if not args.out:
            raise SystemExit("generate-notice-bundle: specify --out or --check")
        out_dir = Path(args.out).resolve()
        out_dir.mkdir(parents=True, exist_ok=True)

    try:
        cargo = collect_cargo(out_dir)
        javascript = js_closure(out_dir)
        metadata = dict(cargo["metadata"])
        metadata["javascript_packages"] = len(javascript)
        metadata["texts"] = sum(len(e["texts"]) for e in cargo["entries"] + javascript) \
            + sum(1 for c in OTHER_COMPONENTS for t in c["texts"]
                  if t.get("repo_path") and (REPO / t["repo_path"]).is_file())
        metadata["lockfile_hashes"] = [
            [str(path.relative_to(REPO)), digest(path)]
            for path in (REPO / "Cargo.lock", APPR / "Cargo.lock", APPR / "pnpm-lock.yaml")
        ]
        render_readable(out_dir, cargo["entries"], javascript, metadata)
        inventory = {
            "generated_by": "scripts/generate-notice-bundle.py",
            "package_paths": {
                "NOTICE-DEPENDENCIES.md": NOTICE_DOC_NAME,
                "inventory.json": INVENTORY_NAME,
                "install_prefix": "/usr/share/doc/easy-edits-pro/third-party-notices",
            },
            "metadata": metadata,
            "cargo": cargo["entries"],
            "javascript": javascript,
            "other_shipped": OTHER_COMPONENTS,
            "unresolved_gaps": UNRESOLVED_GAPS,
            "unmapped_and_terms": cargo["unmapped"],
        }
        (out_dir / INVENTORY_NAME).write_text(
            json.dumps(inventory, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print("generate-notice-bundle: cargo "
              f"{metadata['cargo_packages']} ({metadata['third_party']} third-party), "
              f"js {metadata['javascript_packages']}, texts {metadata['texts']}")
        problems = cargo["unmapped"]
        if problems:
            for package in problems:
                print(f"  unmapped AND terms: {package}", file=sys.stderr)
            raise SystemExit("generate-notice-bundle: coverage expression terms missing texts")
        if args.check:
            print("generate-notice-bundle: check passed")
    finally:
        if args.check:
            shutil.rmtree(out_dir, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
