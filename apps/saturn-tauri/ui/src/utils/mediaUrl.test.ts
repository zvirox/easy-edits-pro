// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import test from "node:test";
import assert from "node:assert/strict";
import { buildMediaStreamUrl } from "./mediaUrl.ts";

test("media stream URL encodes path and token", () => {
  assert.equal(
    buildMediaStreamUrl("/tmp/my clip.mp4", 4321, "a/b"),
    "http://127.0.0.1:4321/media?path=%2Ftmp%2Fmy%20clip.mp4&token=a%2Fb"
  );
});

test("media stream URL is unavailable without a path or server port", () => {
  assert.equal(buildMediaStreamUrl("/tmp/clip.mp4", 0, "token"), "");
  assert.equal(buildMediaStreamUrl("", 4321, "token"), "");
});
