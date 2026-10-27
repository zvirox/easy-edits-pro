import assert from "node:assert/strict";
import test from "node:test";
import { buildEditCommandParams } from "./menuCommandParams.ts";
import { LABEL_COLORS, clipLabelColor } from "../features/timeline/labelColors.ts";

const ctx = (overrides: Record<string, unknown> = {}) =>
  ({ selectedClip: null, selectedClips: [], playhead: 0, project: null, ...overrides }) as Parameters<
    typeof buildEditCommandParams
  >[1];

const clip = { track_id: 7, clip_index: 2 };

test("label builders fan the selection out as clip refs", () => {
  const result = buildEditCommandParams("edit.label.mango", ctx({ selectedClips: [clip] }));
  assert.deepEqual(result, {
    params: { clips: [{ track_id: 7, clip_index: 2 }] },
  });
  const none = buildEditCommandParams("edit.label.rose", ctx());
  assert.equal(none?.cancel, true);
});

test("every label colour maps through the catalogue and renderer", () => {
  assert.equal(LABEL_COLORS.length, 16);
  const names = LABEL_COLORS.map((entry) => entry.name);
  assert.equal(new Set(names).size, 16);
  for (const name of names) {
    const hex = clipLabelColor(name);
    assert.match(hex ?? "", /^#[0-9a-f]{6}$/i);
  }
  assert.equal(clipLabelColor(undefined), null);
  assert.equal(clipLabelColor(""), null);
  assert.equal(clipLabelColor("neon"), null);
});

test("single-clip edit commands target the primary selection", () => {
  for (const id of [
    "edit.duplicate",
    "edit.copy_attributes",
    "edit.paste_attributes",
    "edit.remove_attributes",
    "edit.ripple_delete_head",
  ]) {
    const built = buildEditCommandParams(id, ctx({ selectedClip: clip }));
    assert.deepEqual(built, { params: { track_id: 7, clip_index: 2 } }, id);
    const missing = buildEditCommandParams(id, ctx());
    assert.equal(missing?.cancel, true, id);
  }
});

test("repair commands run without params and unknown ids fall through", () => {
  for (const id of ["edit.consolidate_duplicates", "edit.remove_unused_media"]) {
    assert.deepEqual(buildEditCommandParams(id, ctx()), { params: {} });
  }
  assert.equal(buildEditCommandParams("edit.copy", ctx()), null);
  assert.equal(buildEditCommandParams("timeline.add_clip", ctx()), null);
});

test("clip volume builders read the current volume from the project", () => {
  const project = {
    sequence: {
      tracks: [
        { id: 7, clips: [{ volume_db: -3.5 }, { volume_db: 2 }, { volume_db: 0 }] },
      ],
    },
  };
  const volume = buildEditCommandParams("clip.set_volume_db", ctx({
    selectedClip: { track_id: 7, clip_index: 1 },
    project,
  }));
  assert.deepEqual(volume, {
    params: { track_id: 7, clip_index: 1, volume_db: 2 },
  });
  const fallback = buildEditCommandParams("clip.set_volume_db", ctx({
    selectedClip: { track_id: 4, clip_index: 0 },
  }));
  assert.deepEqual(fallback, {
    params: { track_id: 4, clip_index: 0, volume_db: 0 },
  });
  const missing = buildEditCommandParams("clip.set_volume_db", ctx());
  assert.equal(missing?.cancel, true);
});

test("clip action builders carry the selection and sensible defaults", () => {
  const sel = { track_id: 7, clip_index: 2 };
  assert.deepEqual(buildEditCommandParams("clip.frame_fit", ctx({ selectedClip: sel })), {
    params: { track_id: 7, clip_index: 2, mode: "fit" },
  });
  assert.deepEqual(buildEditCommandParams("clip.make_subclip", ctx({ selectedClip: sel })), {
    params: { track_id: 7, clip_index: 2 },
  });
  assert.deepEqual(buildEditCommandParams("multicam.flatten", ctx({ selectedClip: sel })), {
    params: { track_id: 7, clip_index: 2 },
  });
  for (const id of ["clip.frame_fit", "clip.make_subclip", "multicam.flatten"]) {
    assert.equal(buildEditCommandParams(id, ctx())?.cancel, true, id);
  }
});
