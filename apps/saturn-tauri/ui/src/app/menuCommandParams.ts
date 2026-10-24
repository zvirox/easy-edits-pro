import { TICKS_PER_SECOND } from "./projectTypes";
import type { ProjectDocument, SelectedClip } from "./projectTypes";

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
