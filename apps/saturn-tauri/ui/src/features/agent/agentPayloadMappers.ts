// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Pure mappers converting Rust engine / MCP bridge JSON responses into
 * typed agent store structures.
 */

import type { TaskSheetState } from "../tasksheet/types.ts";
import type {
  AgentTelemetrySummary,
  ConfidenceTier,
  PlanStep,
  ProjectBriefState,
  ReviewChange,
  VerificationEvidenceItem,
  VerificationReport,
} from "./types.ts";

export function parseActor(raw: unknown): { type: "human" | "agent"; id: string } | null {
  if (!raw) return null;
  if (typeof raw === "string") {
    if (raw === "human") return { type: "human", id: "human" };
    if (raw.startsWith("agent:")) return { type: "agent", id: raw.slice("agent:".length) };
    return { type: "agent", id: raw };
  }
  if (typeof raw === "object") {
    const obj = raw as { kind?: unknown; id?: unknown };
    if (obj.kind === "human") return { type: "human", id: "human" };
    if (obj.kind === "agent" && typeof obj.id === "string") {
      return { type: "agent", id: obj.id };
    }
  }
  return null;
}

function parseConfidence(raw: unknown): ConfidenceTier {
  if (typeof raw === "string") {
    const norm = raw.toLowerCase();
    if (norm === "low" || norm === "high" || norm === "medium") return norm;
  }
  return "medium";
}

function confidenceScore(tier: ConfidenceTier): number {
  if (tier === "low") return 0.5;
  if (tier === "high") return 0.95;
  return 0.75;
}

export function mapRawChange(raw: Record<string, unknown>): ReviewChange {
  const tierConf = parseConfidence(raw.confidence);
  const ticks = Array.isArray(raw.tick_range)
    ? ([Number(raw.tick_range[0] ?? 0), Number(raw.tick_range[1] ?? 0)] as [number, number])
    : ([0, 0] as [number, number]);
  const commandId = typeof raw.command_id === "string" ? raw.command_id : undefined;
  const actor = typeof raw.actor === "string" ? raw.actor : undefined;
  const undone = Boolean(raw.undone);
  return {
    change_id: Number(raw.change_id ?? 0),
    command_id: commandId,
    actor,
    branch: typeof raw.branch === "string" ? raw.branch : undefined,
    step_title: commandId,
    tick_range: ticks,
    tier: typeof raw.tier === "string" ? raw.tier : "edit",
    rationale:
      typeof raw.rationale === "string" && raw.rationale.trim()
        ? raw.rationale
        : `${commandId ?? "edit"}${actor ? ` (${actor})` : ""}`,
    confidence: tierConf,
    confidence_score: confidenceScore(tierConf),
    rejected: undone,
    undone,
    conflict: Boolean(raw.conflict),
    conflict_reason: typeof raw.conflict_reason === "string" ? raw.conflict_reason : null,
  };
}

export function mapBrief(raw: unknown): ProjectBriefState | null {
  if (!raw || typeof raw !== "object") return null;
  const b = raw as Record<string, unknown>;
  const goal = typeof b.goal === "string" ? b.goal : "";
  const platform = typeof b.platform === "string" ? b.platform : "";
  const styleNotes = typeof b.style_notes === "string" ? b.style_notes : "";
  const banned = Array.isArray(b.banned_content)
    ? b.banned_content.filter((x): x is string => typeof x === "string")
    : [];
  const mustKeep = Array.isArray(b.must_keep_clip_ids)
    ? b.must_keep_clip_ids.filter((x): x is number => typeof x === "number")
    : [];
  const hasContent =
    goal.trim().length > 0 ||
    platform.trim().length > 0 ||
    styleNotes.trim().length > 0 ||
    banned.length > 0 ||
    mustKeep.length > 0 ||
    b.loudness_target_lufs != null ||
    b.target_duration_ticks != null;
  if (!hasContent) return null;
  return {
    goal,
    platform: platform || "web-1080p",
    target_duration_ticks: typeof b.target_duration_ticks === "number" ? b.target_duration_ticks : 0,
    duration_tolerance_ticks:
      typeof b.duration_tolerance_ticks === "number" ? b.duration_tolerance_ticks : 0,
    must_keep_clip_ids: mustKeep,
    banned_content: banned,
    style_notes: styleNotes,
    loudness_target_lufs:
      typeof b.loudness_target_lufs === "number" ? b.loudness_target_lufs : -14.0,
    source: typeof b.source === "string" ? b.source : "user_prompt",
  };
}

function formatMeasured(value: unknown): number | string {
  if (typeof value === "number" || typeof value === "string") return value;
  if (value && typeof value === "object") return JSON.stringify(value);
  return "n/a";
}

export function parseVerifyReport(raw: unknown): {
  report: VerificationReport | null;
  telemetry: AgentTelemetrySummary | null;
} {
  if (!raw || typeof raw !== "object") return { report: null, telemetry: null };
  const obj = raw as Record<string, unknown>;
  const preset = typeof obj.preset === "string" ? obj.preset : "web-1080p";
  if (Array.isArray(obj.results)) {
    return { report: raw as VerificationReport, telemetry: null };
  }
  if (!Array.isArray(obj.checks)) return { report: null, telemetry: null };

  const results: VerificationEvidenceItem[] = [];
  const unavailable_checks: string[] = [];
  let loudness: number | null = null;
  let truePeak: number | null = null;

  for (const check of obj.checks as Array<Record<string, unknown>>) {
    const checkId = String(check.check_id ?? "");
    const statusStr = String(check.status ?? "measured");
    if (statusStr === "unavailable" || statusStr === "skipped") {
      if (checkId) unavailable_checks.push(checkId);
      continue;
    }
    const verdictRaw = String(check.verdict ?? "pass");
    const status: "pass" | "warn" | "fail" =
      verdictRaw === "fail" ? "fail" : verdictRaw === "warn" ? "warn" : "pass";
    const ranges = Array.isArray(check.offending_ranges) ? check.offending_ranges : [];
    const firstRange = ranges[0] as Record<string, unknown> | undefined;
    results.push({
      check_id: checkId,
      status,
      measured_value: formatMeasured(check.measured),
      threshold: formatMeasured(check.threshold),
      tick_ref: typeof firstRange?.start_tick === "number" ? firstRange.start_tick : undefined,
      clip_id: typeof firstRange?.clip_id === "number" ? firstRange.clip_id : undefined,
      message: typeof check.message === "string" ? check.message : undefined,
    });
    if (checkId === "audio.integrated_loudness" && typeof check.measured === "number") {
      loudness = check.measured;
    }
    if (checkId === "audio.true_peak" && typeof check.measured === "number") {
      truePeak = check.measured;
    }
  }

  const telemetry: AgentTelemetrySummary | null =
    loudness != null || truePeak != null || results.length > 0
      ? {
          loudness_lufs: loudness,
          true_peak_dbtp: truePeak,
          summary: `${results.length} measured check(s), ${unavailable_checks.length} unavailable`,
        }
      : null;

  return {
    report: {
      preset,
      results,
      unavailable_checks,
      notes: (obj.notes as Record<string, unknown>) ?? { aggregate_score: null },
    },
    telemetry,
  };
}

export function mapPlanSteps(sheet: TaskSheetState | null): PlanStep[] {
  if (!sheet?.plan?.steps) return [];
  return sheet.plan.steps
    .filter((s) => s.status !== "skipped")
    .map((s) => ({
      id: s.id,
      title: s.title,
      description: s.description,
      status: s.status === "done" ? "done" : s.status === "current" ? "current" : "pending",
      targetTick: s.target_tick,
    }));
}

export function sheetHasRealState(sheet: TaskSheetState | null, hasAttachedSheet: boolean): boolean {
  if (!sheet?.id) return false;
  if (hasAttachedSheet) return true;
  return (
    sheet.status !== "draft" ||
    (sheet.rounds?.length ?? 0) > 0 ||
    (sheet.questions?.length ?? 0) > 0 ||
    (sheet.plan?.steps?.length ?? 0) > 0 ||
    sheet.title !== "Default Project Sheet"
  );
}
