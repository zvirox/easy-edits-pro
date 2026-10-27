import type { CaptionTrack, SubtitleItem } from "../types";

type SequenceLike = { caption_tracks?: CaptionTrack[] } | null | undefined;

/**
 * The default caption track: the first one, created lazily by the engine for
 * legacy `subtitle.*` commands and imports. `null` when the sequence has no
 * caption tracks yet. Accepts any sequence-like shape so every Sequence
 * variant in the app can use it.
 */
export function defaultCaptionTrack(sequence: SequenceLike): CaptionTrack | null {
  const tracks = sequence?.caption_tracks;
  if (!tracks || tracks.length === 0) return null;
  return tracks[0];
}

/** Cues of the default caption track; empty when there are none. */
export function defaultCaptionCues(sequence: SequenceLike): SubtitleItem[] {
  return defaultCaptionTrack(sequence)?.cues ?? [];
}

/**
 * The caption track id the editor should act on: the stored active track when
 * the live sequence still carries it, else the first track, else `null`.
 * The editor holds `activeTrackId` across sequence and document switches, so
 * the stored id can go stale; sending a stale id to the engine is harmful -
 * `caption.tracks_hide/show` match unknown ids against nothing, which turns
 * "solo this track" into "hide every track and show none".
 */
export function resolveCaptionTrackId(
  activeTrackId: number | null,
  tracks: Array<{ id: number }> | null | undefined,
): number | null {
  if (!tracks || tracks.length === 0) return null;
  if (activeTrackId != null && tracks.some((track) => track.id === activeTrackId)) {
    return activeTrackId;
  }
  return tracks[0].id;
}
