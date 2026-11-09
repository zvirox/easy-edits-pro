#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Generate the AF1 profile/tier PROPOSAL CSV for human review.

docs/AGENT_FIRST_PLAN.md §2.1: the inventory must precede any schema or
enforcement work; treat the output as a proposal until the maintainer
approves it. Script emits docs/reference/command-profiles.csv from:

  * docs/reference/command-catalogue.json (generated catalogue; never
    hand-edited — read here as input),
  * the desktop MCP bridge dispatch table in
    apps/saturn-tauri/src/mcp_bridge.rs (dedicated tools),
  * the Python agent harness under agent-harness/cli_anything_eep
    (wrapper operation -> catalogue command mapping).

Rules kept intentionally legible for the review conversation:

  tiers: read / edit / destructive / filesystem (explicit grants, not a
  hierarchy). Multi-effect operations list every applicable tier.
  profiles: the twelve draft profiles from §2.1 plus `review` (read-only).
  A command may serve several profiles; enforcement will require the
  session to hold every matched grant.

This script does NOT touch the catalogue JSON, CommandSpec or any schema;
it reads generated inputs and writes one CSV file.
"""

import csv
import json
import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
CATALOGUE = REPO / "docs" / "reference" / "command-catalogue.json"
OUT = REPO / "docs" / "reference" / "command-profiles.csv"

PROFILES = (
    "ingest", "rough-cut", "fine-cut", "multicam", "colour", "audio",
    "captions", "graphics", "speed-motion", "review", "export", "admin",
)

# Dedicated bridge tools that have no StepCommand body and are not
# catalogue-dispatchable. `command_run` rounds the dedicated set out to the
# full catalogue for generic dispatch.
BRIDGE_TOOLS = {
    "command_list":   ("read", ["review"], "listing only"),
    "command_run":    ("see-mapping", ["see-mapping"], "generic dispatch; its effective tiers are those of the executed command"),
    "doc_inspect":    ("read", ["review"], None),
    "sequence_inspect": ("read", ["review"], None),
    "render_start":   ("edit filesystem", ["export"], "Renders write media/url outputs outside project state"),
    "render_project": ("edit filesystem", ["export"], "Renders write media/url outputs outside project state"),
    "render_status":  ("read", ["export"], None),
    "render_cancel":  ("edit", ["export", "review"], "Cancels a human/agent render job; also usable by review? cancelled only for own job"),
    "media_search":   ("read", ["ingest", "review"], None),
    "media_preview":  ("read", ["ingest", "colour", "review"], None),
    "media_semantic_query": ("read", ["ingest", "review"], None),
    "vision_extract_frame_sheet": ("read filesystem", ["ingest", "review"], "Contact-sheet artifacts write cache files"),
    "render_preview_clip_proxy": ("edit filesystem", ["ingest"], "Proxy renders write files; media-local effect"),
    "color_read_scopes": ("read", ["colour", "review"], None),
    "color_match_shot_to_reference": ("edit", ["colour"], None),
    "audio_process":  ("edit", ["audio"], "Heavy offline DSP into project-local derived media"),
    "audio_read_telemetry": ("read", ["audio", "review"], None),
    "edit_match_cut_align": ("edit", ["fine-cut"], None),
    "timeline_beat_grid_quantize": ("edit", ["fine-cut", "speed-motion"], None),
}

# Commands that are catalogue-present but marked uiOnly — their availability
# via command_run is "refused by engine (uiOnly)". The wrapper harness also
# does not expose them. Rationale per family.
UI_ONLY_RATIONALE = {
    "export.quick":       "User-interactive quick export panel",
    "export.frame":       "User-interactive frame export",
    "file.saveAs":        "Requires native file picker",
    "file.saveCopy":      "Requires native file picker",
    "file.revert":        "Confirm + discard; UI-owned",
    "file.close":         "Window-management action",
    "file.reveal":        "Opens external file manager",
    "view.toggle_snap":   "Viewport toggle",
    "view.zoom_in":       "Viewport zoom",
    "view.zoom_out":      "Viewport zoom",
    "view.zoom_reset":    "Viewport zoom",
    "view.fullscreen":    "Window state",
    "view.glass_full":    "Appearance setting",
    "view.glass_lite":    "Appearance setting",
    "view.glass_off":     "Appearance setting",
    "view.theme_dark":    "Appearance setting",
    "view.theme_light":   "Appearance setting",
    "view.reduced_motion": "Accessibility setting",
    "view.panel_media":   "Panel visibility",
    "view.panel_inspector": "Panel visibility",
    "caption.panel":      "Panel visibility",
    "caption.show_active_only": "Panel filter",
    "caption.cue_select": "Panel selection",
    "caption.go_to":      "Panel navigation",
    "edit.select_all":    "UI selection state",
    "edit.deselect_all":  "UI selection state",
    "edit.select_track":  "UI selection state",
    "edit.select_clips_at_playhead": "UI selection state",
    "edit.select_range":  "UI selection state",
    "edit.invert_selection": "UI selection state",
    "edit.extend_selection": "UI selection state",
    "edit.select_next_clip": "UI selection state",
    "edit.select_prev_clip": "UI selection state",
    "edit.select_under_playhead": "UI selection state",
    "sequence.inspect":   "Already exposed via doc_inspect bridge tool",
    "project.inspect":    "Already exposed via doc_inspect bridge tool",
    "marker.list":        "Panel list",
    "marker.add":         "Panel add",
    "marker.remove":      "Panel remove",
    "marker.update":      "Panel update",
    "marker.clear":       "Panel clear",
    "marker.go_to":       "Panel navigation",
    "trim.in":            "Interactive trim handle",
    "trim.out":           "Interactive trim handle",
    "trim.ripple":        "Interactive trim handle",
    "trim.roll":          "Interactive trim handle",
    "trim.slip":          "Interactive trim handle",
    "trim.slide":         "Interactive trim handle",
    "trim.clear":         "Interactive trim reset",
    "trim.nudge":         "Interactive trim nudge",
    "trim.nudge_large":   "Interactive trim nudge",
    "graphics.select":    "Canvas selection",
    "graphics.add_text":  "Canvas draw",
    "graphics.add_shape": "Canvas draw",
    "graphics.add_image": "Canvas draw",
    "graphics.add_path":  "Canvas draw",
    "help.shortcuts":     "Help overlay",
    "help.about":         "About dialog",
    "help.whats_new":     "Changelog view",
    "help.report_issue":  "External link",
}


# Tier/Profile derivation rules
# ------------------------------------------------------------
# Tiers (explicit grants; not a hierarchy):
#   read       — inspect project/sequence/media, cursors, search, preview, scopes
#   edit       — mutate project state inside the project (timeline, clips, tracks,
#                markers, colors, captions, nodegraph, multicam, audio DSP)
#   destructive— irreversible deletions or destructive resets (delete, remove,
#                clear, discard, reset on mutating commands)
#   filesystem — writes outside project: export files, network, plugin load,
#                proxy generation, derived media caches, contact sheets
#
# Multi-effect operations list EVERY applicable tier (space-separated in CSV).
#
# Profile membership by id prefix + category + mutating + uiOnly:
#   ingest          media.*, interop.import_*, project.add_media, media.search
#   rough-cut       timeline.* (mutating), clip.*, marker.*, speech.*, edit.* (mutating, not destructive)
#   fine-cut        trim.*, timeline.*, edit.match_cut_align, timeline_beat_grid_quantize,
#                   edit.evaluate_cut_continuity
#   multicam        multicam.*
#   colour          color.*, ocio.*, graphic.* (color nodes), clip.set_color*,
#                   vision_extract_frame_sheet, color_read_scopes
#   audio           audio.*, dsp.*, bus.*, track.set_audio_processing, track.*_plugin,
#                   audio_process, audio_read_telemetry
#   captions        caption.*, subtitle.*, speech.transcribe_to_subtitles
#   graphics        graphic.*, nodegraph.*, title.*, timeline.add_title,
#                   timeline.add_transition
#   speed-motion    clip.set_speed_ramp, clip.set_motion_*, timeline_beat_grid_quantize,
#                   playhead.* (non-mutating cursors)
#   review          ALL read-tier + read-ish bridge tools + doc_inspect/
#                   sequence_inspect + uiOnly inspection commands
#   export          export.*, render_*, render_preview_clip_proxy, vision_extract_frame_sheet,
#                   export.quick, export.frame, file.reveal
#   admin           prefs.*, shortcuts.*, project.* (settings), file.open, file.save,
#                   file.revert, file.close, dsp.load_plugin, dsp.set_plugin_param,
#                   track.add_audio_plugin, track.toggle_audio_plugin,
#                   history.list, workspace.*

# Destructive keywords (mutating commands only)
DESTRUCTIVE_RE = re.compile(r'\b(delete|remove|clear|discard|reset|destruct|flatten)\b')

# Filesystem keywords (commands that write outside project, load plugins, network, export)
# Also catches prefs/shortcuts write operations (user config outside project)
FILESYSTEM_RE = re.compile(
    r'\b(export|render|import_|generate|proxy|load_plugin|set_plugin_param|'
    r'add_audio_plugin|toggle_audio_plugin|frame|reveal|attach_proxy|detach_proxy|'
    r'create_proxies|vision_extract|contact_sheet|offline|relink|'
    r'prefs\.(set|reset)|shortcuts\.(set|reset|clear|import|save_preset|load_preset|delete_preset)|'
    r'shortcuts\.presets)\b'
)

# Profile keywords (id prefix -> profile list). Order matters: more specific first.
PROFILE_MAP = [
    # ingest
    (r'^media\.(search|auto_relink|relink_suggest|check_missing|move_to_bin|create_bin|delete_bin|rename_bin|update_properties|list|color_info|generate_|make_offline|offline_all|refresh_changed|attach_proxy|detach_proxy|toggle_proxies|create_proxies)', ["ingest"]),
    (r'^interop\.import_', ["ingest"]),
    (r'^project\.add_media$', ["ingest"]),
    (r'^project\.(fingerprint_media|relink_media|update_media)$', ["ingest"]),
    (r'^edit\.label\.', ["ingest", "rough-cut"]),
    (r'^edit\.(remove_unused_media|consolidate_duplicates)$', ["ingest", "admin"]),
    # rough-cut
    (r'^timeline\.', ["rough-cut"]),
    (r'^clip\.', ["rough-cut"]),
    (r'^marker\.', ["rough-cut"]),
    (r'^speech\.(ripple_delete|transcribe)', ["rough-cut", "captions"]),
    (r'^edit\.(insert|trim|ripple|split|move|replace|overwrite|lift|extract|paste_attributes|remove_attributes|duplicate)', ["rough-cut"]),
    (r'^project\.set(Marks|Notes)$', ["rough-cut", "admin"]),
    (r'^sequence\.(lift|extract|add_edit_all_tracks)$', ["rough-cut", "fine-cut"]),
    # fine-cut
    (r'^trim\.', ["fine-cut"]),
    (r'^edit\.match_cut_align$', ["fine-cut"]),
    (r'^timeline_beat_grid_quantize$', ["fine-cut", "speed-motion"]),
    (r'^edit\.evaluate_cut_continuity$', ["fine-cut"]),
    # multicam
    (r'^multicam\.', ["multicam"]),
    # colour
    (r'^color\.', ["colour"]),
    (r'^ocio\.', ["colour"]),
    (r'^graphic\.', ["colour", "graphics"]),
    (r'^clip\.set_color', ["colour"]),
    (r'^vision_extract_frame_sheet$', ["colour", "ingest"]),
    (r'^color_read_scopes$', ["colour", "review"]),
    # audio
    (r'^audio\.', ["audio"]),
    (r'^dsp\.', ["audio"]),
    (r'^bus\.', ["audio"]),
    (r'^track\.set_audio_processing$', ["audio"]),
    (r'^track\.(add_audio_plugin|toggle_audio_plugin|set_bus_route)$', ["audio"]),
    (r'^track\.set_(gain_db|pan|muted|solo)$', ["audio"]),
    (r'^track\.(add_automation_point|remove_automation_point|set_automation|set_automation_interpolation|set_automation_mode|clear_automation)$', ["audio"]),
    (r'^audio_process$', ["audio"]),
    (r'^audio_read_telemetry$', ["audio", "review"]),
    # captions
    (r'^caption\.', ["captions"]),
    (r'^subtitle\.', ["captions"]),
    (r'^speech\.transcribe_to_subtitles$', ["captions"]),
    # graphics
    (r'^nodegraph\.', ["graphics"]),
    (r'^title\.', ["graphics"]),
    (r'^timeline\.(add_title|add_transition)$', ["graphics"]),
    # speed-motion
    (r'^clip\.set_(speed_ramp|motion_blur|motion_tracking)$', ["speed-motion"]),
    (r'^playhead\.', ["speed-motion", "review"]),
    # review — read-tier + bridge read tools + uiOnly inspection
    # (computed from tier below)
    # export
    (r'^export\.', ["export"]),
    (r'^render_', ["export"]),
    (r'^render_preview_clip_proxy$', ["export", "ingest"]),
    (r'^vision_extract_frame_sheet$', ["export", "ingest"]),
    (r'^file\.reveal$', ["export"]),
    (r'^media\.import$', ["ingest"]),
    (r'^media\.(proxy|create_proxies|attach_proxy|detach_proxy)$', ["ingest"]),
    (r'^render\.queue$', ["export"]),
    # admin
    (r'^prefs\.', ["admin"]),
    (r'^shortcuts\.', ["admin"]),
    (r'^project\.(preferences|settings|window|workspace|remove|branch_|new|switch_sequence)', ["admin"]),
    (r'^file\.(open|save|revert|close)$', ["admin"]),
    (r'^dsp\.(load_plugin|set_plugin_param)$', ["admin"]),
    (r'^track\.(add_audio_plugin|toggle_audio_plugin|add|move|settings|set_locked)$', ["admin"]),
    (r'^history\.', ["admin", "review"]),
    (r'^workspace\.', ["admin"]),
    (r'^project\.inspect$', ["review", "admin"]),
    (r'^sequence\.inspect$', ["review", "admin"]),
    (r'^sequence\.(delete|remove|create|rename|duplicate|settings|close)$', ["admin"]),
    (r'^edit\.(undo|redo)$', ["admin"]),
]


def derive_tiers(cmd: dict, is_bridge_tool: bool = False) -> tuple[str, list[str]]:
    """Return (tier_string, profile_list) for a catalogue command or bridge tool."""
    cid = cmd.get("id", "")
    mutating = bool(cmd.get("mutating", False))
    ui_only = bool(cmd.get("uiOnly", False))
    category = cmd.get("category", "")

    tiers = []

    if is_bridge_tool:
        # Handled by BRIDGE_TOOLS mapping
        return "", []

    # Base tier from mutating + uiOnly
    if ui_only:
        # UI-only commands are not invocable via command_run; they are read-ish for review
        if mutating:
            # Should not happen per data, but be safe
            tiers.append("read")
        else:
            tiers.append("read")
    elif mutating:
        tiers.append("edit")
        # Check destructive
        if DESTRUCTIVE_RE.search(cid):
            tiers.append("destructive")
        # Check filesystem
        if FILESYSTEM_RE.search(cid):
            tiers.append("filesystem")
    else:
        # Non-mutating, non-uiOnly
        tiers.append("read")
        # Some non-mutating commands still write outside (prefs.set, shortcuts.set)
        if FILESYSTEM_RE.search(cid):
            tiers.append("filesystem")

    # De-dupe while preserving order
    seen = set()
    tiers = [t for t in tiers if not (t in seen or seen.add(t))]

    # Profiles
    profiles = set()
    for pattern, prof_list in PROFILE_MAP:
        if re.search(pattern, cid):
            profiles.update(prof_list)

    # Review profile: all read-tier commands + bridge read tools
    if "read" in tiers:
        profiles.add("review")
    # Also add review to explicitly listed read bridge tools via profile map

    # Core candidate rule: undo/branch mutations are edit, not review
    if cid in ("history.undo", "history.redo", "history.branch", "history.checkpoint"):
        profiles.discard("review")

    # Admin profile is broad; keep explicit mappings

    return " ".join(tiers), sorted(profiles)


def get_availability(cmd: dict, wrapper_ops: set, unmatched_wrapper_ops: set) -> str:
    """Availability string for CSV: 'catalogue', 'catalogue+wrapper', 'bridge', 'wrapper-only', 'uiOnly', 'unshipped'."""
    cid = cmd.get("id", "")
    ui_only = bool(cmd.get("uiOnly", False))
    shipped = bool(cmd.get("shipped", True))

    if not shipped:
        return "unshipped"
    if ui_only:
        return "uiOnly"
    if cid in BRIDGE_TOOLS:
        return "bridge"
    if cid in wrapper_ops:
        return "catalogue+wrapper"
    if cid in unmatched_wrapper_ops:
        return "wrapper-only"
    return "catalogue"


def get_rationale(cmd: dict, wrapper_ops: set, unmatched_wrapper_ops: set) -> str:
    """Human-readable rationale for tier/profile assignments and exceptions."""
    cid = cmd.get("id", "")
    mutating = bool(cmd.get("mutating", False))
    ui_only = bool(cmd.get("uiOnly", False))

    parts = []

    # UI-only rationale
    if ui_only:
        parts.append(UI_ONLY_RATIONALE.get(cid, "UI-owned action; not invocable via command_run"))

    # Mutating but read-tier (semantic write)
    if not mutating and not ui_only and FILESYSTEM_RE.search(cid):
        parts.append("Semantic write (prefs/shortcuts/export) despite mutating=false")

    # Destructive on mutating
    if mutating and DESTRUCTIVE_RE.search(cid):
        parts.append("Destructive mutation (delete/remove/clear/reset/discard)")

    # Filesystem effect
    if FILESYSTEM_RE.search(cid):
        parts.append("Filesystem/network/plugin effect (outside project state)")

    # Wrapper availability
    if cid in wrapper_ops:
        parts.append("Wrapper operation mapped to this catalogue command")
    elif cid in unmatched_wrapper_ops:
        parts.append("Wrapper operation has no catalogue equivalent (unsupported in CLI)")

    # Bridge tool rationale
    if cid in BRIDGE_TOOLS:
        parts.append(BRIDGE_TOOLS[cid][2] or "Dedicated bridge tool")

    return "; ".join(parts) if parts else "Standard mapping"


def load_wrapper_ops():
    """Extract all id-shaped strings from agent-harness commands and split by catalogue presence."""
    import glob
    ops = set()
    for p in glob.glob("agent-harness/cli_anything_eep/commands/*.py"):
        txt = open(p).read()
        for m in re.finditer(r'"([a-z]+[a-z0-9_]*\.[a-z0-9_]+)"', txt):
            ops.add(m.group(1))

    cat = {c["id"] for c in json.load(open(CATALOGUE))}
    matched = {o for o in ops if o in cat}
    unmatched = {o for o in ops if o not in cat}
    return matched, unmatched


def get_wrapper_op_profiles(op_id: str) -> list[str]:
    """Assign proposed profiles to wrapper-only ops based on id prefix."""
    profiles = set()
    for pattern, prof_list in PROFILE_MAP:
        if re.search(pattern, op_id):
            profiles.update(prof_list)
    return sorted(profiles)


def main():
    cat = json.load(open(CATALOGUE))
    wrapper_ops, unmatched_wrapper_ops = load_wrapper_ops()

    rows = []

    # 1. Catalogue commands (465)
    for c in cat:
        cid = c["id"]
        tier_str, profiles = derive_tiers(c)
        avail = get_availability(c, wrapper_ops, unmatched_wrapper_ops)
        rationale = get_rationale(c, wrapper_ops, unmatched_wrapper_ops)
        rows.append({
            "id": cid,
            "kind": "command",
            "existing_availability": avail,
            "proposed_tier": tier_str,
            "proposed_profiles": ",".join(profiles),
            "exceptions_rationale": rationale,
        })

    # 2. Dedicated bridge tools (19)
    for name, (tier, profs, rat) in BRIDGE_TOOLS.items():
        rows.append({
            "id": name,
            "kind": "tool",
            "existing_availability": "bridge",
            "proposed_tier": tier,
            "proposed_profiles": ",".join(profs),
            "exceptions_rationale": rat or "Dedicated bridge tool",
        })

    # 3. Wrapper-only operations (14 unsupported) - assign proposed profiles for review
    for op in sorted(unmatched_wrapper_ops):
        proposed_profiles = get_wrapper_op_profiles(op)
        rows.append({
            "id": op,
            "kind": "wrapper-op",
            "existing_availability": "wrapper-only",
            "proposed_tier": "unavailable",
            "proposed_profiles": ",".join(proposed_profiles),
            "exceptions_rationale": f"Wrapper operation reports unsupported; no catalogue equivalent. Proposed profiles: {','.join(proposed_profiles) or 'none'}",
        })

    # Sort: kind order, then id
    kind_order = {"command": 0, "tool": 1, "wrapper-op": 2}
    rows.sort(key=lambda r: (kind_order.get(r["kind"], 9), r["id"]))

    # Write CSV
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=[
            "id", "kind", "existing_availability", "proposed_tier",
            "proposed_profiles", "exceptions_rationale"
        ])
        writer.writeheader()
        writer.writerows(rows)

    print(f"Wrote {len(rows)} rows to {OUT}")
    print(f"  commands: {sum(1 for r in rows if r['kind']=='command')}")
    print(f"  bridge tools: {sum(1 for r in rows if r['kind']=='tool')}")
    print(f"  wrapper-only: {sum(1 for r in rows if r['kind']=='wrapper-op')}")

    # Spot check
    for r in rows:
        if r["id"] in ("export.quick", "file.revert", "prefs.set", "shortcuts.reset",
                       "track.add_audio_plugin", "render_start", "command_run"):
            print(f"  {r['id']:35} tier={r['proposed_tier']:20} profiles={r['proposed_profiles']}")


if __name__ == "__main__":
    main()