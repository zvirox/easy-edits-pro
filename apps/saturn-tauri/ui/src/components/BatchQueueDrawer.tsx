import { InteriorButton } from "./interior/interior-button";
import React, { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { backdropVariants, drawerVariants, useMotionPreset } from "../motion/variants";
import { invoke } from "@tauri-apps/api/core";
import { runCommand } from "../app/ProjectProvider";
import { Icon } from "./Icon";

interface JobState {
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
  progress?: number;
  phase?: string;
  output?: string | null;
  error?: string;
}

interface LiveJob {
  id: number;
  label: string;
  state: JobState;
  created_at: number;
}

interface BatchQueueDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  onEnqueueCurrentTimeline: (preset: string) => unknown;
}

const statusColor = (status: JobState["status"]): string =>
  status === "running"
    ? "var(--text-on-accent)"
    : status === "completed"
      ? "var(--status-success)"
      : status === "failed"
        ? "var(--status-danger)"
        : "var(--text-secondary)";

/**
 * Live view over the shell's export queue (JobManager). Every row comes from
 * `get_all_jobs`; the toolbar drives pause/resume/clear, and queued jobs can
 * be reordered, cancelled, or removed. Finished and failed jobs carry their
 * parameters in the retry registry, so Retry re-submits them as-is.
 */
export const BatchQueueDrawer: React.FC<BatchQueueDrawerProps> = ({
  isOpen,
  onClose,
  onEnqueueCurrentTimeline,
}) => {
  const motionPreset = useMotionPreset();
  const [jobs, setJobs] = useState<LiveJob[]>([]);
  const [paused, setPaused] = useState(false);
  const [selectedPreset, setSelectedPreset] = useState("source");
  const [presets, setPresets] = useState<string[]>(["source"]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const live = await invoke<LiveJob[]>("get_all_jobs");
      setJobs(live);
    } catch (refreshError) {
      setError(String(refreshError));
    }
  }, []);

  // Poll while the drawer is open; export jobs progress on the worker
  // threads and the queue state changes from every control here.
  useEffect(() => {
    if (!isOpen) return;
    void refresh();
    void invoke<boolean>("is_job_queue_paused").then(setPaused).catch(() => {});
    void runCommand<{ presets: Record<string, unknown> }>("export.presets", {})
      .then((result) => {
        const names = Object.keys(result.presets ?? {});
        if (names.length) setPresets(names.sort((a, b) => a.localeCompare(b)));
      })
      .catch(() => {});
    const timer = window.setInterval(() => void refresh(), 1000);
    return () => window.clearInterval(timer);
  }, [isOpen, refresh]);

  const guard = async (action: () => Promise<unknown>) => {
    setError(null);
    try {
      await action();
    } catch (actionError) {
      setError(String(actionError));
    }
    void refresh();
  };

  const pendingCount = jobs.filter((job) => job.state.status === "queued").length;

  const queuedIds = jobs.filter((job) => job.state.status === "queued").map((job) => job.id);

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          className="modal-backdrop"
          onClick={onClose}
          style={{ zIndex: 9999 }}
          initial={motionPreset.initial}
          animate="animate"
          exit="exit"
          variants={backdropVariants}
          transition={motionPreset.transition}
        >
          <motion.div
            className="batch-queue-drawer glass"
            data-testid="batch-queue-drawer"
            data-elev="3"
            onClick={(e) => e.stopPropagation()}
            style={{
              position: "fixed",
              right: 0,
              top: 0,
              bottom: 0,
              width: 480,
              display: "flex",
              flexDirection: "column",
              zIndex: 10000,
            }}
            initial={motionPreset.initial}
            animate="animate"
            exit="exit"
            variants={drawerVariants}
            transition={motionPreset.transition}
          >
            <header
              className="batch-queue-drawer-header"
              style={{
                padding: "16px 20px",
                borderBottom: "1px solid var(--border-default)",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                background: "transparent",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <div>
                  <h3 style={{ margin: 0, fontSize: 15, fontWeight: 600 }}>Export Queue</h3>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                    Background render workers {paused ? "paused" : "running"}
                  </span>
                </div>
              </div>
              <button
                type="button"
                onClick={onClose}
                aria-label="Close export queue"
                style={{
                  background: "transparent",
                  border: "none",
                  color: "var(--text-secondary)",
                  cursor: "pointer",
                  padding: 2,
                }}
              >
                <Icon name="close" />
              </button>
            </header>

            <div
              style={{
                padding: "10px 16px",
                background: "var(--bg-surface)",
                borderBottom: "1px solid var(--border-default)",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: 8,
              }}
            >
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <InteriorButton
                  type="button"
                  data-testid="queue-pause-resume"
                  onClick={() =>
                    guard(async () => {
                      setPaused(await invoke<boolean>(paused ? "resume_job_queue" : "pause_job_queue"));
                    })
                  }
                >
                  {paused ? "Start Queue" : "Stop Queue"}
                </InteriorButton>
                <InteriorButton
                  type="button"
                  data-testid="queue-clear-finished"
                  onClick={() => guard(() => invoke("clear_job_history"))}
                >
                  Clear Finished
                </InteriorButton>
              </div>
              <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
                {jobs.length} job{jobs.length === 1 ? "" : "s"} · {pendingCount} queued
              </span>
            </div>

            <div
              style={{
                padding: "10px 16px",
                borderBottom: "1px solid var(--border-default)",
                display: "flex",
                gap: 8,
                alignItems: "center",
              }}
            >
              <select
                value={selectedPreset}
                onChange={(event) => setSelectedPreset(event.target.value)}
                aria-label="Queue preset"
                style={{
                  flex: 1,
                  padding: "5px 8px",
                  background: "var(--bg-input)",
                  color: "var(--text-primary)",
                  border: "1px solid var(--border-default)",
                  borderRadius: 4,
                  fontSize: 12,
                }}
              >
                {presets.map((name) => (
                  <option key={name} value={name}>
                    {name}
                  </option>
                ))}
              </select>
              <InteriorButton
                type="button"
                data-testid="queue-add-job"
                onClick={() =>
                  guard(async () => {
                    await onEnqueueCurrentTimeline(selectedPreset);
                  })
                }
              >
                Add Current Timeline
              </InteriorButton>
            </div>

            {error ? (
              <p role="alert" style={{ margin: 0, padding: "6px 16px", fontSize: 11, color: "var(--status-danger)" }}>
                {error}
              </p>
            ) : null}

            <div style={{ flex: 1, overflowY: "auto", padding: 16 }}>
              {jobs.length === 0 ? (
                <div style={{ textAlign: "center", padding: "40px 20px", color: "var(--text-muted)" }}>
                  <strong>Export Queue is Empty</strong>
                  <p style={{ fontSize: 12, margin: "4px 0 0" }}>
                    Add the current timeline with a preset to render in the background.
                  </p>
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                  {jobs.map((job) => {
                    const status = job.state.status;
                    const isQueued = status === "queued";
                    const isTerminal = status === "completed" || status === "failed" || status === "cancelled";
                    return (
                      <div
                        key={job.id}
                        data-testid={`queue-job-${job.id}`}
                        style={{
                          background: "var(--bg-elevated)",
                          border: "1px solid var(--border-default)",
                          borderRadius: 6,
                          padding: 12,
                        }}
                      >
                        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 6 }}>
                          <div style={{ minWidth: 0 }}>
                            <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>
                              #{job.id} {job.label}
                            </div>
                            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>
                              {status === "running"
                                ? `${job.state.phase ?? "Rendering"} · ${Math.round((job.state.progress ?? 0) * 100)}%`
                                : status === "failed"
                                  ? `Failed: ${job.state.error ?? "unknown error"}`
                                  : status === "completed"
                                    ? `Completed: ${job.state.output ?? "done"}`
                                    : status === "cancelled"
                                      ? "Cancelled"
                                      : "Waiting for a worker"}
                            </div>
                          </div>
                          <span
                            style={{
                              fontSize: 10,
                              fontWeight: 700,
                              padding: "2px 6px",
                              borderRadius: 3,
                              textTransform: "uppercase",
                              color: statusColor(status),
                              border: "1px solid var(--border-default)",
                            }}
                          >
                            {status}
                          </span>
                        </div>
                        {status === "running" ? (
                          <div
                            style={{
                              height: 4,
                              borderRadius: 2,
                              background: "var(--border-default)",
                              overflow: "hidden",
                              marginBottom: 8,
                            }}
                          >
                            <div
                              style={{
                                width: "100%",
                                height: "100%",
                                background: "var(--accent-primary)",
                                transform: `scaleX(${Math.max(0, Math.min(1, job.state.progress ?? 0))})`,
                                transformOrigin: "left center",
                                transition: "transform 240ms linear",
                              }}
                            />
                          </div>
                        ) : null}
                        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                          {!isTerminal ? (
                            <InteriorButton
                              type="button"
                              data-testid={`queue-cancel-${job.id}`}
                              onClick={() => guard(() => invoke("cancel_job", { id: job.id }))}
                            >
                              Cancel
                            </InteriorButton>
                          ) : null}
                          {isQueued ? (
                            <>
                              <InteriorButton
                                type="button"
                                onClick={() =>
                                  guard(() =>
                                    invoke("move_queued_export", {
                                      jobId: job.id,
                                      targetIndex: Math.max(0, queuedIds.indexOf(job.id) - 1),
                                    }),
                                  )
                                }
                                disabled={queuedIds.indexOf(job.id) === 0}
                              >
                                Move Up
                              </InteriorButton>
                              <InteriorButton
                                type="button"
                                onClick={() =>
                                  guard(() =>
                                    invoke("move_queued_export", {
                                      jobId: job.id,
                                      targetIndex: queuedIds.indexOf(job.id) + 1,
                                    }),
                                  )
                                }
                                disabled={queuedIds.indexOf(job.id) === queuedIds.length - 1}
                              >
                                Move Down
                              </InteriorButton>
                              <InteriorButton
                                type="button"
                                onClick={() => guard(() => invoke("remove_queued_export", { jobId: job.id }))}
                              >
                                Remove
                              </InteriorButton>
                            </>
                          ) : null}
                          {isTerminal ? (
                            <InteriorButton
                              type="button"
                              data-testid={`queue-retry-${job.id}`}
                              onClick={() => guard(() => invoke("retry_export", { jobId: job.id }))}
                            >
                              Retry
                            </InteriorButton>
                          ) : null}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};
