// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Web Desk MVP types (TS8).
 * Defines data structures for the browser-based B2B self-hosted Web Desk.
 */

import type {
  AgentQuestion,
  DurationConstraint,
  InputRole,
  ReviewNote,
  ReviewRound,
  SheetAcceptanceState,
  SheetBudgetState,
  SheetDeliverableState,
  SheetInputState,
  SheetMustNotState,
  SheetStatus,
  SheetStyleState,
  TaskSheetState,
  TrustLevel,
} from "../tasksheet/types";

export type {
  AgentQuestion,
  DurationConstraint,
  InputRole,
  ReviewNote,
  ReviewRound,
  SheetAcceptanceState,
  SheetBudgetState,
  SheetDeliverableState,
  SheetInputState,
  SheetMustNotState,
  SheetStatus,
  SheetStyleState,
  TaskSheetState,
  TrustLevel,
};

export type ReviewRoundState = ReviewRound;

export type WebDeskView = "jobs" | "new_job" | "detail" | "review";

export type WebDeskBoardMode = "kanban" | "list";

export interface PlannedStep {
  id: number;
  title: string;
  description?: string;
  status: "pending" | "in_progress" | "completed" | "failed";
  target_tick?: number;
}

export interface TaskSheetPlanState {
  steps: PlannedStep[];
  submitted_by?: string;
  submitted_at_unix_ms?: number;
  approved_by?: string;
  approved_at_unix_ms?: number;
  rejection_reason?: string;
}

export interface AgentRunLogEntry {
  id: string;
  timestamp_unix_ms: number;
  tool: string;
  tier: "read" | "edit" | "filesystem";
  actor: string;
  rationale?: string;
  status: "success" | "pending" | "failed";
}

export interface WebDeskJob {
  id: string;
  title: string;
  templateId?: string;
  requester: string;
  status: SheetStatus;
  inputs: SheetInputState[];
  deliverables: SheetDeliverableState[];
  style?: SheetStyleState;
  must_not?: SheetMustNotState;
  acceptance?: SheetAcceptanceState;
  budget: SheetBudgetState;
  plan?: TaskSheetPlanState;
  questions: AgentQuestion[];
  rounds: ReviewRoundState[];
  runLog: AgentRunLogEntry[];
  created_at_unix_ms: number;
  updated_at_unix_ms: number;
  media_proxy_url?: string;
}

export interface WebDeskFilterState {
  status: SheetStatus | "all";
  search: string;
  requester: string | "all";
}

export interface WebDeskMetrics {
  totalJobs: number;
  activeJobs: number;
  inReviewCount: number;
  needsInputCount: number;
  completedJobs: number;
  totalModelCostUsd: number;
  totalWallClockMin: number;
}
