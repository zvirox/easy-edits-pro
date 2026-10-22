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
