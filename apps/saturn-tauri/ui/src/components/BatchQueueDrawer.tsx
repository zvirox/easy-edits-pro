import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface BatchJobItem {
  id: number;
  label: string;
  preset: string;
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
  progress: number;
  phase: string;
  outputPath: string;
}

interface BatchQueueDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  onEnqueueCurrentTimeline: (preset: string) => void;
}

export const BatchQueueDrawer: React.FC<BatchQueueDrawerProps> = ({
  isOpen,
  onClose,
  onEnqueueCurrentTimeline,
}) => {
  const [isDaemonPaused, setIsDaemonPaused] = useState(false);
  const [selectedPreset, setSelectedPreset] = useState("YouTube 4K (H.264/AAC)");

  // Sample and live jobs
  const [jobs, setJobs] = useState<BatchJobItem[]>([
    {
      id: 101,
      label: "Feature Cut_Master_v1.mp4",
      preset: "ProRes 422 HQ Master",
      status: "running",
      progress: 68,
      phase: "Rendering Color & Effects",
      outputPath: "/exports/Feature_Cut_Master_v1.mp4",
    },
    {
      id: 102,
      label: "Social_Teaser_Vertical.mp4",
      preset: "TikTok / Reels 1080x1920",
      status: "queued",
      progress: 0,
      phase: "Pending",
      outputPath: "/exports/Social_Teaser_Vertical.mp4",
    },
  ]);

  if (!isOpen) return null;

  const handleTogglePause = async () => {
    try {
      if (isDaemonPaused) {
        await invoke("resume_job_queue");
        setIsDaemonPaused(false);
      } else {
        await invoke("pause_job_queue");
        setIsDaemonPaused(true);
      }
    } catch {
      setIsDaemonPaused(!isDaemonPaused);
    }
  };

  const handleClearCompleted = async () => {
    try {
      await invoke("clear_job_history");
    } catch {
      // fallback
    }
    setJobs((prev) => prev.filter((j) => j.status === "running" || j.status === "queued"));
  };

  const handleCancelJob = async (id: number) => {
    try {
      await invoke("cancel_job", { jobId: id });
    } catch {
      // fallback
    }
    setJobs((prev) =>
      prev.map((j) => (j.id === id ? { ...j, status: "cancelled", phase: "Cancelled by user" } : j))
    );
  };

  const handleAddJob = () => {
    onEnqueueCurrentTimeline(selectedPreset);
    const newJob: BatchJobItem = {
      id: Date.now(),
      label: `Export_${selectedPreset.split(" ")[0]}_${Date.now().toString().slice(-4)}.mp4`,
      preset: selectedPreset,
      status: "queued",
      progress: 0,
      phase: "Queued",
      outputPath: `/exports/render_${Date.now().toString().slice(-4)}.mp4`,
    };
    setJobs((prev) => [...prev, newJob]);
  };

  const exportPresets = [
    "YouTube 4K (H.264/AAC)",
    "ProRes 422 HQ Master",
    "Web 1080p 60fps (Fast Encode)",
    "TikTok / Reels (1080x1920 9:16)",
    "Cinema DCI 4K (DCP Master)",
  ];

  return (
    <div className="modal-backdrop" onClick={onClose} style={{ zIndex: 9999 }}>
      <div
        className="batch-queue-drawer"
        onClick={(e) => e.stopPropagation()}
        style={{
          position: "fixed",
          right: 0,
          top: 0,
          bottom: 0,
          width: 480,
          background: "#11141c",
          borderLeft: "1px solid #2d3748",
          boxShadow: "-10px 0 25px rgba(0, 0, 0, 0.6)",
          color: "#e2e8f0",
          display: "flex",
          flexDirection: "column",
          zIndex: 10000,
        }}
      >
        {/* Drawer Header */}
        <header
          style={{
            padding: "16px 20px",
            borderBottom: "1px solid #2d3748",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            background: "#181b24",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span style={{ fontSize: 20 }}>📦</span>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <h3 style={{ margin: 0, fontSize: 15, fontWeight: 600 }}>Background Batch Queue</h3>
                <span
                  style={{
                    fontSize: 10,
                    padding: "2px 6px",
                    borderRadius: 4,
                    fontWeight: 700,
                    background: isDaemonPaused ? "#744210" : "#22543d",
                    color: isDaemonPaused ? "#f6e05e" : "#68d391",
                  }}
                >
                  {isDaemonPaused ? "⏸️ DAEMON PAUSED" : "🟢 DAEMON RUNNING"}
                </span>
              </div>
              <span style={{ fontSize: 11, color: "#a0aec0" }}>
                Multi-job headless background render executor
              </span>
            </div>
          </div>
          <button
            onClick={onClose}
            style={{
              background: "transparent",
              border: "none",
              color: "#a0aec0",
              fontSize: 18,
              cursor: "pointer",
            }}
          >
            ✕
          </button>
        </header>

        {/* Toolbar */}
        <div
          style={{
            padding: "10px 16px",
            background: "#141822",
            borderBottom: "1px solid #2d3748",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div style={{ display: "flex", gap: 8 }}>
            <button
              type="button"
              onClick={handleTogglePause}
              style={{
                padding: "5px 10px",
                fontSize: 11,
                fontWeight: 600,
                background: "#2d3748",
                color: "#e2e8f0",
                border: "none",
                borderRadius: 4,
                cursor: "pointer",
              }}
            >
              {isDaemonPaused ? "▶️ Resume Daemon" : "⏸️ Pause Daemon"}
            </button>
            <button
              type="button"
              onClick={handleClearCompleted}
              style={{
                padding: "5px 10px",
                fontSize: 11,
                background: "#1a202c",
                color: "#a0aec0",
                border: "1px solid #2d3748",
                borderRadius: 4,
                cursor: "pointer",
              }}
            >
              🧹 Clear Completed
            </button>
          </div>
          <span style={{ fontSize: 11, color: "#718096" }}>{jobs.length} jobs in queue</span>
        </div>

        {/* Job List */}
        <div style={{ flex: 1, overflowY: "auto", padding: 16 }}>
          {jobs.length === 0 ? (
            <div style={{ textAlign: "center", padding: "40px 20px", color: "#718096" }}>
              <span style={{ fontSize: 32, display: "block", marginBottom: 8 }}>📭</span>
              <strong>Render Queue is Empty</strong>
              <p style={{ fontSize: 12, margin: "4px 0 0" }}>
                Enqueue timelines with presets to render in the background.
              </p>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {jobs.map((job) => (
                <div
                  key={job.id}
                  style={{
                    background: "#181c26",
                    border: "1px solid #2d3748",
                    borderRadius: 6,
                    padding: 12,
                  }}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 6 }}>
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 600, color: "#f7fafc" }}>
                        {job.label}
                      </div>
                      <div style={{ fontSize: 11, color: "#63b3ed", marginTop: 2 }}>
                        🏷️ {job.preset}
                      </div>
                    </div>
                    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                      <span
                        style={{
                          fontSize: 10,
                          fontWeight: 700,
                          padding: "2px 6px",
                          borderRadius: 3,
                          textTransform: "uppercase",
                          background:
                            job.status === "running"
                              ? "#2b6cb0"
                              : job.status === "completed"
                              ? "#22543d"
                              : job.status === "cancelled"
                              ? "#742a2a"
                              : "#2d3748",
                          color: "#ffffff",
                        }}
                      >
                        {job.status}
                      </span>
                      {job.status === "running" || job.status === "queued" ? (
                        <button
                          type="button"
                          onClick={() => handleCancelJob(job.id)}
                          title="Cancel Job"
                          style={{
                            background: "transparent",
                            border: "none",
                            color: "#e53e3e",
                            cursor: "pointer",
                            fontSize: 13,
                          }}
                        >
                          ✕
                        </button>
                      ) : null}
                    </div>
                  </div>

                  {/* Progress Bar */}
                  {job.status === "running" && (
                    <div style={{ margin: "8px 0 4px" }}>
                      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#a0aec0", marginBottom: 3 }}>
                        <span>{job.phase}</span>
                        <span>{job.progress}%</span>
                      </div>
                      <div style={{ height: 5, background: "#2d3748", borderRadius: 3, overflow: "hidden" }}>
                        <div
                          style={{
                            width: `${job.progress}%`,
                            height: "100%",
                            background: "#3182ce",
                            transition: "width 0.3s ease",
                          }}
                        />
                      </div>
                    </div>
                  )}

                  <div style={{ fontSize: 10, color: "#718096", marginTop: 4, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                    Destination: {job.outputPath}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Quick Enqueue Footer */}
        <footer
          style={{
            padding: 16,
            background: "#181b24",
            borderTop: "1px solid #2d3748",
          }}
        >
          <div style={{ fontSize: 11, fontWeight: 600, color: "#cbd5e0", marginBottom: 6 }}>
            Quick Enqueue Timeline:
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <select
              value={selectedPreset}
              onChange={(e) => setSelectedPreset(e.target.value)}
              style={{
                flex: 1,
                background: "#0d1117",
                border: "1px solid #4a5568",
                borderRadius: 4,
                color: "#f7fafc",
                fontSize: 12,
                padding: "6px 8px",
              }}
            >
              {exportPresets.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={handleAddJob}
              style={{
                padding: "6px 14px",
                background: "#3182ce",
                border: "none",
                borderRadius: 4,
                color: "#ffffff",
                fontWeight: 600,
                fontSize: 12,
                cursor: "pointer",
                whiteSpace: "nowrap",
              }}
            >
              ➕ Add to Queue
            </button>
          </div>
        </footer>
      </div>
    </div>
  );
};
