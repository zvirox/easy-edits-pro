// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import type { Clip, MediaAsset } from "./projectTypes";
import { formatTime } from "../utils/formatters.ts";

export interface RevealTarget {
  /** The media asset to select in the project panel, or null when nothing
   * can be revealed. */
  media: MediaAsset | null;
  message: string;
}

/** Shared resolution for "reveal in project panel" style actions (media.reveal,
 * FC match frame): EEP has no source monitor, so the gesture is the panel
 * selection plus a notice carrying the playhead position. */
export function resolveRevealTarget(
  media: MediaAsset[],
  clip: Clip | null,
  playheadTicks: number,
): RevealTarget {
  const asset = clip
    ? (media.find((item) => item.id === clip.media_id) ?? null)
    : null;
  if (!asset) {
    return {
      media: null,
      message: "Select a timeline clip to reveal its media",
    };
  }
  return {
    media: asset,
    message: `Revealed "${asset.name}" in the project panel (playhead ${formatTime(playheadTicks)})`,
  };
}
