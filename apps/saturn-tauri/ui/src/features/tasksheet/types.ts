// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Task Sheet v1 UI types and template definitions (TS0/TS1/TS2).
 */

export type SheetStatus =
  | "draft"
  | "ready"
  | "planning"
  | "awaiting_plan_approval"
  | "running"
  | "needs_input"
  | "in_review"
  | "revision_requested"
  | "approved"
  | "delivered"
  | "failed"
  | "cancelled"
  | "stopped";

export type InputRole =
  | "main_video"
  | "isolated_audio"
  | "music_track"
  | "b_roll"
  | "transcript"
  | "captions"
  | "capture_event_track"
  | "other";

export type TrustLevel = "trusted" | "untrusted";

export interface SheetInputState {
  id: string;
  media: string;
  role: InputRole;
  trust?: TrustLevel;
}

export interface DurationConstraint {
  target_s: number;
  tolerance_s?: number;
}

export interface SheetDeliverableState {
  id: string;
  name: string;
  preset: string;
  duration?: DurationConstraint;
  loudness_lufs?: number;
  true_peak_dbtp?: number;
  captions?: boolean;
  must_include?: string[];
}

export interface SheetStyleState {
  house_style?: string;
  pacing?: "fast" | "medium" | "slow" | "dynamic";
  music?: string;
  notes?: string;
}

export interface SheetMustNotState {
  words?: string[];
  people?: string[];
  content?: string[];
}

export interface SheetAcceptanceState {
  auto?: string[];
  human?: string[];
}

export interface SheetBudgetState {
  wall_clock_min?: number;
  model_cost_usd?: number;
  render_min?: number;
}

export interface SheetHandoffState {
  exports?: boolean;
  project_file?: boolean;
  interchange?: ("fcpxml" | "xmeml" | "edl" | "otio")[];
  qc_report?: boolean;
  change_log?: boolean;
}

export interface SheetStepState {
  id: number;
  title: string;
  description?: string;
  status: "pending" | "current" | "done" | "skipped";
  target_tick?: number;
}

export interface SheetPlanState {
  steps: SheetStepState[];
  submitted_by?: string;
  submitted_at_unix_ms?: number;
  approved_by?: string;
  approved_at_unix_ms?: number;
  rejection_reason?: string;
}

export interface SheetTransitionState {
  from: SheetStatus;
  to: SheetStatus;
  actor: string;
  timestamp_unix_ms: number;
  reason?: string;
}

export interface QuestionOption {
  id: string;
  label: string;
  description?: string;
  is_default?: boolean;
  thumbnail?: string;
}

export interface AgentQuestion {
  id: string;
  question: string;
  context?: string;
  options: QuestionOption[];
  default_option_id: string;
  deadline_seconds?: number;
  status: "open" | "answered" | "expired";
  selected_option_id?: string;
}

export type ReviewNoteStatus = "open" | "resolved" | "dismissed";

export interface ReviewNote {
  id: string;
  author: string;
  created_at_unix_ms: number;
  text: string;
  target_tick?: number;
  end_tick?: number;
  clip_id?: number;
  track_id?: number;
  status: ReviewNoteStatus;
  resolution?: string;
  resolved_by?: string;
  resolved_at_unix_ms?: number;
}

export interface ChangedTickRange {
  start_tick: number;
  end_tick: number;
  kind: string;
  entity_id?: number;
  track_id?: number;
  summary: string;
}

export interface ClipDiffEntry {
  kind: "added" | "removed" | "modified";
  clip_id: number;
  track_id: number;
  start_tick: number;
  end_tick: number;
  description: string;
}

export interface RoundDiffReport {
  from_round: number;
  to_round: number;
  from_revision: number;
  to_revision: number;
  changed_ranges: ChangedTickRange[];
  clip_changes: ClipDiffEntry[];
  merged_tick_ranges: [number, number][];
  summary: string;
}

export interface ReviewRound {
  round_number: number;
  label?: string;
  started_at_unix_ms: number;
  completed_at_unix_ms?: number;
  opened_by?: string;
  closed_by?: string;
  outcome?: "revision_requested" | "approved" | string;
  snapshot_at_open?: unknown;
  snapshot_at_close?: unknown;
  diff_from_prior?: RoundDiffReport | null;
  notes: ReviewNote[];
}

export interface TaskSheetState {
  schema: string;
  id: string;
  title: string;
  requester: string;
  inputs: SheetInputState[];
  deliverables: SheetDeliverableState[];
  style?: SheetStyleState;
  must_not?: SheetMustNotState;
  acceptance?: SheetAcceptanceState;
  budget?: SheetBudgetState;
  escalate_when?: string[];
  handoff?: SheetHandoffState;
  status: SheetStatus;
  plan?: SheetPlanState;
  questions?: AgentQuestion[];
  rounds?: ReviewRound[];
  transitions?: SheetTransitionState[];
}

export interface TemplateDefinition {
  id: string;
  label: string;
  description: string;
  template: TaskSheetState;
}

export const FLAGSHIP_TEMPLATES: TemplateDefinition[] = [
  {
    id: "podcast_social",
    label: "Podcast to Social Clips",
    description: "Vertical 9:16 cuts, -14 LUFS, safe-area captions, dead-air limits",
    template: {
      schema: "tasksheet/1",
      id: "ts_podcast_social",
      title: "Podcast Episode Highlights & Vertical Clips",
      requester: "human:producer",
      inputs: [
        { id: "in_camera", media: "media/podcast_cam_a.mp4", role: "main_video", trust: "trusted" },
        { id: "in_mic", media: "media/podcast_mic_master.wav", role: "isolated_audio", trust: "trusted" },
        { id: "in_transcript", media: "media/podcast_whisper.json", role: "transcript", trust: "untrusted" },
      ],
      deliverables: [
        {
          id: "deliv_vertical_01",
          name: "Social Hook Clip A (Vertical)",
          preset: "vertical-9x16",
          duration: { target_s: 45, tolerance_s: 10 },
          loudness_lufs: -14,
          true_peak_dbtp: -1,
          captions: true,
          must_include: ["topic:intro_hook"],
        },
        {
          id: "deliv_vertical_02",
          name: "Deep Dive Clip B (Vertical)",
          preset: "vertical-9x16",
          duration: { target_s: 60, tolerance_s: 15 },
          loudness_lufs: -14,
          true_peak_dbtp: -1,
          captions: true,
        },
      ],
      style: {
        house_style: "default_social",
        pacing: "fast",
        music: "ambient_low",
        notes: "Snappy pacing, remove dead air > 1.0s, kinetic captions centered in safe area",
      },
      must_not: {
        words: ["filler_um", "stumble"],
        people: [],
        content: ["silent_gap_over_1.5s"],
      },
      acceptance: {
        auto: [
          "verify.preset:vertical-9x16",
          "audio.integrated_loudness",
          "audio.true_peak",
          "captions.safe_area",
          "audio.silence_dead_air",
        ],
        human: ["hook_in_first_3s", "cut_pacing_natural"],
      },
      budget: { wall_clock_min: 30, model_cost_usd: 2.5, render_min: 15 },
      escalate_when: ["ambiguous_hook_selection", "unclear_word_transcription", "destructive_action"],
      handoff: {
        exports: true,
        project_file: true,
        interchange: ["fcpxml"],
        qc_report: true,
        change_log: true,
      },
      status: "ready",
    },
  },
  {
    id: "tutorial_screen",
    label: "Tutorial & Screen Capture",
    description: "Screen Studio Pro CET cursor zooms, filler cleanup, chaptering",
    template: {
      schema: "tasksheet/1",
      id: "ts_tutorial_screen",
      title: "Software Tutorial with Screen Studio Capture",
      requester: "human:instructor",
      inputs: [
        { id: "in_screen_recording", media: "media/desktop_capture.mkv", role: "main_video", trust: "trusted" },
        { id: "in_cet_events", media: "media/desktop_capture.cet.jsonl", role: "capture_event_track", trust: "trusted" },
        { id: "in_voiceover", media: "media/voiceover.wav", role: "isolated_audio", trust: "trusted" },
      ],
      deliverables: [
        {
          id: "deliv_tutorial_master",
          name: "Complete Master Tutorial (1080p)",
          preset: "web-1080p",
          duration: { target_s: 300, tolerance_s: 30 },
          loudness_lufs: -14,
          true_peak_dbtp: -1,
          captions: true,
        },
      ],
      style: {
        house_style: "tutorial_clean",
        pacing: "medium",
        music: "none",
        notes: "Zoom in 1.4x on click events with CET button down, smooth pan on cursor focus",
      },
      must_not: {
        words: [],
        people: [],
        content: ["redacted_passwords", "mouse_idle_gaps"],
      },
      acceptance: {
        auto: [
          "verify.preset:web-1080p",
          "audio.integrated_loudness",
          "audio.silence_dead_air",
          "captions.safe_area",
        ],
        human: ["clear_cursor_highlights", "zoom_crop_framing_legible"],
      },
      budget: { wall_clock_min: 45, model_cost_usd: 3.0, render_min: 20 },
      escalate_when: ["unidentified_shortcut_key", "occluded_menu_action"],
      handoff: {
        exports: true,
        project_file: true,
        interchange: ["otio", "fcpxml"],
        qc_report: true,
        change_log: true,
      },
      status: "ready",
    },
  },
  {
    id: "webinar_recap",
    label: "Webinar & Training Recap",
    description: "Executive highlight reel + full broadcast R128 archive, slide chapters",
    template: {
      schema: "tasksheet/1",
      id: "ts_webinar_recap",
      title: "Enterprise Webinar Recap & Training Summary",
      requester: "human:compliance_lead",
      inputs: [
        { id: "in_webinar_stream", media: "media/webinar_full_session.mp4", role: "main_video", trust: "trusted" },
        { id: "in_whisper_transcript", media: "media/webinar_transcript.json", role: "transcript", trust: "untrusted" },
      ],
      deliverables: [
        {
          id: "deliv_executive_summary",
          name: "Executive Summary Reel (1080p)",
          preset: "web-1080p",
          duration: { target_s: 180, tolerance_s: 20 },
          loudness_lufs: -14,
          true_peak_dbtp: -1,
          captions: true,
          must_include: ["topic:q3_financial_results"],
        },
        {
          id: "deliv_cleaned_full_archive",
          name: "Cleaned Full Archive Recording",
          preset: "broadcast-r128",
          duration: { target_s: 3600, tolerance_s: 300 },
          loudness_lufs: -23,
          true_peak_dbtp: -1,
          captions: true,
        },
      ],
      style: {
        house_style: "corporate_formal",
        pacing: "medium",
        music: "none",
        notes: "Clean speaker introductions, chapter markers on slide deck transitions",
      },
      must_not: {
        words: ["confidential_internal_previews"],
        people: [],
        content: ["unmute_chatter", "waiting_room_slate"],
      },
      acceptance: {
        auto: [
          "verify.preset:web-1080p",
          "verify.preset:broadcast-r128",
          "audio.integrated_loudness",
          "audio.silence_dead_air",
        ],
        human: ["topic_coverage_complete", "slide_transitions_aligned"],
      },
      budget: { wall_clock_min: 90, model_cost_usd: 6.0, render_min: 60 },
      escalate_when: ["unauthorized_sensitive_statement", "audio_dropouts_over_3s"],
      handoff: {
        exports: true,
        project_file: true,
        interchange: ["fcpxml"],
        qc_report: true,
        change_log: true,
      },
      status: "ready",
    },
  },
];
