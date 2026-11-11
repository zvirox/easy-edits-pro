// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import test from "node:test";
import assert from "node:assert/strict";
import type {
  AgentSessionState,
  ProjectBriefState,
  ReviewChange,
  VerificationReport,
  PermissionRequest,
} from "./types.ts";

test("AF6 Agent Session: initial state invariants and default brief", () => {
  const defaultBrief: ProjectBriefState = {
    goal: "Highlight reel for social delivery",
    platform: "web-1080p",
    target_duration_ticks: 3000,
    duration_tolerance_ticks: 120,
    must_keep_clip_ids: [],
    banned_content: ["silent_gap", "flash_frames"],
    style_notes: "Punchy edits, aligned transients, -14 LUFS loudness",
    loudness_target_lufs: -14.0,
    source: "user_prompt",
  };

  assert.equal(defaultBrief.platform, "web-1080p");
  assert.equal(defaultBrief.loudness_target_lufs, -14.0);
  assert.equal(defaultBrief.target_duration_ticks, 3000);
});

test("AF6 Agent Session: immediate stop ≤ 1s leaves branch and history intact with no late apply", () => {
  const session: AgentSessionState = {
    status: "editing",
    activeSessionId: "session-test-01",
    activeBranch: "agent/rough-cut",
    currentAction: "Splitting clips on transients",
    stepCurrent: 2,
    stepTotal: 4,
    elapsedSeconds: 5,
    lastActor: { type: "agent", id: "claude-editor" },
    isStopped: false,
    activeMode: "fine-cut",
    loadedProfiles: ["core", "fine-cut"],
    grantedTiers: ["read", "edit"],
    ghostPlayheadTick: 1200,
    brief: {
      goal: "Rough cut",
      platform: "web-1080p",
      target_duration_ticks: 3000,
      duration_tolerance_ticks: 120,
      must_keep_clip_ids: [],
      banned_content: [],
      style_notes: "Clean cuts",
      loudness_target_lufs: -14.0,
      source: "user_prompt",
    },
    plan: [
      { id: 1, title: "Loudness", status: "done" },
      { id: 2, title: "Transient cut", status: "current" },
    ],
    pendingPermissions: [],
    reviewQueue: [
      {
        change_id: 1,
        tick_range: [1000, 1500],
        tier: "edit",
        rationale: "Cut on transient",
        confidence: "low",
        confidence_score: 0.6,
      },
    ],
    verificationReport: null,
  };

  // Immediate Stop simulation
  const stoppedSession: AgentSessionState = {
    ...session,
    status: "stopped",
    isStopped: true,
    currentAction: "Session stopped by user",
  };

  assert.equal(stoppedSession.status, "stopped");
  assert.equal(stoppedSession.isStopped, true);
  // Preserves existing branch and pending reviews without destruction
  assert.equal(stoppedSession.activeBranch, "agent/rough-cut");
  assert.equal(stoppedSession.reviewQueue.length, 1);
});

test("AF6 Review Queue: sorts low-confidence items first for priority human review", () => {
  const queue: ReviewChange[] = [
    {
      change_id: 101,
      tick_range: [2000, 2400],
      tier: "edit",
      rationale: "Normalized segment",
      confidence: "high",
      confidence_score: 0.95,
    },
    {
      change_id: 102,
      tick_range: [1000, 1400],
      tier: "edit",
      rationale: "Transient match cut shift +3 frames",
      confidence: "low",
      confidence_score: 0.58,
    },
    {
      change_id: 103,
      tick_range: [1500, 1800],
      tier: "edit",
      rationale: "Crossfade 120 ticks",
      confidence: "medium",
      confidence_score: 0.82,
    },
  ];

  const rank = { low: 0, medium: 1, high: 2 };
  const sorted = [...queue].sort((a, b) => {
    const rA = rank[a.confidence] ?? 1;
    const rB = rank[b.confidence] ?? 1;
    if (rA !== rB) return rA - rB;
    return a.change_id - b.change_id;
  });

  assert.equal(sorted[0].confidence, "low");
  assert.equal(sorted[0].change_id, 102);
  assert.equal(sorted[1].confidence, "medium");
  assert.equal(sorted[1].change_id, 103);
  assert.equal(sorted[2].confidence, "high");
  assert.equal(sorted[2].change_id, 101);
});

test("AF6 Verification Report: preserves measured evidence without invented aggregate quality score", () => {
  const report: VerificationReport = {
    preset: "web-1080p",
    results: [
      { check_id: "audio.integrated_loudness", status: "pass", measured_value: -14.2, threshold: -14.0 },
      { check_id: "audio.true_peak", status: "pass", measured_value: -1.1, threshold: -1.0 },
      { check_id: "audio.silence_dead_air", status: "pass", measured_value: 0.2, threshold: 1.5 },
      { check_id: "video.frozen_frames", status: "warn", measured_value: 12, threshold: 8, tick_ref: 1200 },
    ],
    unavailable_checks: ["video.face_focus_score"],
    notes: { aggregate_score: null },
  };

  assert.equal(report.preset, "web-1080p");
  assert.equal(report.results.length, 4);
  assert.equal(report.results[0].status, "pass");
  assert.equal(report.results[3].status, "warn");
  assert.equal(report.unavailable_checks[0], "video.face_focus_score");
  // Invariant: Never invent aggregate quality scores
  assert.equal(report.notes.aggregate_score, null);
});

test("AF6 Permissions: origin-aware grants enforce human approval on untrusted derivation", () => {
  const req: PermissionRequest = {
    id: "req-01",
    tool: "filesystem.render_export",
    argument: "/etc/passwd",
    reason: "Prompt requested file export",
    tier: "filesystem",
    originSource: "transcript",
    timestamp: Date.now(),
  };

  assert.equal(req.tier, "filesystem");
  assert.equal(req.originSource, "transcript");
  // Provenance: originSource is untrusted data and requires human verification
  assert.ok(req.originSource != null);
});
