#!/usr/bin/env python3
"""MCP command-matrix test: exercise every shipped command of the running app.

Launches the desktop app under Xvfb with an isolated HOME, connects to its
local MCP bridge (same socket the acceptance runbook drives), and runs every
command in `command_list` with crafted parameters.

Honesty model: records carry a class field.
  ok       - bridge accepted the command
  ui_only  - answered "handled by the interface" (by design over MCP)
  error    - bridge returned an error for the crafted parameters; whether that
             is a product bug or a needs-state limitation is judged in the
             analysis afterwards, not silently folded into pass.

Every mutating command that succeeds is rolled back with `edit.undo`.
Commands needing editor-held state (edit points, targeted tracks, multicam
clips, compound clips, in/out marks) get hook commands first; mutating hooks
are undone right after. A full baseline rebuild is the fallback when undo
fails, and after document-replacing commands (file.open, imports).
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

TICKS_PER_SECOND = 254_016_000_000


def ticks(seconds: float) -> int:
    return round(seconds * TICKS_PER_SECOND)


# ---------------------------------------------------------------- bridge I/O

def bridge_call(sock_path: Path, method: str, params=None, timeout=25):
    """One JSONL bridge request. Returns (ok, result_or_error)."""
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(timeout)
    try:
        client.connect(str(sock_path))
        request = {"method": method}
        if params is not None:
            request["params"] = params
        client.sendall(json.dumps(request).encode() + b"\n")
        buffer = bytearray()
        while not buffer.endswith(b"\n"):
            count = client.recv(65536)
            if not count:
                return False, "socket closed"
            buffer.extend(count)
        response = json.loads(bytes(buffer))
    except (socket.timeout, OSError) as error:
        return False, f"bridge I/O: {error}"
    finally:
        client.close()
    if "error" in response:
        return False, response["error"]
    return True, response.get("result")


def run_cmd(sock, cmd_id: str, params=None, timeout=25):
    return bridge_call(sock, "command_run", {"id": cmd_id, "params": params or {}}, timeout)


# ------------------------------------------------------------ params parsing

def parse_params(s: str) -> dict:
    """Lenient parse of catalogue param hints like {"edge":"start"|"end"?}."""
    s = (s or "").strip()
    if s in ("", "{}"):
        return {}
    s = re.sub(r"([,{]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:", r'\1"\2":', s)
    s = re.sub(r':\s*"?([A-Za-z_][A-Za-z0-9_\[\]\.,\+ |]*)"?(\??)\s*(?=[,}])', r': "\1\2"', s)
    try:
        return json.loads(s)
    except json.JSONDecodeError:
        out = {}
        for m in re.finditer(r'"?([A-Za-z_][A-Za-z0-9_]*)"?\s*:\s*([^,}]+)', s):
            out[m.group(1)] = m.group(2).strip().strip('"')
        return out


TICK_KEYS = {
    "at", "at_tick", "at_ticks", "start", "end", "start_ticks", "end_ticks",
    "timeline_start", "source_in", "duration", "cut_at", "delta",
    "new_duration", "time", "start_time", "end_time",
}
CLIP_INDEX_KEYS = {"clip_index", "from_index", "to_index", "target_index",
                   "left_clip_index", "index"}

SRT_SAMPLE = (
    "1\n00:00:00,200 --> 00:00:01,800\nHello matrix\n\n"
    "2\n00:00:02,000 --> 00:00:03,500\nSecond cue\n"
)
OTIO_SAMPLE = json.dumps({
    "OTIO_SCHEMA": "Timeline.1",
    "name": "Matrix OTIO",
    "metadata": {},
    "global_start_time": None,
    "tracks": {
        "OTIO_SCHEMA": "Stack.1", "name": "tracks", "children": [],
        "markers": [], "metadata": {}, "source_range": None, "effects": [],
    },
})


class Context:
    """Live ids and paths shared by param builders."""

    def __init__(self, fixture: Path, tmp: Path):
        self.fixture = fixture
        self.tmp = tmp
        self.doc = {}
        self.media_id = None
        self.spare_media_id = None
        self.title_media_id = None
        self.v_track = None
        self.v2_track = None
        self.a_track = None
        self.marker_id = None
        self.effect_id = None
        self.shader_effect_id = None
        self.transition_id = None
        self.cue_id = None
        self.cue2_id = None
        self.cap_track = None
        self.cap_track2 = None
        self.subtitle_id = None
        self.graphic_id = None
        self.graphic_ids = []
        self.template_id = None
        self.bin_id = None
        self.group_id = None
        self.seq2_id = None
        self.seq1_id = None
        self.node_id = None
        self.multicam_media_id = None
        self.saved_project = str(tmp / "baseline.eep")
        self.export_xml = None

    def clip_ref(self, track_id=None, index=0):
        return {"track_id": track_id if track_id is not None else self.v_track,
                "clip_index": index}


def state_signature(ctx: Context) -> tuple:
    """Cheap drift signature of the working sequence."""
    seq = ctx.doc.get("sequence", {}) if isinstance(ctx.doc, dict) else {}
    tracks = tuple(
        (t.get("id"), len(t.get("clips") or []))
        for t in seq.get("tracks", []))
    return (
        tracks,
        len(ctx.doc.get("markers") or []),
        len(ctx.doc.get("media") or []),
        len(ctx.doc.get("sequences") or []),
    )


def refresh_ctx(ctx: Context, sock) -> dict:
    ok, doc = bridge_call(sock, "doc_inspect")
    if not ok:
        return ctx.doc
    project = doc.get("project", {})
    if isinstance(project, dict) and "project" in project:
        project = project["project"]
    ctx.doc = project
    tracks = project.get("sequence", {}).get("tracks", [])
    ctx.seq1_id = project.get("sequence", {}).get("id") or ctx.seq1_id
    videos = [t for t in tracks if t.get("kind") == "video"]
    audios = [t for t in tracks if t.get("kind") == "audio"]
    if videos:
        ctx.v_track = videos[0]["id"]
        if len(videos) > 1:
            ctx.v2_track = videos[1]["id"]
    if audios:
        ctx.a_track = audios[0]["id"]
    media = project.get("media") or []
    by_name = {m.get("name"): m.get("id") for m in media}
    ctx.media_id = by_name.get(ctx.fixture.name) or (media[0].get("id") if media else None)
    ctx.spare_media_id = by_name.get("Spare") or ctx.spare_media_id
    ctx.title_media_id = by_name.get("Title 1") or ctx.title_media_id
    return ctx.doc


# --------------------------------------------------------------- baseline

def capture_result(ctx: Context, cid: str, result) -> None:
    """Update ctx ids from successful family-creator results."""
    if not isinstance(result, dict):
        return

    def grab(*keys):
        for k in keys:
            if result.get(k) is not None:
                return result[k]
        return None

    if cid == "marker.add":
        ctx.marker_id = grab("id")
    elif cid == "media.create_bin":
        b = result.get("bin")
        if isinstance(b, dict):
            ctx.bin_id = b.get("id")
    elif cid == "graphic.add":
        gid = grab("id")
        if gid is not None:
            ctx.graphic_ids.append(gid)
        ctx.graphic_id = ctx.graphic_ids[0] if ctx.graphic_ids else None
    elif cid == "graphic.template_save":
        ctx.template_id = grab("template_id", "id")
    elif cid == "caption.track_add":
        if ctx.cap_track is None:
            ctx.cap_track = grab("id", "track_id")
        else:
            ctx.cap_track2 = grab("id", "track_id")
    elif cid == "caption.cue_add":
        ctx.cue2_id = ctx.cue_id
        ctx.cue_id = grab("id")
    elif cid == "subtitle.add":
        ctx.subtitle_id = grab("id")
    elif cid == "clip.add_effect":
        eid = grab("effect_id", "id")
        if ctx.effect_id is None:
            ctx.effect_id = eid
        else:
            ctx.shader_effect_id = eid
    elif cid == "timeline.add_transition":
        ctx.transition_id = grab("transition_id", "id")
    elif cid == "timeline.create_track_group":
        g = result.get("group")
        ctx.group_id = g.get("id") if isinstance(g, dict) else grab("group_id", "id")
    elif cid == "sequence.create":
        ctx.seq2_id = grab("id", "sequence_id")
    elif cid == "nodegraph.add_node":
        ctx.node_id = grab("node_id", "id")
    elif cid == "multicam.create":
        ctx.multicam_media_id = grab("media_id", "id")


def call_baseline(ctx: Context, sock, cid, params, errors: list):
    ok, result = run_cmd(sock, cid, params)
    if ok:
        capture_result(ctx, cid, result)
    else:
        errors.append(f"{cid}: {str(result)[:120]}")
    return ok, result


def reset_to_baseline(ctx: Context, sock, meta: dict, work: Path, stats: dict) -> None:
    """Rebuild the full rich baseline; capture every id the overrides need."""
    errs: list = []
    ctx.media_id = ctx.spare_media_id = ctx.title_media_id = None
    ctx.cap_track = ctx.cap_track2 = None
    ctx.cue_id = ctx.cue2_id = None
    ctx.effect_id = ctx.shader_effect_id = None

    def r(cid, params=None):
        return call_baseline(ctx, sock, cid, params, errs)

    def mid(res):
        return res.get("media_id") if isinstance(res, dict) else None

    r("project.new", {"name": "Matrix", "sequence_name": "Seq"})
    _, res = r("project.add_media", {
        "name": ctx.fixture.name, "path": str(ctx.fixture), "kind": "video",
        "duration": ticks(meta["duration"]), "width": meta["width"],
        "height": meta["height"], "fps": meta["fps"], "codec": meta["codec"],
    })
    ctx.media_id = mid(res)
    # a distinct file so the dedupe-by-path in add_media does not merge them
    spare_file = work / "spare-copy.webm"
    if not spare_file.exists():
        shutil.copyfile(ctx.fixture, spare_file)
    _, res = r("project.add_media", {"name": "Spare", "path": str(spare_file),
                                     "kind": "video", "duration": ticks(meta["duration"])})
    ctx.spare_media_id = mid(res)
    _, res = r("project.add_media", {"name": "Title 1", "path": "title:///Matrix",
                                     "kind": "title", "title": {"text": "Matrix"}})
    ctx.title_media_id = mid(res)
    refresh_ctx(ctx, sock)
    # three contiguous clips on V1 (roll/slide want adjacency), one on V2
    for start in (0.0, 2.0, 4.0):
        r("timeline.add_clip", {"track_id": ctx.v_track, "media_id": ctx.media_id,
                                "timeline_start": ticks(start), "duration": ticks(1.9)})
    # 8 video + 8 audio tracks so timeline.toggleTargetV1..V8/A1..A8 all apply
    for _ in range(7):
        r("timeline.add_track", {"kind": "video"})
    for _ in range(7):
        r("timeline.add_track", {"kind": "audio"})
    refresh_ctx(ctx, sock)
    if ctx.v2_track is not None:
        r("timeline.add_clip", {"track_id": ctx.v2_track, "media_id": ctx.media_id,
                                "timeline_start": 0, "duration": ticks(1.9)})
    r("marker.add", {"at": ticks(1.0), "name": "Marker A"})
    r("media.create_bin", {"name": "Shots"})
    r("graphic.add", {"kind": "text", "name": "T1", "text": "Hi",
                      "start_ticks": 0, "duration_ticks": ticks(2.0)})
    r("graphic.add", {"kind": "rectangle", "name": "R1", "start_ticks": 0,
                      "duration_ticks": ticks(2.0)})
    r("graphic.add", {"kind": "ellipse", "name": "E1", "start_ticks": 0,
                      "duration_ticks": ticks(2.0)})
    if ctx.graphic_id is not None:
        r("graphic.template_save", {"graphic_id": ctx.graphic_id, "name": "MatrixTpl"})
    r("caption.track_add", {"name": "C1", "language": "en"})
    r("caption.track_add", {"name": "C2", "language": "en"})
    if ctx.cap_track is not None:
        r("caption.cue_add", {"track_id": ctx.cap_track, "start_ticks": ticks(0.2),
                              "end_ticks": ticks(0.8), "text": "Hello"})
        r("caption.cue_add", {"track_id": ctx.cap_track, "start_ticks": ticks(1.2),
                              "end_ticks": ticks(1.8), "text": "World"})
    r("subtitle.add", {"start_ticks": ticks(0.2), "end_ticks": ticks(0.8), "text": "Hi"})
    r("subtitle.add", {"start_ticks": ticks(1.2), "end_ticks": ticks(1.8), "text": "Yo"})
    if ctx.v_track is not None:
        r("clip.add_effect", {"track_id": ctx.v_track, "clip_index": 0,
                              "kind": "color_correction"})
        r("clip.add_effect", {"track_id": ctx.v_track, "clip_index": 0,
                              "kind": "custom_shader"})
        r("timeline.add_transition", {"track_id": ctx.v_track, "cut_at": ticks(1.0)})
    r("timeline.create_track_group", {"name": "G1"})
    r("project.searchBin.set", {"name": "Matrix", "query": "shot"})
    r("project.viewPreset.saveAs", {"name": "Matrix"})
    r("export.presets.save", {"name": "Matrix",
                              "settings": {"format": "webm", "width": 1920, "height": 1080}})
    ok, _ = run_cmd(sock, "interop.export_fcpxml", {"path": str(work / "baseline.fcpxml")})
    if ok:
        try:
            ctx.export_xml = (work / "baseline.fcpxml").read_text()
        except OSError:
            ctx.export_xml = None
    # second sequence at the very end: sequence.create activates the blank
    # sequence, so switch back to the working one afterwards
    r("sequence.create", {"name": "Seq2"})
    if ctx.seq1_id is not None:
        run_cmd(sock, "project.switch_sequence", {"sequence_id": ctx.seq1_id})
    r("file.save", {"path": str(work / "baseline.eep")})
    refresh_ctx(ctx, sock)
    stats["resets"] += 1
    if errs:
        print(f"[matrix] baseline setup problems: {errs[:8]}")


# ----------------------------------------------------------- param builder

def spare_audio_track_id(ctx: Context) -> int | None:
    audios = [t for t in ctx.doc.get("sequence", {}).get("tracks", [])
              if t.get("kind") == "audio"]
    if len(audios) >= 2:
        return audios[-1]["id"]
    return None


def generic_params(cmd: dict, ctx: Context, tmp: Path) -> dict:
    hints = parse_params(cmd["params"])
    params = {}
    for key, hint in hints.items():
        raw = str(hint).split("|")
        optional = any(t.strip().endswith("?") for t in raw)
        tokens = [t.strip().rstrip("?").strip('"') for t in raw]
        base = tokens[0] if tokens and tokens[0] else "obj"
        if base.startswith("["):
            value = []  # array-typed params are normally given by overrides
        elif key == "media_id":
            value = ctx.media_id
        elif key == "track_id":
            value = ctx.v_track if ctx.v_track is not None else 1
        elif key in CLIP_INDEX_KEYS:
            value = 0
        elif key == "effect_id":
            value = ctx.effect_id if ctx.effect_id is not None else 1
        elif key == "transition_id":
            value = ctx.transition_id if ctx.transition_id is not None else 1
        elif key == "bin_id":
            value = ctx.bin_id if ctx.bin_id is not None else 1
        elif key == "group_id":
            value = ctx.group_id if ctx.group_id is not None else 1
        elif key == "sequence_id":
            value = ctx.seq2_id if ctx.seq2_id is not None else 1
        elif key in TICK_KEYS:
            value = ticks(1.5) if key in ("end", "end_ticks", "end_time") else ticks(0.5)
        elif base == "int":
            value = 1
        elif base == "num":
            value = 1.0
        elif base == "bool":
            value = True
        elif base == "path":
            value = str(tmp / f"{cmd['id'].replace('.', '_')}")
        elif base == "str" and key in ("name", "note", "text", "query", "title"):
            value = "Matrix"
        elif base == "str":
            value = ""
        elif tokens:
            value = tokens[0]  # enum alternatives: keep the first
        else:
            value = {}
        params[key] = value
        if optional and value != []:
            params.pop(key, None)
    return params


def graphic_args(ctx: Context, cid: str) -> dict:
    g = ctx.graphic_id
    table = {
        "graphic.add": {"kind": "text", "name": "T2", "text": "New",
                        "start_ticks": 0, "duration_ticks": ticks(1.0)},
        "graphic.remove": {"id": g},
        "graphic.duplicate": {"id": g},
        "graphic.set_fill": {"id": g, "color": "#ff0000", "opacity": 1.0},
        "graphic.set_stroke": {"id": g, "color": "#00ff00", "width": 2.0},
        "graphic.set_shadow": {"id": g, "color": "#000000c0", "blur": 4.0,
                               "offset_x": 1.0, "offset_y": 1.0},
        "graphic.set_text": {"id": g, "text": "Matrix", "font_size": 32},
        "graphic.set_transform": {"id": g, "x": 10.0, "y": 10.0, "scale": 1.0},
        "graphic.set_flags": {"id": g, "visible": True, "locked": False},
        "graphic.arrange": {"id": g, "order": "front"},
        "graphic.template_save": {"graphic_id": g, "name": "MatrixTpl"},
        "graphic.template_apply": {"template_id": ctx.template_id or 1,
                                   "start_ticks": ticks(0.2)},
    }
    args = table.get(cid) or {"id": g}
    return {k: val for k, val in args.items() if val is not None} or {"id": 1}


def marker_args(ctx: Context, cid: str) -> dict:
    m = ctx.marker_id
    table = {
        "marker.add": {"at": ticks(1.2), "name": "Matrix"},
        "marker.update": {"id": m, "name": "Marker A2"},
        "marker.remove": {"id": m},
        "marker.add_chapter": {"at": ticks(1.2), "name": "Chapter"},
        "marker.add_flash_cue": {"at": ticks(1.3), "name": "Flash"},
        "marker.split": {"at": ticks(1.0)},
        "marker.ripple": {"from": ticks(1.0), "delta": ticks(0.1)},
        "marker.set_split_audio": {"id": m, "at": ticks(0.5)},
        "marker.set_split_audio_out": {"id": m, "at": ticks(0.7)},
        "marker.set_split_video": {"id": m, "at": ticks(0.6)},
        "marker.set_split_video_out": {"id": m, "at": ticks(0.8)},
        "marker.set_color_filter": {"colors": ["red"]},
        "marker.set_copy_paste_includes": {"enabled": True},
        "marker.add_range": {"start": ticks(0.5), "end": ticks(1.5), "name": "Range"},
        "marker.next": {"from": ticks(2.0)},
        "marker.previous": {"from": ticks(2.0)},
        "marker.import_csv": {"csv": "Alpha,chapter,100,0\nBeta,flash,200,0\n",
                              "replace": True},
    }
    resolved = table.get(cid)
    if isinstance(resolved, dict):
        return {k: val for k, val in resolved.items() if val is not None} or {}
    return {}


def caption_args(ctx: Context, cid: str) -> dict:
    t = ctx.cap_track
    cue = ctx.cue_id
    table = {
        "caption.track_add": {"name": "C3"},
        "caption.cue_add": {"track_id": t, "start_ticks": ticks(2.2),
                            "end_ticks": ticks(2.8), "text": "New"},
        "caption.track_rename": {"id": t, "name": "C1"},
        "caption.track_remove": {"id": ctx.cap_track2 or t},
        "caption.style_set": {"track_id": t, "font_size": 32.0, "font_family": "Sans"},
        "caption.cue_update": {"track_id": t, "id": cue, "text": "Matrix"},
        "caption.cue_nudge": {"track_id": t, "id": cue, "delta_ticks": ticks(0.05)},
        "caption.cue_split": {"track_id": t, "id": cue, "at_ticks": ticks(1.5)},
        "caption.cue_remove": {"track_id": t, "id": cue},
        "caption.cue_delete": {"track_id": t, "ids": [x for x in (ctx.cue2_id,) if x]},
        "caption.cue_merge": {"track_id": t,
                              "ids": [x for x in (ctx.cue_id, ctx.cue2_id) if x]},
        "caption.import": {"track_id": t, "content": SRT_SAMPLE},
        "caption.next": {"from": ticks(0.5), "track_id": t},
        "caption.previous": {"from": ticks(0.5), "track_id": t},
    }
    return table.get(cid) or {}


def O(ctx: Context, cid: str):
    """Authoritative overrides - fully replace the generic params."""
    v, v2 = ctx.v_track, ctx.v2_track
    if cid.startswith("edit.label."):
        return {"clips": [ctx.clip_ref()]}
    if cid == "timeline.ripple_delete_batch":
        return {"clips": [ctx.clip_ref(v, 0), ctx.clip_ref(v, 1)]}
    if cid in ("timeline.link_clips", "timeline.unlink_clips"):
        return {"clips": [ctx.clip_ref(v, 0), ctx.clip_ref(v2, 0)]}
    if cid == "timeline.mark_selection":
        return {"clips": [ctx.clip_ref(v, 0)]}
    if cid == "timeline.create_compound_clip":
        return {"name": "Compound", "clips": [ctx.clip_ref(v, 0), ctx.clip_ref(v2, 0)]}
    if cid == "timeline.decompose_compound_clip":
        return {"track_id": v, "clip_index": 0}
    if cid == "sequence.reveal_nested":
        return {"track_id": v, "clip_index": 0}
    if cid == "timeline.roll_edit":
        return {"track_id": v, "left_clip_index": 0, "delta": ticks(0.1)}
    if cid in ("timeline.slide_clip", "timeline.slip_clip"):
        return {"track_id": v, "clip_index": 1, "delta": ticks(0.1)}
    if cid == "timeline.ripple_delete_gap":
        return {"track_id": v2, "at_tick": ticks(3.0)}
    if cid == "timeline.rate_stretch":
        return {"track_id": v, "clip_index": 0, "speed": 1.5}
    if cid == "timeline.move_clips_batch":
        return {"moves": [{"track_id": v, "clip_index": 0,
                           "target_track_id": v2, "timeline_start": ticks(0.2)}]}
    if cid in ("timeline.trim_clip", "trim.inspect"):
        out = {"track_id": v, "clip_index": 0, "edge": "start"}
        if cid == "timeline.trim_clip":
            out["delta"] = ticks(0.1)
            out["ripple"] = False
        return out
    if cid == "timeline.update_transition":
        return {"track_id": v, "transition_id": ctx.transition_id or 1,
                "kind": "crossfade", "duration": ticks(0.5)}
    if cid == "timeline.set_track_group_properties":
        return {"group_id": ctx.group_id or 1, "name": "G1", "muted": False}
    if cid in ("timeline.toggle_track_group_collapsed", "timeline.delete_track_group"):
        return {"group_id": ctx.group_id or 1}
    if cid == "timeline.remove_track":
        spare = spare_audio_track_id(ctx)
        return {"track_id": spare} if spare else {}
    if cid == "multicam.create":
        return {"name": "MC",
                "angles": [{"id": 1, "name": "A", "media_id": ctx.media_id},
                           {"id": 2, "name": "B",
                            "media_id": ctx.spare_media_id or ctx.media_id}],
                "sync_mode": "timecode"}
    if cid == "multicam.switch_angle":
        return {"track_id": v, "clip_index": 0, "angle_id": 1}
    if cid == "multicam.sync":
        return {"multicam_id": ctx.multicam_media_id or ctx.media_id,
                "sync_mode": "audio_waveform"}
    if cid == "multicam.flatten":
        return {"track_id": v, "clip_index": 0}
    if cid == "clip.add_effect":
        return {"track_id": v, "clip_index": 0, "kind": "color_correction"}
    if cid == "clip.update_effect":
        return {"track_id": v, "clip_index": 0, "effect_id": ctx.effect_id or 1,
                "payload": {"kind": "color_correction", "params": {}}}
    if cid == "clip.toggle_effect":
        return {"track_id": v, "clip_index": 0, "effect_id": ctx.effect_id or 1}
    if cid == "clip.remove_effect":
        return {"track_id": v, "clip_index": 0, "effect_id": ctx.shader_effect_id or 2}
    if cid == "clip.reorder_effects":
        return {"track_id": v, "clip_index": 0, "from_index": 0, "to_index": 1}
    if cid == "clip.update_custom_shader":
        return {"track_id": v, "clip_index": 0,
                "effect_id": ctx.shader_effect_id or 2, "shader_code": "void main(){}"}
    if cid in ("clip.set_keyframe", "clip.remove_keyframe"):
        return {"track_id": v, "clip_index": 0, "property": "opacity",
                "time": ticks(0.5), "value": 1.0}
    if cid == "clip.frame_fit":
        return {"track_id": v, "clip_index": 0, "mode": "fill"}
    if cid == "clip.set_color_wheels":
        return {"track_id": v, "clip_index": 0,
                "wheels": {"lift": {"red": 0.1}, "gain": {"blue": 0.1}}}
    if cid == "clip.set_motion_tracking":
        return {"track_id": v, "clip_index": 0,
                "tracking": {"id": 1, "name": "Track1", "points": []}}
    if cid == "clip.set_speed_ramp":
        return {"track_id": v, "clip_index": 0,
                "speed_ramp": {"points": [{"time": ticks(0.5), "speed_factor": 1.5}]}}
    if cid == "clip.set_motion_blur":
        return {"track_id": v, "clip_index": 0,
                "motion_blur": {"enabled": True, "shutter_angle_deg": 180.0, "samples": 8}}
    if cid == "clip.update_rgb_curves":
        return {"track_id": v, "clip_index": 0,
                "curves": {"master": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}]}}
    if cid == "clip.update_hsl_qualifier":
        return {"track_id": v, "clip_index": 0,
                "qualifier": {"enabled": True, "hue_center": 30.0}}
    if cid == "nodegraph.add_node":
        return {"track_id": v, "clip_index": 0, "kind": "merge", "title": "Node"}
    if cid == "nodegraph.update_node":
        return {"track_id": v, "clip_index": 0, "node_id": ctx.node_id or 1, "enabled": True}
    if cid == "nodegraph.add_mask":
        return {"track_id": v, "clip_index": 0,
                "mask": {"id": 1, "name": "M1", "closed": True,
                         "points": [{"x": 0.1, "y": 0.1}, {"x": 0.9, "y": 0.1},
                                    {"x": 0.9, "y": 0.9}]}}
    if cid.startswith("graphic.align"):
        ids = [x for x in ctx.graphic_ids if x]
        return {"ids": ids, "mode": "left"}
    if cid == "graphic.distribute":
        ids = [x for x in ctx.graphic_ids if x]
        return {"ids": ids, "axis": "horizontal"}
    if cid.startswith("graphic."):
        return graphic_args(ctx, cid)
    if cid.startswith("marker."):
        return marker_args(ctx, cid)
    if cid.startswith("caption."):
        return caption_args(ctx, cid)
    if cid == "subtitle.update":
        return {"id": ctx.cue_id or ctx.subtitle_id or 1, "text": "Matrix"}
    if cid == "title.update":
        return {"media_id": ctx.title_media_id or ctx.media_id, "text": "Hello"}
    if cid == "project.searchBin.set":
        return {"name": "Matrix", "query": "shot"}
    if cid in ("project.searchBin.delete", "project.searchBin.items"):
        return {"name": "Matrix"}
    if cid == "prefs.set":
        return {"values": {"ui.autosave_minutes": 5}}
    if cid == "prefs.reset":
        return {}
    if cid == "shortcuts.set" or cid == "shortcuts.import":
        return {"shortcuts": {"edit.undo": "ctrl+shift+z"}}
    if cid == "project.viewPreset.rename":
        return {"from": "Matrix", "to": "Matrix2"}
    if cid == "project.viewPreset.saveAs":
        return {"name": "Matrix3"}
    if cid in ("project.viewPreset.restore", "project.viewPreset.delete"):
        return {"name": "Matrix"}
    if cid == "project.columns.set":
        return {"columns": [{"id": "name", "width": 260, "visible": True}]}
    if cid == "project.columns.resize":
        return {"id": "name", "width": 300}
    if cid == "project.remove":
        return {"media_ids": [x for x in (ctx.spare_media_id,) if x]}
    if cid == "project.freeform.resize":
        return {"media_ids": [x for x in (ctx.media_id,) if x], "scale": 1.0}
    if cid == "project.freeform.move":
        return {"media_ids": [x for x in (ctx.media_id,) if x], "dx": 0, "dy": 0}
    if cid == "project.relink_media":
        return {"media_id": ctx.media_id, "path": str(ctx.fixture)}
    if cid in ("project.freeform.stack", "project.freeform.unstack"):
        return {"media_ids": [x for x in (ctx.spare_media_id,) if x]}
    if cid in ("media.attach_proxy", "media.detach_proxy"):
        return {"media_id": ctx.media_id, "path": str(ctx.fixture)}
    if cid == "media.auto_relink":
        return {"search_dirs": [str(ctx.fixture.parent)]}
    if cid == "media.create_bin":
        return {"name": "Shots2"}
    if cid == "media.rename_bin":
        return {"bin_id": ctx.bin_id or 1, "name": "ShotsX"}
    if cid == "media.delete_bin":
        return {"bin_id": ctx.bin_id or 1}
    if cid == "export.presets.save":
        return {"name": "Matrix2", "settings": {"format": "webm"}}
    if cid == "export.presets.import":
        return {"presets": {"Imported": {"format": "webm", "width": 1280, "height": 720}}}
    if cid == "export.presets.favorite":
        return {"name": "Matrix", "favorite": True}
    if cid == "export.presets.delete":
        return {"name": "Matrix"}
    if cid == "export.presets.export":
        return {"path": str(ctx.tmp / "export-presets.json")}
    if cid in ("interop.import_fcpxml", "interop.import_xmeml_xml"):
        return {"xml": ctx.export_xml or ""}
    if cid == "interop.import_otio":
        return {"content": OTIO_SAMPLE}
    if cid == "file.open":
        return {"path": ctx.saved_project}
    if cid in ("sequence.lift", "sequence.extract", "timeline.ripple_delete_ranges"):
        return {"ranges": [[ticks(0.2), ticks(0.8)]]}
    if cid == "sequence.delete":
        return {"sequence_id": ctx.seq2_id} if ctx.seq2_id else {}
    if cid == "speech.transcribe_to_subtitles":
        return {"segments": [{"id": 1, "start_ticks": ticks(0.2),
                              "end_ticks": ticks(0.8), "text": "Hi"}],
                "replace_existing": True}
    if cid == "speech.ripple_delete_silence":
        return {"peaks": [10, 220, 10, 220, 10, 220, 10]}
    if cid == "speech.ripple_delete_fillers":
        return {"segments": [{"id": 1, "start_ticks": ticks(0.2),
                              "end_ticks": ticks(0.8), "text": "um hi"}],
                "custom_fillers": ["um"]}
    if cid == "shortcuts.resolve":
        # keep = command id that must already be bound to key (hook binds it)
        return {"key": "ctrl+shift+z", "keep": "edit.undo"}
    if cid == "shortcuts.export":
        return {"path": str(ctx.tmp / "shortcuts.json")}
    if cid == "bus.set_gain_db":
        return {"target": "master", "gain_db": -3.0}
    if cid == "track.set_bus_route":
        return {"track_id": ctx.a_track, "bus": "master"}
    if cid == "track.set_automation":
        return {"track_id": ctx.a_track, "automation": {"mode": "read"}}
    if cid.startswith("track."):
        return {"track_id": ctx.a_track, "gain_db": -3.0,
                "pan": 1.0, "muted": True, "solo": True,
                "eq_enabled": True, "comp_enabled": True, "denoise_enabled": True,
                "eq_low_gain": 1.0, "eq_mid_gain": 1.0, "eq_high_gain": 1.0,
                "comp_threshold": -20.0, "comp_ratio": 4.0, "comp_attack": 5.0,
                "comp_release": 100.0, "denoise_amount": 0.5, "mode": "read",
                "interpolation": "linear", "lane": "volume", "time": ticks(0.5),
                "value": 1.0, "locked": True}
    return None


def build_params(cmd: dict, ctx: Context, tmp: Path):
    cid = cmd["id"]
    override = O(ctx, cid)
    if override is not None:
        return override
    return generic_params(cmd, ctx, tmp)


# ------------------------------------------------------------------ hooks

HOOKS: dict[str, list] = {}
for _trim in ("trim.backward", "trim.backwardMany"):
    HOOKS[_trim] = [("trim.selectEditPoint",
                     lambda c: {"track_id": c.v_track, "clip_index": 0,
                                "edge": "end"})]
for _trim in ("trim.edit", "trim.forward", "trim.forwardMany", "trim.nudge",
              "trim.extendToPlayhead", "trim.inspect"):
    HOOKS[_trim] = [("trim.selectEditPoint",
                     lambda c: {"track_id": c.v_track, "clip_index": 0,
                                "edge": "start"})]
# out-splits need the matching in-split set first (must land after the in)
HOOKS["marker.set_split_audio_out"] = [("marker.set_split_audio",
                                        lambda c: {"id": c.marker_id,
                                                   "at": ticks(0.5)})]
HOOKS["marker.set_split_video_out"] = [("marker.set_split_video",
                                        lambda c: {"id": c.marker_id,
                                                   "at": ticks(0.6)})]
HOOKS["nodegraph.update_node"] = [("nodegraph.add_node",
                                   lambda c: {"track_id": c.v_track,
                                              "clip_index": 0,
                                              "kind": "merge",
                                              "title": "Node"})]
HOOKS["media.detach_proxy"] = [("media.attach_proxy",
                                lambda c: {"media_id": c.media_id,
                                           "path": str(c.fixture)})]
for _tgt in ("timeline.toggleMuteTargetedAudio", "timeline.toggleSoloTargetedAudio"):
    # explicit targeting set - a toggle hook would flip state off if the
    # track was already targeted
    HOOKS[_tgt] = [("timeline.setTargeting", lambda c: {"audio": [1]})]
HOOKS["timeline.toggleOutputTargetedVideo"] = [("timeline.setTargeting",
                                                lambda c: {"video": [1]})]
HOOKS["shortcuts.resolve"] = [("shortcuts.set",
                               lambda c: {"shortcuts": {"edit.undo": "ctrl+shift+z"}})]
_MULTICAM = ("multicam.create",
             lambda c: {"name": "MC",
                        "angles": [{"id": 1, "name": "A", "media_id": c.media_id},
                                   {"id": 2, "name": "B",
                                    "media_id": c.spare_media_id or c.media_id}],
                        "sync_mode": "timecode"})
for _mc in ("multicam.switch_angle", "multicam.sync", "multicam.flatten"):
    HOOKS[_mc] = [_MULTICAM]
_COMPOUND = ("timeline.create_compound_clip",
             lambda c: {"name": "Compound",
                        "clips": [c.clip_ref(c.v_track, 0), c.clip_ref(c.v2_track, 0)]})
for _cp in ("timeline.decompose_compound_clip", "sequence.reveal_nested"):
    HOOKS[_cp] = [_COMPOUND]
# edit.redo needs an undone entry to redo
HOOKS["edit.redo"] = [("edit.undo", lambda c: {})]
HOOKS["marker.add_range_in_out"] = [
    ("timeline.set_mark_in", lambda c: {"at": ticks(0.5)}),
    ("timeline.set_mark_out", lambda c: {"at": ticks(1.5)}),
]
# the delete test may have removed the bin (settings-like store, undo-blind)
HOOKS["project.searchBin.items"] = [("project.searchBin.set",
                                     lambda c: {"name": "Matrix", "query": "shot"})]

# document-replacing or history-exiting commands: rebuild baseline afterwards
RESET_AFTER = {"file.open", "project.new", "project.switch_sequence",
               "sequence.create", "export.presets.delete",
               "project.viewPreset.delete", "project.viewPreset.rename",
               "interop.import_fcpxml", "interop.import_xmeml_xml",
               "interop.import_otio"}
# mutating commands that only touch the settings store (no document change,
# no undo entry): skip the undo and never count them as mutating hooks
SETTINGS_NO_UNDO = {"prefs.set", "prefs.reset",
                    "export.presets.save", "export.presets.delete",
                    "export.presets.favorite", "export.presets.import",
                    "project.viewPreset.save", "project.viewPreset.saveAs",
                    "project.viewPreset.restore", "project.viewPreset.delete",
                    "project.viewPreset.rename"}
SKIP_UNDO = RESET_AFTER | {"file.save", "edit.undo", "edit.redo"} | SETTINGS_NO_UNDO


# ------------------------------------------------------------------- main

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", required=True)
    ap.add_argument("--fixture", required=True)
    ap.add_argument("--work", required=True, type=Path)
    ap.add_argument("--json-out", required=True, type=Path)
    ap.add_argument("--user-tag", default="matrix")
    ap.add_argument("--limit", type=int, default=0, help="run only the first N commands")
    ap.add_argument("--only", default="",
                    help="comma-separated command ids to run (after baseline)")
    args = ap.parse_args()

    work = args.work.resolve()
    work.mkdir(parents=True, exist_ok=True)
    fixture = Path(args.fixture).resolve()

    ffprobe = subprocess.run(
        ["ffprobe", "-v", "error", "-print_format", "json",
         "-show_streams", "-show_format", str(fixture)],
        capture_output=True, text=True)
    probe = json.loads(ffprobe.stdout)
    vs = next(s for s in probe["streams"] if s.get("codec_type") == "video")
    num, _, den = vs["avg_frame_rate"].partition("/")
    meta = {"duration": float(probe["format"]["duration"]),
            "width": vs["width"], "height": vs["height"],
            "fps": round(int(num) / float(den or 1), 6),
            "codec": vs.get("codec_name", "vp9")}

    # -- launch stack -------------------------------------------------------
    xdisplay = ":99"
    subprocess.run(["rm", "-f", f"/tmp/.X{xdisplay[1:]}-lock",
                    f"/tmp/.X11-unix/X{xdisplay[1:]}"])
    xvfb = subprocess.Popen(["Xvfb", xdisplay, "-screen", "0", "1600x900x24",
                             "-nolisten", "tcp"])
    time.sleep(1.5)
    vite = subprocess.Popen(
        ["pnpm", "dev", "--host", "127.0.0.1", "--strictPort"],
        cwd=Path(__file__).resolve().parents[2] / "apps/saturn-tauri",
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, preexec_fn=os.setsid)
    up = False
    for _ in range(150):
        try:
            with urllib.request.urlopen("http://127.0.0.1:1420/", timeout=1) as resp:
                if resp.status == 200:
                    up = True
                    break
        except OSError:
            time.sleep(0.4)
    if not up:
        print("[matrix] vite dev server did not come up")
        for p in (xvfb, vite):
            p.terminate()
        return 2

    env = dict(os.environ)
    env.update({
        "DISPLAY": xdisplay, "HOME": str(work / "home"),
        "XDG_CONFIG_HOME": str(work / "home" / ".config"),
        "XDG_DATA_HOME": str(work / "home" / ".local" / "share"),
        "XDG_CACHE_HOME": str(work / "home" / ".cache"),
        "XDG_RUNTIME_DIR": str(work / "xdg-runtime"),
        "USER": args.user_tag,
        "LIBGL_ALWAYS_SOFTWARE": "1",
        "WEBKIT_DISABLE_DMABUF_RENDERER": "1",
        "RUST_BACKTRACE": "1",
    })
    for key in ("HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_RUNTIME_DIR"):
        os.makedirs(env[key], mode=0o700, exist_ok=True)
    app_err = (work / "app-stderr.log").open("wb")
    app = subprocess.Popen([args.binary], env=env,
                           stdout=subprocess.DEVNULL, stderr=app_err)

    sock_path = Path(env["XDG_RUNTIME_DIR"]) / "easy-edits-pro-mcp.sock"
    sock_ok = False
    for _ in range(150):
        if sock_path.exists():
            ok, _ = bridge_call(sock_path, "command_list")
            if ok:
                sock_ok = True
                break
        if app.poll() is not None:
            print(f"[matrix] app exited early rc={app.returncode}")
            break
        time.sleep(0.4)
    if not sock_ok:
        print("[matrix] app socket never became ready; app-stderr tail:")
        print((work / "app-stderr.log").read_text(errors="replace")[-1500:])
        for p in (xvfb, vite, app):
            p.terminate()
        return 2

    _, listing = bridge_call(sock_path, "command_list")
    bridge_commands = listing if isinstance(listing, list) else listing.get("commands", [])
    catalogue = {
        c["id"]: c for c in json.loads(
            (Path(__file__).resolve().parents[2]
             / "docs/reference/command-catalogue.json").read_text())
    }
    commands = [{**catalogue.get(c.get("id"), {}), **c} for c in bridge_commands]
    shipped = [c for c in commands if c.get("shipped", True)]
    if args.only:
        wanted = [x.strip() for x in args.only.split(",") if x.strip()]
        present = {c["id"] for c in shipped}
        missing = [w for w in wanted if w not in present]
        if missing:
            print(f"[matrix] not in catalogue: {missing}")
        shipped = [c for c in shipped if c["id"] in wanted]
    shipped = shipped[: args.limit] if args.limit else shipped
    print(f"[matrix] bridge lists {len(bridge_commands)} commands; "
          f"exercising {len(shipped)}")

    stats = {"resets": 0, "ok": 0, "ui_only": 0, "error": 0, "undos": 0,
             "undo_drift": 0}
    drift_ids: list[str] = []
    per_menu: dict[str, dict[str, int]] = {}
    records = []
    ctx = Context(fixture, work)
    reset_to_baseline(ctx, sock_path, meta, work, stats)
    good_state = state_signature(ctx)

    def record(cmd, params, cls, result):
        menu = cmd.get("menu", [""])[0] if cmd.get("menu") else ""
        records.append({
            "id": cmd["id"], "menu": menu, "category": cmd.get("category"),
            "mutating": cmd.get("mutating"), "params": params, "class": cls,
            "result": (json.dumps(result)[:260] if cls == "ok" else str(result)[:260]),
        })
        bucket = per_menu.setdefault(menu, {"ok": 0, "ui_only": 0, "error": 0})
        bucket[cls] += 1

    try:
        for i, cmd in enumerate(shipped):
            cid = cmd["id"]

            # hooks first (editor-held state the command requires)
            mut_hooks = 0
            for hook_cmd, hook_params_fn in HOOKS.get(cid, []):
                hok, hres = run_cmd(sock_path, hook_cmd, hook_params_fn(ctx))
                if not hok:
                    print(f"[matrix] hook failed: {hook_cmd} for {cid}: {str(hres)[:90]}")
                if hok:
                    capture_result(ctx, hook_cmd, hres)
                    mutating = catalogue.get(hook_cmd, {}).get("mutating", False)
                    if mutating and hook_cmd not in SETTINGS_NO_UNDO:
                        mut_hooks += 1

            params = build_params(cmd, ctx, work)
            okk, result = run_cmd(sock_path, cid, params, timeout=30)
            if okk:
                cls = "ok"
                stats["ok"] += 1
            elif "handled by the interface" in str(result):
                cls = "ui_only"
                stats["ui_only"] += 1
            else:
                cls = "error"
                stats["error"] += 1
            record(cmd, params, cls, result)
            if cls in ("ok", "error") and "bridge I/O" in str(result):
                print(f"[matrix] bridge died at {cid}; stopping")
                break

            # -- restore the baseline -------------------------------
            if okk and cid in SKIP_UNDO:
                undoes = mut_hooks
            elif okk and cmd.get("mutating"):
                undoes = 1 + mut_hooks
            else:
                undoes = mut_hooks
            broken = False
            for _ in range(undoes):
                uok, _ = run_cmd(sock_path, "edit.undo")
                stats["undos"] += 1
                if not uok:
                    broken = True
                    break
            if not broken and undoes and cid not in RESET_AFTER:
                # sentinel: an undo whose result no longer matches the
                # baseline signature means the command never pushed an undo
                # entry and we popped baseline state instead
                refresh_ctx(ctx, sock_path)
                if state_signature(ctx) != good_state:
                    broken = True
                    stats["undo_drift"] += 1
                    drift_ids.append(cid)
            if broken or (okk and cid in RESET_AFTER):
                reset_to_baseline(ctx, sock_path, meta, work, stats)
                good_state = state_signature(ctx)
            else:
                refresh_ctx(ctx, sock_path)

            if (i + 1) % 50 == 0:
                print(f"[matrix] progress {i + 1}/{len(shipped)} ok={stats['ok']} "
                      f"ui_only={stats['ui_only']} err={stats['error']}")
    except KeyboardInterrupt:
        print("[matrix] interrupted")
    finally:
        for p in (xvfb, vite, app):
            try:
                p.terminate()
            except Exception:
                pass
        try:
            app_err.close()
        except Exception:
            pass

    args.json_out.write_text(json.dumps(
        {"summary": stats, "per_menu": per_menu, "undo_drift_ids": drift_ids,
         "records": records}, indent=1))
    print(f"[matrix] DONE ok={stats['ok']} ui_only={stats['ui_only']} "
          f"error={stats['error']} resets={stats['resets']} "
          f"undos={stats['undos']} -> {args.json_out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
