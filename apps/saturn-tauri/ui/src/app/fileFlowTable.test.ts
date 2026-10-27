import assert from "node:assert/strict";
import test from "node:test";
import {
  INTEROP_EXPORTS,
  FILE_FLOW_COMMANDS,
  SYNTHETIC_KINDS,
  isFileFlowCommand,
  recoveryNotice,
  sanitizeName,
  syntheticKindOf,
} from "./fileFlowTable.ts";

test("interchange export table covers the four interop formats", () => {
  assert.deepEqual(Object.keys(INTEROP_EXPORTS), [
    "interop.export_edl",
    "interop.export_fcpxml",
    "interop.export_xmeml_xml",
    "interop.export_otio",
  ]);
  assert.equal(INTEROP_EXPORTS["interop.export_edl"].extension, "edl");
  assert.equal(INTEROP_EXPORTS["interop.export_xmeml_xml"].extension, "xml");
});

test("file flow command list carries the FC save/close/recovery set", () => {
  for (const id of [
    "file.saveAs",
    "file.saveCopy",
    "file.revert",
    "file.close",
    "file.reveal",
    "export.frame",
    "project.autosave.now",
    "project.recovery.discard",
  ]) {
    assert.ok(isFileFlowCommand(id), id);
  }
  assert.equal(isFileFlowCommand("file.save"), false);
  assert.equal(isFileFlowCommand("project.new"), false);
  assert.equal(isFileFlowCommand("prefs.get"), false);
  assert.equal(FILE_FLOW_COMMANDS.length > 10, true);
});

test("sanitize name strips filesystem-hostile characters", () => {
  assert.equal(sanitizeName("My: Project *2026*"), "My- Project -2026-");
  assert.equal(sanitizeName("plain"), "plain");
});

test("synthetic kinds map to generate_synthetic_media kinds 1:1", () => {
  assert.deepEqual([...SYNTHETIC_KINDS], [
    "media.generate_black_video",
    "media.generate_color_matte",
    "media.generate_transparent_video",
    "media.generate_bars_and_tone",
  ]);
  assert.equal(syntheticKindOf("media.generate_color_matte"), "color_matte");
  assert.equal(syntheticKindOf("media.generate_bars_and_tone"), "bars_and_tone");
  assert.equal(syntheticKindOf("media.generate_unknown"), null);
  assert.equal(syntheticKindOf("media.search"), null);
  for (const id of SYNTHETIC_KINDS) {
    assert.ok(isFileFlowCommand(id), id);
  }
});

test("recovery notice formats both states", () => {
  assert.match(recoveryNotice({ available: false }), /No recovery snapshot/);
  const notice = recoveryNotice({
    available: true,
    projectName: "Crash documentary",
    savedAt: 1_760_000_000,
  });
  assert.match(notice, /"Crash documentary"/);
  assert.match(notice, /Recovery snapshot:/);
});
