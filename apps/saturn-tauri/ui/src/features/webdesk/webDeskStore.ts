// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Web Desk State Store (TS8).
 * Manages jobs, plan approvals, agent questions, and review flows for the Web Desk UI.
 */

import { useState, useEffect, useCallback, useMemo } from "react";
import { agentBridge } from "../agent/agentBridge";
import type {
  AgentQuestion,
  AgentRunLogEntry,
  ReviewNote,
  SheetDeliverableState,
  SheetInputState,
  SheetStatus,
  TaskSheetPlanState,
  WebDeskBoardMode,
  WebDeskFilterState,
  WebDeskJob,
  WebDeskMetrics,
  WebDeskView,
} from "./types";

function isDemoMode(): boolean {
  if (typeof window === "undefined") return false;
  try {
    const params = new URLSearchParams(window.location.search);
    return params.get("demo") === "1";
  } catch {
    return false;
  }
}

function createInitialJobs(): WebDeskJob[] {
  if (!isDemoMode()) {
    return [];
  }

  const now = Date.now();
  return [
    {
      id: "job_seed_podcast",
      title: "Episode 104 — Tech Founders Roundtable Clips",
      templateId: "podcast_social_clips",
      requester: "human:alex_prod",
      status: "in_review",
      inputs: [
        { id: "in_vid", media: "media/raw_episode_104.mp4", role: "main_video", trust: "trusted" },
        { id: "in_aud", media: "media/host_lav.wav", role: "isolated_audio", trust: "trusted" },
        { id: "in_sub", media: "media/whisper_transcription.json", role: "transcript", trust: "untrusted" },
      ],
      deliverables: [
        {
          id: "deliv_vertical_hook",
          name: "Viral Hook 9:16 Short",
          preset: "social-9x16",
          duration: { target_s: 45, tolerance_s: 5 },
          loudness_lufs: -14,
          true_peak_dbtp: -1,
          captions: true,
        },
      ],
      budget: { wall_clock_min: 60, model_cost_usd: 3.5, render_min: 20 },
      plan: {
        steps: [
          { id: 1, title: "Silence & filler detection", status: "completed" },
          { id: 2, title: "Candidate highlight selection", status: "completed" },
          { id: 3, title: "Reframe & safe area caption alignment", status: "completed" },
          { id: 4, title: "Loudness normalisation (-14 LUFS)", status: "completed" },
        ],
        submitted_by: "agent:planner",
        submitted_at_unix_ms: now - 3600000,
        approved_by: "human:alex_prod",
        approved_at_unix_ms: now - 3500000,
      },
      questions: [],
      rounds: [
        {
          round_number: 1,
          label: "v1",
          started_at_unix_ms: now - 3000000,
          opened_by: "system",
          diff_from_prior: {
            from_round: 0,
            to_round: 1,
            from_revision: 0,
            to_revision: 1,
            changed_ranges: [],
            clip_changes: [],
            merged_tick_ranges: [],
            summary: "Initial automated rough cut with captions",
          },
          notes: [
            {
              id: "note_101",
              target_tick: 4800,
              text: "Trim host laugh slightly before guest responds",
              author: "human:alex_prod",
              created_at_unix_ms: now - 1800000,
              status: "open",
            },
          ],
        },
      ],
      runLog: [
        {
          id: "log_1",
          timestamp_unix_ms: now - 3400000,
          tool: "audio.silence_detect",
          tier: "read",
          actor: "agent:rough-cut",
          rationale: "Scanning speech pauses > 750ms",
          status: "success",
        },
        {
          id: "log_2",
          timestamp_unix_ms: now - 3200000,
          tool: "edit.ripple_delete",
          tier: "edit",
          actor: "agent:rough-cut",
          rationale: "Removed 14 dead-air intervals",
          status: "success",
        },
      ],
      created_at_unix_ms: now - 4000000,
      updated_at_unix_ms: now - 1800000,
      media_proxy_url: "editor-smoke.webm",
    },
  ];
}

// Global in-memory singleton for jobs across components during a session
let globalJobs: WebDeskJob[] = createInitialJobs();
let listeners: Array<() => void> = [];

function notifyListeners() {
  for (const listener of listeners) {
    listener();
  }
}

export function useWebDeskStore() {
  const [jobs, setJobsState] = useState<WebDeskJob[]>(globalJobs);
  const [activeJobId, setActiveJobId] = useState<string | null>(
    () => (globalJobs.length > 0 ? globalJobs[0].id : null)
  );
  const [activeView, setActiveView] = useState<WebDeskView>("jobs");
  const [boardMode, setBoardMode] = useState<WebDeskBoardMode>("kanban");
  const [isAgplNoticeOpen, setIsAgplNoticeOpen] = useState(false);
  const [filter, setFilter] = useState<WebDeskFilterState>({
    status: "all",
    search: "",
    requester: "all",
  });

  useEffect(() => {
    const handler = () => {
      setJobsState([...globalJobs]);
    };
    listeners.push(handler);
    return () => {
      listeners = listeners.filter((l) => l !== handler);
    };
  }, []);

  const updateJobs = useCallback((updater: (prev: WebDeskJob[]) => WebDeskJob[]) => {
    globalJobs = updater(globalJobs);
    setJobsState([...globalJobs]);
    notifyListeners();
  }, []);

  // Sync engine / mock backend sheet changes (e.g. agent tool calls, questions)
  useEffect(() => {
    const syncFromEngine = async () => {
      try {
        const res = await agentBridge.runCommand<{ sheet?: any; status?: string }>("tasksheet.get");
        if (res?.sheet && activeJobId) {
          const sheet = res.sheet;
          updateJobs((prev) =>
            prev.map((j) => {
              if (j.id === activeJobId) {
                return {
                  ...j,
                  status: (sheet.status as SheetStatus) || j.status,
                  questions: sheet.questions?.length ? sheet.questions : j.questions,
                  rounds: sheet.rounds?.length ? sheet.rounds : j.rounds,
                  plan: sheet.plan ?? j.plan,
                  updated_at_unix_ms: Date.now(),
                };
              }
              return j;
            })
          );
        }
      } catch (err) {
        // bridge not available or failed
      }
    };

    let unlisten: (() => void) | null = null;
    try {
      void import("@tauri-apps/api/event").then(({ listen }) => {
        void listen("mcp_project_changed", () => {
          void syncFromEngine();
        }).then((u) => {
          unlisten = u;
        });
      });
    } catch {}

    const handleCustomEvent = () => {
      void syncFromEngine();
    };
    window.addEventListener("mcp_project_changed", handleCustomEvent);

    return () => {
      if (unlisten) unlisten();
      window.removeEventListener("mcp_project_changed", handleCustomEvent);
    };
  }, [activeJobId, updateJobs]);

  const activeJob = useMemo(() => {
    return jobs.find((j) => j.id === activeJobId) ?? null;
  }, [jobs, activeJobId]);

  const metrics = useMemo<WebDeskMetrics>(() => {
    let active = 0;
    let inReview = 0;
    let needsInput = 0;
    let completed = 0;
    let totalCost = 0;
    let totalTime = 0;

    for (const job of jobs) {
      if (job.status === "approved" || job.status === "delivered") {
        completed += 1;
      } else if (job.status !== "failed" && job.status !== "cancelled") {
        active += 1;
      }
      if (job.status === "in_review" || job.status === "revision_requested") {
        inReview += 1;
      }
      if (job.status === "needs_input") {
        needsInput += 1;
      }
      totalCost += job.budget?.model_cost_usd ?? 0;
      totalTime += job.budget?.wall_clock_min ?? 0;
    }

    return {
      totalJobs: jobs.length,
      activeJobs: active,
      inReviewCount: inReview,
      needsInputCount: needsInput,
      completedJobs: completed,
      totalModelCostUsd: Number(totalCost.toFixed(2)),
      totalWallClockMin: totalTime,
    };
  }, [jobs]);

  const selectJob = useCallback((id: string) => {
    setActiveJobId(id);
  }, []);

  const createJob = useCallback(
    async (params: {
      title: string;
      templateId?: string;
      requester: string;
      inputs: SheetInputState[];
      deliverables: SheetDeliverableState[];
      budget: { wall_clock_min: number; model_cost_usd: number; render_min: number };
    }): Promise<WebDeskJob> => {
      const newId = `job_${Date.now()}_${Math.random().toString(36).substring(2, 6)}`;
      const now = Date.now();

      // Create sheet in engine if possible
      try {
        await agentBridge.runCommand("tasksheet.create", {
          id: newId,
          title: params.title,
          requester: params.requester,
          inputs: params.inputs,
          deliverables: params.deliverables,
          budget: params.budget,
        });
        await agentBridge.runCommand("tasksheet.submit", { reason: "Web Desk user submission" });
      } catch (err) {
        // Fall back gracefully if engine bridge isn't active
        console.warn("[WebDeskStore] backend sheet command:", err);
      }

      const newJob: WebDeskJob = {
        id: newId,
        title: params.title,
        templateId: params.templateId,
        requester: params.requester,
        status: "ready",
        inputs: params.inputs,
        deliverables: params.deliverables,
        budget: params.budget,
        plan: {
          steps: [
            { id: 1, title: "Detect silence and conversational pauses", status: "pending" },
            { id: 2, title: "Generate candidate cuts based on chapter boundaries", status: "pending" },
            { id: 3, title: "Apply title graphics and safe-area captions", status: "pending" },
            { id: 4, title: "Export verification proxy and QC summary", status: "pending" },
          ],
          submitted_by: "agent:planner",
          submitted_at_unix_ms: now,
        },
        questions: [],
        rounds: [],
        runLog: [
          {
            id: `log_init_${now}`,
            timestamp_unix_ms: now,
            tool: "tasksheet.submit",
            tier: "edit",
            actor: params.requester,
            rationale: "Initial sheet creation and submission",
            status: "success",
          },
        ],
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
        media_proxy_url: "editor-smoke.webm",
      };

      updateJobs((prev) => [newJob, ...prev]);
      setActiveJobId(newId);
      return newJob;
    },
    [updateJobs]
  );

  const approvePlan = useCallback(
    async (jobId: string) => {
      try {
        await agentBridge.runCommand("tasksheet.plan_approve", {
          reason: "Web Desk plan approval",
        });
      } catch (err) {
        console.warn("[WebDeskStore] plan_approve command:", err);
      }

      updateJobs((prev) =>
        prev.map((j) => {
          if (j.id !== jobId) return j;
          const updatedPlan: TaskSheetPlanState = {
            ...(j.plan ?? { steps: [] }),
            approved_by: "human:supervisor",
            approved_at_unix_ms: Date.now(),
          };
          const runLogEntry: AgentRunLogEntry = {
            id: `log_plan_${Date.now()}`,
            timestamp_unix_ms: Date.now(),
            tool: "tasksheet.plan_approve",
            tier: "edit",
            actor: "human:supervisor",
            rationale: "Plan approved by supervisor",
            status: "success",
          };
          return {
            ...j,
            status: "running",
            plan: updatedPlan,
            runLog: [runLogEntry, ...j.runLog],
            updated_at_unix_ms: Date.now(),
          };
        })
      );
    },
    [updateJobs]
  );

  const rejectPlan = useCallback(
    async (jobId: string, reason: string) => {
      try {
        await agentBridge.runCommand("tasksheet.plan_reject", { reason });
      } catch (err) {
        console.warn("[WebDeskStore] plan_reject command:", err);
      }

      updateJobs((prev) =>
        prev.map((j) => {
          if (j.id !== jobId) return j;
          const updatedPlan: TaskSheetPlanState = {
            ...(j.plan ?? { steps: [] }),
            rejection_reason: reason,
          };
          return {
            ...j,
            status: "planning",
            plan: updatedPlan,
            updated_at_unix_ms: Date.now(),
          };
        })
      );
    },
    [updateJobs]
  );

  const postAgentQuestion = useCallback(
    (jobId: string, question: AgentQuestion) => {
      updateJobs((prev) =>
        prev.map((j) => {
          if (j.id !== jobId) return j;
          return {
            ...j,
            status: "needs_input",
            questions: [...j.questions, question],
            updated_at_unix_ms: Date.now(),
          };
        })
      );
    },
    [updateJobs]
  );

  const answerQuestion = useCallback(
    async (jobId: string, questionId: string, optionId: string) => {
      try {
        await agentBridge.runCommand("tasksheet.question_answer", {
          question_id: questionId,
          option_id: optionId,
        });
      } catch (err) {
        console.warn("[WebDeskStore] question_answer command:", err);
      }

      updateJobs((prev) =>
        prev.map((j) => {
          if (j.id !== jobId) return j;
          const updatedQuestions = j.questions.map((q) =>
            q.id === questionId ? { ...q, status: "answered" as const, selected_option_id: optionId } : q
          );
          const hasRemainingOpen = updatedQuestions.some((q) => q.status === "open");
          return {
            ...j,
            status: hasRemainingOpen ? "needs_input" : "in_review",
            questions: updatedQuestions,
            updated_at_unix_ms: Date.now(),
          };
        })
      );
    },
    [updateJobs]
  );

  const addReviewNote = useCallback(
    async (jobId: string, text: string, targetTick: number = 0) => {
      try {
        await agentBridge.runCommand("review.note_add", {
          text,
          target_tick: targetTick,
        });
      } catch (err) {
        console.warn("[WebDeskStore] review.note_add:", err);
      }

      const noteId = `note_${Date.now()}`;
      const now = Date.now();
      const newNote: ReviewNote = {
        id: noteId,
        text,
        target_tick: targetTick,
        author: "human:reviewer",
        created_at_unix_ms: now,
        status: "open",
      };

      updateJobs((prev) =>
        prev.map((j) => {
          if (j.id !== jobId) return j;
          const rounds = [...j.rounds];
          if (rounds.length === 0) {
            rounds.push({
              round_number: 1,
              label: "v1",
              started_at_unix_ms: now,
              opened_by: "human:reviewer",
              notes: [newNote],
              diff_from_prior: {
                from_round: 0,
                to_round: 1,
                from_revision: 0,
                to_revision: 1,
                changed_ranges: [],
                clip_changes: [],
                merged_tick_ranges: [],
                summary: "Initial review candidate",
              },
            });
          } else {
            const latestRound = { ...rounds[rounds.length - 1] };
            latestRound.notes = [...(latestRound.notes ?? []), newNote];
            rounds[rounds.length - 1] = latestRound;
          }
          return {
            ...j,
            rounds,
            updated_at_unix_ms: now,
          };
        })
      );
    },
    [updateJobs]
  );

  const resolveReviewNote = useCallback(
    async (jobId: string, noteId: string, resolution: string) => {
      try {
        await agentBridge.runCommand("review.note_resolve", {
          note_id: noteId,
          resolution,
          status: "resolved",
        });
      } catch (err) {
        console.warn("[WebDeskStore] review.note_resolve:", err);
      }

      updateJobs((prev) =>
        prev.map((j) => {
          if (j.id !== jobId) return j;
          const rounds = j.rounds.map((r) => ({
            ...r,
            notes: (r.notes ?? []).map((n: ReviewNote) =>
              n.id === noteId
                ? {
                    ...n,
                    status: "resolved" as const,
                    resolution,
                    resolved_at_unix_ms: Date.now(),
                  }
                : n
            ),
          }));
          return {
            ...j,
            rounds,
            updated_at_unix_ms: Date.now(),
          };
        })
      );
    },
    [updateJobs]
  );

  const requestRevision = useCallback(
    async (jobId: string, reason?: string) => {
      try {
        await agentBridge.runCommand("review.round_close", {
          outcome: "revision_requested",
          reason,
        });
      } catch (err) {
        console.warn("[WebDeskStore] review.round_close (revision):", err);
      }

      updateJobs((prev) =>
        prev.map((j) => {
          if (j.id !== jobId) return j;
          return {
            ...j,
            status: "revision_requested",
            updated_at_unix_ms: Date.now(),
          };
        })
      );
    },
    [updateJobs]
  );

  const approveDelivery = useCallback(
    async (jobId: string) => {
      try {
        await agentBridge.runCommand("review.round_close", {
          outcome: "approved",
        });
      } catch (err) {
        console.warn("[WebDeskStore] review.round_close (approved):", err);
      }

      updateJobs((prev) =>
        prev.map((j) => {
          if (j.id !== jobId) return j;
          return {
            ...j,
            status: "delivered",
            updated_at_unix_ms: Date.now(),
          };
        })
      );
    },
    [updateJobs]
  );

  useEffect(() => {
    if (typeof window !== "undefined") {
      (window as any).__webDeskStore = {
        jobs,
        activeJob,
        activeJobId,
        selectJob,
        setActiveView,
        postAgentQuestion,
        answerQuestion,
        approvePlan,
        rejectPlan,
        addReviewNote,
        resolveReviewNote,
        approveDelivery,
      };
    }
  }, [
    jobs,
    activeJob,
    activeJobId,
    selectJob,
    setActiveView,
    postAgentQuestion,
    answerQuestion,
    approvePlan,
    rejectPlan,
    addReviewNote,
    resolveReviewNote,
    approveDelivery,
  ]);

  return {
    jobs,
    activeJob,
    activeJobId,
    activeView,
    boardMode,
    filter,
    metrics,
    isAgplNoticeOpen,
    selectJob,
    setActiveView,
    setBoardMode,
    setFilter,
    setIsAgplNoticeOpen,
    createJob,
    approvePlan,
    rejectPlan,
    postAgentQuestion,
    answerQuestion,
    addReviewNote,
    resolveReviewNote,
    requestRevision,
    approveDelivery,
  };
}
