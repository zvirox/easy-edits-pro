import assert from "node:assert/strict";
import test from "node:test";
import { resolveRevealTarget } from "./revealMedia.ts";
import { openDockPanelCommand } from "./dockPanelCommands.ts";
import type { Clip, MediaAsset } from "./projectTypes.ts";

const asset = (id: number, name = `Asset ${id}`) =>
  ({ id, name }) as unknown as MediaAsset;
const clipOf = (mediaId: number) => ({ media_id: mediaId }) as unknown as Clip;

test("reveal resolves the selected clip's media and reports the playhead", () => {
  const media = [asset(1, "Interview A"), asset(2, "B-roll B")];
  const result = resolveRevealTarget(media, clipOf(2), 254016000000 * 65);
  assert.equal(result.media?.id, 2);
  assert.match(result.message, /Revealed "B-roll B"/);
  assert.match(result.message, /00:01:05/);
});

test("reveal without a resolvable clip only reports guidance", () => {
  const noClip = resolveRevealTarget([asset(1)], null, 0);
  assert.equal(noClip.media, null);
  assert.match(noClip.message, /Select a timeline clip/);

  const dangling = resolveRevealTarget([asset(1)], clipOf(9), 0);
  assert.equal(dangling.media, null);
  assert.match(dangling.message, /Select a timeline clip/);
});

test("dock panel command handles the three panel ids and nothing else", () => {
  const calls: string[] = [];
  const ops = {
    selectEditorWorkspace: (ws: string) => calls.push(`ws:${ws}`),
    openMarkerPanel: () => void calls.push("marker"),
    openCaptionPanel: () => void calls.push("caption"),
    openGraphicPanel: () => void calls.push("graphic"),
  };
  assert.equal(openDockPanelCommand("marker.panel", ops), true);
  assert.deepEqual(calls, ["ws:edit", "marker"]);

  calls.length = 0;
  assert.equal(openDockPanelCommand("caption.panel", ops), true);
  assert.deepEqual(calls, ["ws:edit", "caption"]);

  calls.length = 0;
  assert.equal(openDockPanelCommand("prefs.get", ops), false);
  assert.deepEqual(calls, []);
});
