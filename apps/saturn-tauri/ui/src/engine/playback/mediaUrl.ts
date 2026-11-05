// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import type { MediaAsset } from "./types.ts";
import { buildMediaStreamUrl } from "../../utils/mediaUrl.ts";

export function resolveMediaStreamUrl(
  mediaAsset: MediaAsset | null | undefined,
  preparedPreviews: Record<number, string>,
  mediaServerPort: number,
  mediaServerToken?: string
): string {
  if (!mediaAsset || mediaServerPort <= 0) return "";
  const targetPath = preparedPreviews[mediaAsset.id] || mediaAsset.path;
  return buildMediaStreamUrl(targetPath, mediaServerPort, mediaServerToken);
}

/**
 * Builds the complete dual-deck video and multi-track audio playback evaluation
 * for any given timeline playhead position.
 */
