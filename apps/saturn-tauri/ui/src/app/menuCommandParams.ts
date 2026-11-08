// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { TICKS_PER_SECOND } from "./projectTypes.ts";
import type { ProjectDocument, SelectedClip } from "./projectTypes.ts";

export interface BatchMenuContext {
  selectedClip: SelectedClip | null;
  selectedClips: SelectedClip[];
  playhead: number;
  project: ProjectDocument["project"] | null;
}

export interface MenuBatchResult {
  params?: Record<string, unknown> | null;
  notice?: string;
  cancel?: boolean;
}

/**
 * Parameter builders for the batch/range timeline menu commands
 * (ripple-delete batch/ranges, batch move, track-folder assign/delete).
 * Returns null when `id` is not one of these commands; `cancel: true` aborts
 * the dispatch after showing the notice. Prompts follow the app's native
 * prompt/confirm idiom.
 */
export function buildBatchCommandParams(
  id: string,
  ctx: BatchMenuContext
): MenuBatchResult | null {
  switch (id) {
    case "timeline.ripple_delete_batch": {
      const refs = [
        ...(ctx.selectedClips.length
          ? ctx.selectedClips
          : ctx.selectedClip
            ? [ctx.selectedClip]
            : []),
      ];
      if (refs.length === 0) {
        return { notice: "Select one or more timeline clips first", cancel: true };
      }
      return {
        params: {
          clips: refs.map((c) => ({ track_id: c.track_id, clip_index: c.clip_index })),
          ripple: true,
        },
      };
    }
    case "timeline.ripple_delete_ranges": {
      const seq = ctx.project?.sequence;
      if (seq?.mark_in != null && seq?.mark_out != null && seq.mark_out > seq.mark_in) {
        return { params: { ranges: [[seq.mark_in, seq.mark_out]] } };
      }
      const input = window.prompt(
        "Ripple-delete time range (milliseconds, start end):",
        "1000 2000"
      );
      if (!input) return { cancel: true };
      const parts = input
        .trim()
        .split(/[\s,]+/)
        .map(Number)
        .filter((n) => Number.isFinite(n));
      if (parts.length < 2 || parts[1] <= parts[0]) return { cancel: true };
      return { params: { ranges: [[parts[0], parts[1]]] } };
    }
    case "timeline.move_clips_batch": {
      const refs = ctx.selectedClips.length
        ? ctx.selectedClips
        : ctx.selectedClip
          ? [ctx.selectedClip]
          : [];
      if (refs.length === 0) {
        return { notice: "Select one or more timeline clips first", cancel: true };
      }
      const tracks = ctx.project?.sequence.tracks ?? [];
      const placed = refs
        .map((ref) => {
          const clip = tracks.find((t) => t.id === ref.track_id)?.clips[ref.clip_index];
          return clip ? { ref, start: clip.timeline_start } : null;
        })
        .filter((x): x is { ref: SelectedClip; start: number } => x !== null);
      if (placed.length === 0) return { notice: "Selected clips are gone", cancel: true };
      const earliest = Math.min(...placed.map((x) => x.start));
      const target = Math.round(ctx.playhead * TICKS_PER_SECOND);
      return {
        params: {
          moves: placed.map(({ ref, start }) => ({
            track_id: ref.track_id,
            clip_index: ref.clip_index,
            timeline_start: Math.max(0, target + (start - earliest)),
          })),
        },
      };
    }
    case "timeline.assign_track_group": {
      const groups = ctx.project?.sequence.track_groups ?? [];
      const list = groups.map((g) => `${g.id}: ${g.name}`).join(", ") || "none yet";
      const input = window.prompt(
        `Assign track to folder — enter "trackId groupId" (groupId 0 removes). Folders: ${list}`,
        "1 1"
      );
      if (!input) return { cancel: true };
      const parts = input
        .trim()
        .split(/[\s,]+/)
        .map(Number)
        .filter((n) => Number.isFinite(n));
      if (parts.length < 2) return { cancel: true };
      return { params: { track_id: parts[0], group_id: parts[1] > 0 ? parts[1] : null } };
    }
    case "timeline.delete_track_group": {
      const groups = ctx.project?.sequence.track_groups ?? [];
      const list = groups.map((g) => `${g.id}: ${g.name}`).join(", ") || "none yet";
      const input = window.prompt(`Delete folder — enter groupId. Folders: ${list}`);
      if (!input) return { cancel: true };
      const gid = Number(input.trim());
      if (!Number.isFinite(gid)) return { cancel: true };
      return { params: { group_id: gid } };
    }
    default:
      return null;
  }
}

/**
 * Parameter builders for the mark commands that act on the current timeline
 * selection (Mark Clip / Mark Selection).
 */
export function buildMarkerCommandParams(
  id: string,
  ctx: BatchMenuContext
): MenuBatchResult | null {
  switch (id) {
    case "timeline.mark_clip": {
      const selected = ctx.selectedClip;
      if (!selected) {
        return { notice: "Select a timeline clip to mark it", cancel: true };
      }
      return {
        params: { track_id: selected.track_id, clip_index: selected.clip_index },
      };
    }
    case "timeline.mark_selection": {
      const refs = [
        ...(ctx.selectedClips.length
          ? ctx.selectedClips
          : ctx.selectedClip
            ? [ctx.selectedClip]
            : []),
      ];
      if (refs.length === 0) {
        return { notice: "Select one or more timeline clips to mark them", cancel: true };
      }
      return {
        params: {
          clips: refs.map((c) => ({ track_id: c.track_id, clip_index: c.clip_index })),
        },
      };
    }
    default:
      return null;
  }
}

/**
 * Parameter builders for the engine-backed trim commands. Menu dispatch only
 * carries a command id, so the trim family fills from the current selection
 * and playhead, with native prompts for the frame-count inputs.
 */
export function buildTrimCommandParams(
  id: string,
  ctx: BatchMenuContext
): MenuBatchResult | null {
  switch (id) {
    case "edit.match_cut_align": {
      const selected = ctx.selectedClip;
      const track = ctx.project?.sequence.tracks.find((t) => t.id === selected?.track_id);
      const incoming = selected && track?.clips[selected.clip_index];
      if (!selected || !track || !incoming) {
        return { notice: "Select the incoming clip of an adjacent pair", cancel: true };
      }
      const predecessors = track.clips.flatMap((clip, index) =>
        index !== selected.clip_index && clip.timeline_start + clip.duration === incoming.timeline_start ? [index] : []);
      if (predecessors.length !== 1) {
        return { notice: "The incoming clip needs one adjacent outgoing clip on the same track", cancel: true };
      }
      const input = window.prompt(
        "Match mode: eye_trace (contrast-centroid heuristic, not gaze), motion (block displacement), or transient (audio attack). Slips incoming source only; align before detaching sound.",
        "eye_trace",
      );
      if (input === null) return { cancel: true };
      const mode = input.trim().toLowerCase();
      if (!["eye_trace", "motion", "transient"].includes(mode)) {
        return { notice: "Enter eye_trace, motion, or transient", cancel: true };
      }
      const search = window.prompt("Search frames on each side (1 to 30; at most two seconds per side)", "12");
      if (search === null) return { cancel: true };
      const searchFrames = Number(search.trim());
      if (!Number.isInteger(searchFrames) || searchFrames < 1 || searchFrames > 30) {
        return { notice: "Enter a search range from 1 to 30 frames", cancel: true };
      }
      return { params: { track_id: selected.track_id, left_clip_index: predecessors[0],
        right_clip_index: selected.clip_index, mode, search_frames: searchFrames } };
    }
    case "timeline.asymmetric_trim": {
      const selected = ctx.selectedClip;
      if (!selected) return { notice: "Select the incoming video clip of an adjacent pair", cancel: true };
      const values: number[] = [];
      for (const [label, initial] of [
        ["Picture cut offset in seconds from the original cut (negative = earlier)", "0"],
        ["Sound cut offset in seconds from the original cut (earlier than picture = J-cut; later = L-cut)", "-0.25"],
        ["Audio crossfade duration in seconds, centered on the sound cut (0 to 5)", "0.04"],
      ]) {
        const input = window.prompt(label, initial);
        if (input === null) return { cancel: true };
        const seconds = Number(input.trim());
        const ticks = Math.round(seconds * TICKS_PER_SECOND);
        if (!input.trim() || !Number.isFinite(seconds) || !Number.isSafeInteger(ticks)
          || (values.length === 2 && (seconds < 0 || seconds > 5))) {
          return { notice: "Enter a finite offset in seconds and a fade between 0 and 5 seconds", cancel: true };
        }
        values.push(ticks);
      }
      return { params: { track_id: selected.track_id, clip_index: selected.clip_index,
        video_delta: values[0], audio_delta: values[1], fade_ticks: values[2] } };
    }
    case "trim.selectEditPoint": {
      const selected = ctx.selectedClip;
      if (!selected) {
        return { notice: "Select a clip to choose its edit point", cancel: true };
      }
      const edge = window.prompt("Select edit point edge - start or end", "end");
      if (edge === null) return { cancel: true };
      const normalized = edge.trim().toLowerCase();
      if (normalized !== "start" && normalized !== "end") {
        return { notice: "Enter start or end", cancel: true };
      }
      return {
        params: {
          track_id: selected.track_id,
          clip_index: selected.clip_index,
          edge: normalized,
        },
      };
    }
    case "trim.extendToPlayhead":
    case "trim.extendNextEdit":
    case "trim.extendPreviousEdit":
    case "trim.rippleNext":
    case "trim.ripplePrevious":
    case "trim.next":
    case "trim.previous":
    case "trim.selectNearest":
    case "trim.monitor":
      return { params: { at: Math.round(ctx.playhead * TICKS_PER_SECOND) } };
    case "trim.nudge": {
      const input = window.prompt("Nudge trim by signed frames", "-1");
      if (input === null) return { cancel: true };
      const frames = Number(input.trim());
      if (!Number.isInteger(frames)) {
        return { notice: "Enter a signed integer frame count", cancel: true };
      }
      return { params: { frames } };
    }
    case "trim.edit":
    case "trim.forward":
    case "trim.backward":
    case "trim.forwardMany":
    case "trim.backwardMany":
    case "trim.toggleType":
    case "trim.clear":
    case "trim.applyDefaultTransition":
      return { params: {} };
    default:
      return null;
  }
}

/**
 * Parameter builders for the engine-backed Edit-menu commands (FC label
 * colours, the attribute clipboard, duplicate/repair commands). The ui_only
 * edit commands never reach here — dispatchCommand intercepts them first.
 * Returns null when `id` is not one of these commands.
 */
export function buildEditCommandParams(
  id: string,
  ctx: BatchMenuContext
): MenuBatchResult | null {
  if (id.startsWith("edit.label.")) {
    const refs = ctx.selectedClips.length
      ? ctx.selectedClips
      : ctx.selectedClip
        ? [ctx.selectedClip]
        : [];
    if (refs.length === 0) {
      return { notice: "Select one or more timeline clips first", cancel: true };
    }
    return {
      params: {
        clips: refs.map((c) => ({ track_id: c.track_id, clip_index: c.clip_index })),
      },
    };
  }
  switch (id) {
    case "edit.duplicate":
    case "edit.copy_attributes":
    case "edit.paste_attributes":
    case "edit.remove_attributes":
    case "edit.ripple_delete_head": {
      if (!ctx.selectedClip) {
        return { notice: "Select a timeline clip first", cancel: true };
      }
      return {
        params: {
          track_id: ctx.selectedClip.track_id,
          clip_index: ctx.selectedClip.clip_index,
        },
      };
    }
    case "clip.set_enabled":
    case "clip.set_volume_db":
    case "clip.frame_fit":
    case "clip.make_subclip":
    case "multicam.flatten": {
      if (!ctx.selectedClip) {
        return { notice: "Select a timeline clip first", cancel: true };
      }
      const params: Record<string, unknown> = {
        track_id: ctx.selectedClip.track_id,
        clip_index: ctx.selectedClip.clip_index,
      };
      if (id === "clip.set_volume_db") {
        const track = ctx.project?.sequence.tracks.find(
          (item) => item.id === ctx.selectedClip?.track_id,
        );
        const clip = track?.clips[ctx.selectedClip.clip_index];
        params.volume_db = clip?.volume_db ?? 0;
      }
      if (id === "clip.frame_fit") {
        params.mode = "fit";
      }
      return { params };
    }
    case "edit.consolidate_duplicates":
    case "edit.remove_unused_media":
      return { params: {} };
    default:
      return null;
  }
}
