#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Normalize AF0 captures into review evidence; does not approve licences.

Usage: python3 scripts/af0-inventory.py CAPTURE_DIR OUTPUT_DIR
Capture commands and limitations: docs/audit/af0-inventory.md.
"""

import csv
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_csv(output, name, fields, rows):
    with (output / name).open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def normalize(capture, output):
    output.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parents[1]
    packages = {}
    scopes = ("root", "desktop", "root-all", "desktop-all")
    for scope in scopes:
        data = json.loads((capture / f"cargo-{scope}.json").read_text())
        nodes = {node["id"]: node for node in data["resolve"]["nodes"]}
        for package in data["packages"]:
            if package["id"] not in nodes:
                continue
            source = package["source"]
            owned = False
            if source is None:
                manifest = Path(package["manifest_path"]).resolve()
                owned = manifest.is_relative_to(repo)
                source = str(manifest.relative_to(repo)) if owned else "external-path"
            key = (package["name"], package["version"], source)
            row = packages.setdefault(key, {
                "name": package["name"], "version": package["version"],
                "source": source, "ownership": "first-party" if owned else "third-party",
                "declared_license": package["license"] or "UNDECLARED",
                "repository": package["repository"] or "", "native_links": package["links"] or "",
                **{s: "" for s in scopes}, **{s + "_features": "" for s in scopes},
            })
            assert row["declared_license"] == (package["license"] or "UNDECLARED")
            row[scope] = "yes"
            row[scope + "_features"] = ";".join(sorted(nodes[package["id"]]["features"]))
    rows = [packages[key] for key in sorted(packages)]
    write_csv(output, "cargo.csv", list(rows[0]), rows)

    js = {}
    for filename, production in (("pnpm-licenses.json", False), ("pnpm-prod-licenses.json", True)):
        for license_id, entries in json.loads((capture / filename).read_text()).items():
            for entry in entries:
                for version in entry["versions"]:
                    row = js.setdefault((entry["name"], version), {
                        "name": entry["name"], "version": version,
                        "declared_license": license_id, "production_graph": "no",
                    })
                    if production:
                        row["production_graph"] = "yes"
    write_csv(output, "javascript.csv", ["name", "version", "declared_license", "production_graph"],
              [js[key] for key in sorted(js)])
    python_data = json.loads((capture / "python-installed.json").read_text())
    (output / "python-installed.json").write_text(json.dumps(python_data, indent=2) + "\n")

    release = json.loads((capture / "release.json").read_text())
    assets = []
    for asset in release["assets"]:
        path = capture / "packages" / asset["name"]
        sha256 = digest(path)
        assert "sha256:" + sha256 == asset["digest"], asset["name"]
        assert path.stat().st_size == asset["size"], asset["name"]
        assets.append({"name": asset["name"], "bytes": asset["size"], "sha256": sha256,
                       "url": asset["url"], "release": release["tagName"]})
    write_csv(output, "release-assets.csv", list(assets[0]), sorted(assets, key=lambda a: a["name"]))

    payload = []
    elf = []
    for kind in ("deb", "rpm", "flatpak"):
        root = capture / kind
        assert root.is_dir(), root
        for path in sorted(root.rglob("*")):
            if path.is_symlink():
                payload.append({"package": kind, "path": str(path.relative_to(root)),
                                "kind": "symlink", "bytes": "", "sha256": "", "target": str(path.readlink())})
            elif path.is_file():
                payload.append({"package": kind, "path": str(path.relative_to(root)),
                                "kind": "file", "bytes": path.stat().st_size, "sha256": digest(path), "target": ""})
        binary = root / ("files/bin/saturn-tauri" if kind == "flatpak" else "usr/bin/saturn-tauri")
        dynamic = subprocess.check_output(["readelf", "-d", str(binary)], text=True)
        for library in re.findall(r"\(NEEDED\).*?\[(.*?)\]", dynamic):
            elf.append({"package": kind, "needed": library})
    write_csv(output, "package-files.csv", ["package", "path", "kind", "bytes", "sha256", "target"], payload)
    write_csv(output, "elf-needed.csv", ["package", "needed"], elf)

    captured = []
    for path in sorted(capture.glob("*.json")):
        captured.append({"capture": path.name, "sha256": digest(path)})
    for name in ("deny-root.log", "deny-desktop.log"):
        path = capture / name
        assert path.read_text().rstrip().endswith("licenses ok"), name
        captured.append({"capture": name, "sha256": digest(path)})
        (output / name.replace(".log", ".txt")).write_text(path.read_text().replace(str(repo) + "/", ""))
    write_csv(output, "capture-hashes.csv", ["capture", "sha256"], captured)
    print(f"Cargo: {len(rows)} unique rows; JS: {len(js)}; payload files/symlinks: {len(payload)}")
    print("Cargo graph sizes:", {scope: sum(row[scope] == "yes" for row in rows) for scope in scopes})


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    normalize(Path(sys.argv[1]), Path(sys.argv[2]))
