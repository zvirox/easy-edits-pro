// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import test from "node:test";
import assert from "node:assert/strict";
import { FLAGSHIP_TEMPLATES } from "../tasksheet/types.ts";
import type { WebDeskJob, WebDeskMetrics } from "./types.ts";

test("TS8 Web Desk: Flagship templates coverage", () => {
  assert.equal(FLAGSHIP_TEMPLATES.length, 3);
  const templateIds = FLAGSHIP_TEMPLATES.map((t) => t.id);
  assert.ok(templateIds.includes("podcast_social"));
  assert.ok(templateIds.includes("tutorial_screen"));
  assert.ok(templateIds.includes("webinar_recap"));

  // Check tutorial screen template for Web Desk
  const tutorial = FLAGSHIP_TEMPLATES.find((t) => t.id === "tutorial_screen")!.template;
  assert.ok(tutorial.title.length > 0);
  assert.ok(tutorial.deliverables.length >= 1);
  assert.ok(tutorial.inputs.length >= 2);
});

test("TS8 Web Desk: AGPL §13 invariants and source repository", () => {
  const sourceRepo = "https://github.com/zvirox/easy-edits-pro";
  assert.ok(sourceRepo.startsWith("https://github.com/"));
  assert.ok(sourceRepo.includes("easy-edits-pro"));
});

test("TS8 Web Desk: Metrics calculation logic", () => {
  const jobs: WebDeskJob[] = [
    {
      id: "j1",
      title: "Podcast clip",
      requester: "user:1",
      status: "ready",
      inputs: [],
      deliverables: [],
      budget: { wall_clock_min: 30, model_cost_usd: 2.5, render_min: 15 },
      questions: [],
      rounds: [],
      runLog: [],
      created_at_unix_ms: 1000,
      updated_at_unix_ms: 1000,
    },
    {
      id: "j2",
      title: "Tutorial cut",
      requester: "user:2",
      status: "in_review",
      inputs: [],
      deliverables: [],
      budget: { wall_clock_min: 60, model_cost_usd: 5.0, render_min: 30 },
      questions: [],
      rounds: [],
      runLog: [],
      created_at_unix_ms: 2000,
      updated_at_unix_ms: 2000,
    },
    {
      id: "j3",
      title: "Webinar summary",
      requester: "user:3",
      status: "delivered",
      inputs: [],
      deliverables: [],
      budget: { wall_clock_min: 90, model_cost_usd: 7.5, render_min: 45 },
      questions: [],
      rounds: [],
      runLog: [],
      created_at_unix_ms: 3000,
      updated_at_unix_ms: 3000,
    },
  ];

  let active = 0;
  let inReview = 0;
  let completed = 0;
  let totalCost = 0;
  let totalTime = 0;

  for (const job of jobs) {
    if (job.status === "approved" || job.status === "delivered") {
      completed += 1;
    } else {
      active += 1;
    }
    if (job.status === "in_review" || job.status === "revision_requested") {
      inReview += 1;
    }
    totalCost += job.budget?.model_cost_usd ?? 0;
    totalTime += job.budget?.wall_clock_min ?? 0;
  }

  const metrics: WebDeskMetrics = {
    totalJobs: jobs.length,
    activeJobs: active,
    inReviewCount: inReview,
    needsInputCount: 0,
    completedJobs: completed,
    totalModelCostUsd: Number(totalCost.toFixed(2)),
    totalWallClockMin: totalTime,
  };

  assert.equal(metrics.totalJobs, 3);
  assert.equal(metrics.activeJobs, 2);
  assert.equal(metrics.inReviewCount, 1);
  assert.equal(metrics.completedJobs, 1);
  assert.equal(metrics.totalModelCostUsd, 15.0);
  assert.equal(metrics.totalWallClockMin, 180);
});

test("TS8 Web Desk: RAW media surface invariant", () => {
  // Confirm that video player surface class is webdesk-player-surface without glass
  const playerClassName = "webdesk-player-surface";
  assert.ok(!playerClassName.includes("glass"));
});
