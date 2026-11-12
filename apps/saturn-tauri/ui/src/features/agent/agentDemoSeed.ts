// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Explicit `?demo=1` preview seed for Agent / Review / TaskSheet UI states.
 * Never loaded by default in normal app startup.
 */

import {
  FLAGSHIP_TEMPLATES,
  type AgentQuestion,
  type ReviewRound,
  type TaskSheetState,
} from "../tasksheet/types.ts";
import { ENGINE_TICKS_PER_SECOND } from "./reviewHelpers.ts";
import type {
  AgentSessionState,
  PlanStep,
  ProjectBriefState,
  ReviewChange,
  VerificationReport,
} from "./types.ts";

const SEC = ENGINE_TICKS_PER_SECOND;

export interface DemoSeedState {
  session: AgentSessionState;
  playheadTicks: number;
  selectedRoundNumber: number | "all";
  selectedNoteId: string | null;
}

export function createDemoAgentSnapshot(): DemoSeedState {
  const brief: ProjectBriefState = {
    goal: "Highlight reel for social delivery",
    platform: "web-1080p",
    target_duration_ticks: 45 * SEC,
    duration_tolerance_ticks: 2 * SEC,
    must_keep_clip_ids: [],
    banned_content: ["silent_gap", "flash_frames"],
    style_notes: "Punchy edits, aligned transients, -14 LUFS loudness",
    loudness_target_lufs: -14.0,
    source: "user_prompt",
  };

  const plan: PlanStep[] = [
    { id: 1, title: "Analyze audio loudness & silence gaps", status: "done", targetTick: 0 },
    { id: 2, title: "Align match cuts on video transients", status: "current", targetTick: 2 * SEC },
    { id: 3, title: "Quantize cuts to beat grid", status: "pending", targetTick: 4 * SEC },
    { id: 4, title: "Verify delivery presets & loudness targets", status: "pending", targetTick: 6 * SEC },
  ];

  const reviewQueue: ReviewChange[] = [
    {
      change_id: 101,
      step_id: 2,
      step_title: "Align match cuts on video transients",
      tick_range: [0, 2 * SEC],
      tier: "edit",
      rationale: "Shifted incoming cut by +3 frames to match snare transient peak",
      confidence: "low",
      confidence_score: 0.62,
      before_preview: "",
      after_preview: "",
    },
    {
      change_id: 102,
      step_id: 2,
      step_title: "Align match cuts on video transients",
      tick_range: [2 * SEC, 4 * SEC],
      tier: "edit",
      rationale: "Audio crossfade applied across cut point",
      confidence: "high",
      confidence_score: 0.94,
      before_preview: "",
      after_preview: "",
    },
  ];

  const verificationReport: VerificationReport = {
    preset: "web-1080p",
    results: [
      { check_id: "audio.integrated_loudness", status: "pass", measured_value: -14.2, threshold: -14.0 },
      { check_id: "audio.true_peak", status: "pass", measured_value: -1.1, threshold: -1.0 },
      { check_id: "audio.silence_dead_air", status: "pass", measured_value: 0.2, threshold: 1.5 },
      { check_id: "video.frozen_frames", status: "warn", measured_value: 12, threshold: 8, tick_ref: 2 * SEC, message: "Short pause on title slate" },
      { check_id: "project.brief_duration", status: "pass", measured_value: 45, threshold: 45 },
    ],
    unavailable_checks: ["video.face_focus_score"],
    notes: { aggregate_score: null },
  };

  const rounds: ReviewRound[] = [
    {
      round_number: 1,
      label: "v1",
      started_at_unix_ms: 1_762_900_000_000,
      completed_at_unix_ms: 1_762_900_060_000,
      opened_by: "agent:claude-editor",
      closed_by: "human:reviewer",
      outcome: "revision_requested",
      notes: [
        {
          id: "note_r1_1",
          author: "human:reviewer",
          created_at_unix_ms: 1_762_900_020_000,
          text: "Tighten pause before cold open hook",
          target_tick: 1 * SEC,
          status: "resolved",
          resolution: "Trimmed 14 frames of dead air before intro line",
          resolved_by: "agent:claude-editor",
          resolved_at_unix_ms: 1_762_900_090_000,
        },
      ],
    },
    {
      round_number: 2,
      label: "v2",
      started_at_unix_ms: 1_762_900_120_000,
      opened_by: "agent:claude-editor",
      diff_from_prior: {
        from_round: 1,
        to_round: 2,
        from_revision: 1,
        to_revision: 3,
        changed_ranges: [
          {
            start_tick: 1 * SEC,
            end_tick: 3 * SEC,
            kind: "clip_modified",
            entity_id: 101,
            track_id: 1,
            summary: "Trimmed cold-open pause and smoothed transient cut",
          },
        ],
        clip_changes: [
          {
            kind: "modified",
            clip_id: 101,
            track_id: 1,
            start_tick: 1 * SEC,
            end_tick: 3 * SEC,
            description: "Clip 101 trimmed by -14 frames on V1",
          },
        ],
        merged_tick_ranges: [[1 * SEC, 3 * SEC]],
        summary: "1 change(s) across 1 merged range(s) between v1 and v2",
      },
      notes: [
        {
          id: "note_r2_1",
          author: "human:reviewer",
          created_at_unix_ms: 1_762_900_140_000,
          text: "Verify audio crossfade smoothness at incoming transient",
          target_tick: 2 * SEC,
          end_tick: 3 * SEC,
          status: "open",
        },
      ],
    },
  ];

  const taskSheet: TaskSheetState = {
    ...JSON.parse(JSON.stringify(FLAGSHIP_TEMPLATES[0].template)),
    rounds,
  };

  const inbox: AgentQuestion[] = [
    {
      id: "q_hook_selection",
      question: "Select hook clip for Social Reel A",
      context: "Two potential viral hooks detected in intro discussion around 01:24 and 04:12.",
      options: [
        { id: "opt_hook_1", label: "01:24 - AI editing paradigm shift", is_default: true, description: "Strong punchy quote, high speaker energy" },
        { id: "opt_hook_2", label: "04:12 - Future of native codecs", is_default: false, description: "Deep technical insight" },
      ],
      default_option_id: "opt_hook_1",
      deadline_seconds: 300,
      status: "open",
    },
  ];

  return {
    session: {
      status: "idle",
      activeSessionId: "session-claude-01",
      activeBranch: "agent/rough-cut",
      currentAction: "Ready for round v2 review",
      stepCurrent: 2,
      stepTotal: 4,
      elapsedSeconds: 14,
      lastActor: { type: "agent", id: "claude-editor" },
      isStopped: false,
      activeMode: "fine-cut",
      loadedProfiles: ["core", "fine-cut", "review", "tasksheet"],
      grantedTiers: ["read", "edit"],
      ghostPlayheadTick: 2 * SEC,
      brief,
      plan,
      pendingPermissions: [],
      reviewQueue,
      verificationReport,
      taskSheet,
      inbox,
      budgetSpent: {
        wall_clock_seconds: 840,
        model_cost_usd: 1.25,
        render_seconds: 320,
      },
      lastError: null,
      lastTelemetry: {
        loudness_lufs: -14.2,
        true_peak_dbtp: -1.1,
        summary: "Demo EBU R128 (-14.2 LUFS / -1.1 dBTP)",
      },
    },
    playheadTicks: 0,
    selectedRoundNumber: 2,
    selectedNoteId: null,
  };
}
