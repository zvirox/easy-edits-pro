// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import assert from "node:assert/strict";
import test from "node:test";
import { evaluateTimelinePlayback } from "./scheduler.ts";
import type { ProjectDocument, MediaAsset } from "./types.ts";
import { TICKS_PER_SECOND as S } from "./constants.ts";

test("split-edit pictures stay visible while only the detached WAV is scheduled", () => {
  const clip = { media_id: 3, timeline_start: 0, source_in: 0, duration: S,
    audio_enabled: false, enabled: true, keyframes: [], color: {} };
  const media = [
    { id: 3, kind: "video", path: "/left.mkv", duration: 3 * S },
    { id: 4, kind: "video", path: "/right.mkv", duration: 3 * S },
    { id: 5, kind: "audio", path: "/split.wav", duration: 2 * S },
  ] as MediaAsset[];
  const document = { project: { settings: { width: 160, height: 90 }, sequences: [],
    sequence: { tracks: [
      { id: 1, kind: "video", gain_db: 0, clips: [clip, { ...clip, media_id: 4, timeline_start: S }] },
      { id: 2, kind: "audio", gain_db: -3, clips: [{ ...clip, media_id: 5, duration: 2 * S,
        audio_enabled: true, volume_db: -2 }] },
    ] } } } as unknown as ProjectDocument;
  for (const at of [0.5 * S, 1.5 * S]) {
    const result = evaluateTimelinePlayback(document, at, media, {}, 1234, "test");
    assert.equal(result.activeAudioClips.length, 1);
    assert.equal(result.activeAudioClips[0].media.id, 5);
    assert.equal(result.activeAudioClips[0].gainDb, -5);
    assert.ok([result.deckA, result.deckB].some(deck => deck.visible && deck.mediaId === (at < S ? 3 : 4)));
  }
  // Legacy clips lacking the field still schedule their attached source audio.
  delete (clip as { audio_enabled?: boolean }).audio_enabled;
  assert.equal(evaluateTimelinePlayback(document, S / 2, media, {}, 1234).activeAudioClips.length, 2);
});
