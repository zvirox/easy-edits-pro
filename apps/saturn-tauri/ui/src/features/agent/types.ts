// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Agent and Review UI types (AF6 / Packet 5.2).
 */

export type AgentStatus = "idle" | "thinking" | "editing" | "waiting" | "stopped" | "error";

export type ConfidenceTier = "high" | "medium" | "low";

export interface PlanStep {
  id: number;
  title: string;
  description?: string;
  status: "done" | "current" | "pending";
  targetTick?: number;
}

export interface PermissionRequest {
  id: string;
  tool: string;
  argument: string;
  reason: string;
  tier: "read" | "edit" | "filesystem" | "admin";
  originSource?: "transcript" | "caption" | "filename" | "metadata" | "ocr" | "imported_project_text";
  timestamp: number;
}

export interface ReviewChange {
  change_id: number;
  step_id?: number;
  step_title?: string;
  tick_range: [number, number];
  tier: string;
  rationale: string;
  confidence: ConfidenceTier;
  confidence_score: number;
  before_preview?: string;
  after_preview?: string;
  accepted?: boolean;
  rejected?: boolean;
  feedback?: string;
}

export interface VerificationEvidenceItem {
  check_id: string;
  status: "pass" | "warn" | "fail";
  measured_value: number | string;
  threshold: number | string;
  tick_ref?: number;
  clip_id?: number;
  message?: string;
}

export interface VerificationReport {
  preset: string;
  results: VerificationEvidenceItem[];
  unavailable_checks: string[];
  notes: Record<string, unknown>;
}

export interface ProjectBriefState {
  goal: string;
  platform: string;
  target_duration_ticks: number;
  duration_tolerance_ticks: number;
  must_keep_clip_ids: number[];
  banned_content: string[];
  style_notes: string;
  loudness_target_lufs: number;
  source: string;
}

export interface AgentSessionState {
  status: AgentStatus;
  activeSessionId: string | null;
  activeBranch: string;
  currentAction: string;
  stepCurrent: number;
  stepTotal: number;
  elapsedSeconds: number;
  lastActor: { type: "human" | "agent"; id: string };
  isStopped: boolean;
  activeMode: string;
  loadedProfiles: string[];
  grantedTiers: string[];
  ghostPlayheadTick: number | null;
  brief: ProjectBriefState;
  plan: PlanStep[];
  pendingPermissions: PermissionRequest[];
  reviewQueue: ReviewChange[];
  verificationReport: VerificationReport | null;
}
