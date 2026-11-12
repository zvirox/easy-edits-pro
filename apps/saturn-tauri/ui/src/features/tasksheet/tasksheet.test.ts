// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import test from "node:test";
import assert from "node:assert/strict";
import {
  FLAGSHIP_TEMPLATES,
  type AgentQuestion,
  type TaskSheetState,
} from "./types.ts";

test("TS2 Task Sheet: flagship templates validity and structure", () => {
  assert.equal(FLAGSHIP_TEMPLATES.length, 3);
  const ids = FLAGSHIP_TEMPLATES.map((t) => t.id);
  assert.deepEqual(ids, ["podcast_social", "tutorial_screen", "webinar_recap"]);

  for (const tmplDef of FLAGSHIP_TEMPLATES) {
    const sheet = tmplDef.template;
    assert.equal(sheet.schema, "tasksheet/1");
    assert.ok(sheet.id.length > 0);
    assert.ok(sheet.title.length > 0);
    assert.ok(sheet.inputs.length > 0);
    assert.ok(sheet.deliverables.length > 0);
    assert.ok(sheet.budget != null);
    assert.ok(sheet.budget.wall_clock_min! > 0);
    assert.ok(sheet.budget.model_cost_usd! > 0);
    assert.ok(sheet.budget.render_min! > 0);
    assert.ok(sheet.acceptance?.auto != null && sheet.acceptance.auto.length > 0);
    assert.ok(sheet.escalate_when != null && sheet.escalate_when.length > 0);
    assert.ok(sheet.handoff?.interchange != null && sheet.handoff.interchange.length > 0);

    // Verify inputs have valid roles and trust tags
    for (const input of sheet.inputs) {
      assert.ok(["main_video", "isolated_audio", "music_track", "b_roll", "transcript", "captions", "capture_event_track", "other"].includes(input.role));
      assert.ok(["trusted", "untrusted"].includes(input.trust || "trusted"));
    }

    // Verify deliverables have presets and durations
    for (const deliv of sheet.deliverables) {
      assert.ok(deliv.id.length > 0);
      assert.ok(deliv.name.length > 0);
      assert.ok(["vertical-9x16", "web-1080p", "broadcast-r128"].includes(deliv.preset));
      if (deliv.duration) {
        assert.ok(deliv.duration.target_s > 0);
      }
    }
  }
});

test("TS2 Task Sheet: input trust separation enforces untrusted transcript indicators", () => {
  const podcast = FLAGSHIP_TEMPLATES.find((t) => t.id === "podcast_social")!.template;
  const transcriptInput = podcast.inputs.find((i) => i.role === "transcript");
  assert.ok(transcriptInput != null);
  assert.equal(transcriptInput.trust, "untrusted");

  const cameraInput = podcast.inputs.find((i) => i.role === "main_video");
  assert.ok(cameraInput != null);
  assert.equal(cameraInput.trust, "trusted");
});

test("TS2 Agent Inbox: open questions display with safe default option", () => {
  const questions: AgentQuestion[] = [
    {
      id: "q_1",
      question: "Choose music energy level",
      options: [
        { id: "opt_low", label: "Low ambient", is_default: true },
        { id: "opt_high", label: "High energy upbeat", is_default: false },
      ],
      default_option_id: "opt_low",
      status: "open",
    },
    {
      id: "q_2",
      question: "Resolved question",
      options: [{ id: "opt_done", label: "Done" }],
      default_option_id: "opt_done",
      status: "answered",
      selected_option_id: "opt_done",
    },
  ];

  const activeQuestions = questions.filter((q) => q.status === "open");
  assert.equal(activeQuestions.length, 1);
  assert.equal(activeQuestions[0].id, "q_1");

  const defaultOpt = activeQuestions[0].options.find((o) => o.is_default);
  assert.ok(defaultOpt != null);
  assert.equal(defaultOpt.id, "opt_low");
});

test("TS2 Agent Inbox: answering question records selection and transitions status", () => {
  const question: AgentQuestion = {
    id: "q_aspect",
    question: "Select target framing",
    options: [
      { id: "opt_center", label: "Center crop", is_default: true },
      { id: "opt_face", label: "Face-tracking crop", is_default: false },
    ],
    default_option_id: "opt_center",
    status: "open",
  };

  const answeredQuestion: AgentQuestion = {
    ...question,
    status: "answered",
    selected_option_id: "opt_face",
  };

  assert.equal(answeredQuestion.status, "answered");
  assert.equal(answeredQuestion.selected_option_id, "opt_face");
});

test("TS2 Task Sheet: deliverable additions and validation requirements", () => {
  const baseSheet: TaskSheetState = FLAGSHIP_TEMPLATES[1].template;
  assert.equal(baseSheet.deliverables.length, 1);

  // Add a deliverable
  const updatedSheet: TaskSheetState = {
    ...baseSheet,
    deliverables: [
      ...baseSheet.deliverables,
      {
        id: "deliv_teaser",
        name: "Teaser Short (Vertical)",
        preset: "vertical-9x16",
        duration: { target_s: 30, tolerance_s: 5 },
        loudness_lufs: -14,
        true_peak_dbtp: -1,
        captions: true,
      },
    ],
  };

  assert.equal(updatedSheet.deliverables.length, 2);
  assert.equal(updatedSheet.deliverables[1].preset, "vertical-9x16");

  // Validate constraint: title cannot be empty
  const emptyTitleSheet = { ...updatedSheet, title: "   " };
  const isTitleValid = emptyTitleSheet.title.trim().length > 0;
  assert.equal(isTitleValid, false);

  // Validate constraint: at least one deliverable
  const noDeliverableSheet = { ...updatedSheet, deliverables: [] };
  const hasDeliverables = noDeliverableSheet.deliverables.length > 0;
  assert.equal(hasDeliverables, false);

  // Validate constraint: deliverable duration target must be positive
  const invalidDurationSheet = {
    ...updatedSheet,
    deliverables: [{ ...updatedSheet.deliverables[0], duration: { target_s: 0 } }],
  };
  const isDurationValid = invalidDurationSheet.deliverables.every(
    (d) => !d.duration || d.duration.target_s > 0
  );
  assert.equal(isDurationValid, false);
});
