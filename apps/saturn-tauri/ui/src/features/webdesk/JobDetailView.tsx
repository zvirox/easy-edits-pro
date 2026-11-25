// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Web Desk Job Detail View (TS8).
 * Displays task sheet specifications, plan approval card, live agent run log, and questions inbox.
 */

import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { AgentInbox } from "../tasksheet/AgentInbox";
import type { useWebDeskStore } from "./webDeskStore";
import type { SheetStatus, WebDeskJob } from "./types";

interface JobDetailViewProps {
  store: ReturnType<typeof useWebDeskStore>;
  job: WebDeskJob;
}

function getStatusColor(status: SheetStatus): string {
  switch (status) {
    case "ready":
    case "approved":
    case "delivered":
      return "var(--accent-text-hover)";
    case "running":
    case "planning":
      return "var(--accent-text)";
    case "needs_input":
    case "awaiting_plan_approval":
      return "var(--accent-text-violet)";
    case "in_review":
    case "revision_requested":
      return "var(--accent-text)";
    case "failed":
    case "stopped":
    case "cancelled":
      return "var(--text-muted)";
    case "draft":
    default:
      return "var(--text-muted)";
  }
}

export const JobDetailView: React.FC<JobDetailViewProps> = ({ store, job }) => {
  const {
    setActiveView,
    approvePlan,
    rejectPlan,
    answerQuestion,
  } = store;

  const [rejectReason, setRejectReason] = useState("");
  const [showRejectInput, setShowRejectInput] = useState(false);

  const statusColor = getStatusColor(job.status);
  const plan = job.plan;
  const isPlanPending = Boolean(
    plan &&
      !plan.approved_at_unix_ms &&
      (job.status === "ready" ||
        job.status === "planning" ||
        job.status === "awaiting_plan_approval")
  );

  const openQuestions = job.questions.filter((q) => q.status === "open");

  const handleApprovePlan = async () => {
    await approvePlan(job.id);
  };

  const handleRejectPlan = async () => {
    if (!rejectReason.trim()) return;
    await rejectPlan(job.id, rejectReason);
    setShowRejectInput(false);
    setRejectReason("");
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", gap: "var(--gap, 12px)", overflow: "hidden" }}>
      {/* Header bar */}
      <div
        className="surface-1" style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "10px 16px",
          borderRadius: "var(--radius, 14px)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <InteriorButton
            size="sm" variant="ghost" onClick={() => setActiveView("jobs")}
            data-testid="back-to-jobs-btn"
          >
            ← Back to Jobs
          </InteriorButton>
          <span style={{ fontWeight: 700, fontSize: "15px" }}>{job.title}</span>
          <span
            className="status-chip" style={{
              padding: "2px 8px",
              borderRadius: "4px",
              fontSize: "11px",
              fontWeight: 700,
              color: statusColor,
              border: `1px solid ${statusColor}`,
              textTransform: "uppercase",
            }}
            data-testid="job-detail-status-chip"
          >
            {job.status.replace(/_/g, " ").toUpperCase()}
          </span>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            Requester: <strong>{job.requester}</strong>
          </span>
          <InteriorButton
            size="sm" variant="secondary" onClick={() => setActiveView("review")}
            data-testid="open-review-btn"
          >
             Open Review →
          </InteriorButton>
        </div>
      </div>

      {/* Main detail content grid */}
      <div className="webdesk-detail-layout">
        {/* Left column: Task Sheet Spec & Plan Approval Card */}
        <div className="webdesk-detail-col">
          {/* Plan Approval Card */}
          {plan && (
            <div
              className="surface-1" style={{
                padding: "14px",
                borderRadius: "var(--radius, 14px)",
                display: "flex",
                flexDirection: "column",
                gap: "10px",
                border: "1px solid var(--glass-border)",
              }}
              data-testid="plan-approval-card"
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <span style={{ fontSize: "13px", fontWeight: 700 }}>
                   Agent Execution Plan {plan.approved_at_unix_ms ? "(Approved)" : "(Pending Approval)"}
                </span>
                {plan.approved_at_unix_ms ? (
                  <span style={{ fontSize: "11px", color: "var(--accent-text-hover)", fontWeight: 600 }}>
                    ✓ Approved by {plan.approved_by}
                  </span>
                ) : (
                  <span style={{ fontSize: "11px", color: "var(--accent-text)", fontWeight: 600 }}>
                    Awaiting Supervisor Approval
                  </span>
                )}
              </div>

              {/* Plan steps list */}
              <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                {plan.steps.map((st) => (
                  <div
                    key={st.id}
                    className="surface-1" style={{
                      padding: "8px 12px",
                      borderRadius: "var(--radius-sm, 8px)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      fontSize: "12px",
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                      <span style={{ color: "var(--text-muted)", fontFamily: "monospace" }}>#{st.id}</span>
                      <span>{st.title}</span>
                    </div>
                    <span
                      style={{
                        fontSize: "10px",
                        fontWeight: 600,
                        textTransform: "uppercase",
                        color: st.status === "completed" ? "var(--accent-text-hover)" : "var(--text-muted)",
                      }}
                    >
                      {st.status}
                    </span>
                  </div>
                ))}
              </div>

              {/* Approval / Rejection buttons */}
              {isPlanPending && (
                <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginTop: "4px" }}>
                  {!showRejectInput ? (
                    <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
                      <InteriorButton
                        size="sm" variant="secondary" onClick={() => setShowRejectInput(true)}
                        data-testid="reject-plan-btn"
                      >
                        Reject / Request Changes
                      </InteriorButton>
                      <InteriorButton
                        size="sm" variant="primary" onClick={handleApprovePlan}
                        data-testid="approve-plan-btn"
                      >
                        Approve Plan & Run
                      </InteriorButton>
                    </div>
                  ) : (
                    <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                      <input
                        type="text" className="webdesk-search-input" placeholder="State reason for rejecting plan..." value={rejectReason}
                        onChange={(e) => setRejectReason(e.target.value)}
                        aria-label="Rejection Reason"
                      />
                      <div style={{ display: "flex", gap: "6px", justifyContent: "flex-end" }}>
                        <InteriorButton size="sm" variant="ghost" onClick={() => setShowRejectInput(false)}>
                          Cancel
                        </InteriorButton>
                        <InteriorButton size="sm" variant="primary" onClick={handleRejectPlan}>
                          Confirm Rejection
                        </InteriorButton>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Task Sheet Specification Card */}
          <div
            className="surface-1" style={{
              padding: "14px",
              borderRadius: "var(--radius, 14px)",
              display: "flex",
              flexDirection: "column",
              gap: "12px",
            }}
            data-testid="tasksheet-specs-card"
          >
            <span style={{ fontSize: "13px", fontWeight: 700 }}>
               Task Sheet Specification ({job.id})
            </span>

            {/* Deliverables Targets */}
            <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
              <span style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase" }}>
                Deliverables ({job.deliverables.length})
              </span>
              {job.deliverables.map((deliv) => (
                <div
                  key={deliv.id}
                  className="surface-1" style={{
                    padding: "8px 10px",
                    borderRadius: "var(--radius-sm, 6px)",
                    fontSize: "12px",
                    display: "flex",
                    justifyContent: "space-between",
                  }}
                >
                  <span style={{ fontWeight: 600 }}>{deliv.name}</span>
                  <span style={{ color: "var(--text-muted)" }}>
                    {deliv.preset} · {deliv.duration?.target_s ?? 60}s · {deliv.loudness_lufs ?? -14} LUFS
                  </span>
                </div>
              ))}
            </div>

            {/* Media Inputs */}
            <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
              <span style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase" }}>
                Inputs ({job.inputs.length})
              </span>
              {job.inputs.map((inp) => (
                <div
                  key={inp.id}
                  className="surface-1" style={{
                    padding: "8px 10px",
                    borderRadius: "var(--radius-sm, 6px)",
                    fontSize: "12px",
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                  }}
                >
                  <span>{inp.media} ({inp.role})</span>
                  <span
                    style={{
                      fontSize: "10px",
                      fontWeight: 600,
                      color: inp.trust === "untrusted" ? "var(--accent-text)" : "var(--accent-text-hover)",
                    }}
                  >
                    {inp.trust === "untrusted" ? "UNTRUSTED TRANSCRIPT" : "TRUSTED MEDIA"}
                  </span>
                </div>
              ))}
            </div>

            {/* Budget Meters */}
            <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
              <span style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase" }}>
                Budget Limits
              </span>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "8px", fontSize: "12px" }}>
                <div className="surface-1" style={{ padding: "6px 8px", borderRadius: "6px" }}>
                  <div style={{ color: "var(--text-muted)", fontSize: "10px" }}>TIME</div>
                  <div style={{ fontWeight: 700 }}>{job.budget?.wall_clock_min ?? 60} min</div>
                </div>
                <div className="surface-1" style={{ padding: "6px 8px", borderRadius: "6px" }}>
                  <div style={{ color: "var(--text-muted)", fontSize: "10px" }}>MODEL COST</div>
                  <div style={{ fontWeight: 700 }}>${(job.budget?.model_cost_usd ?? 0).toFixed(2)}</div>
                </div>
                <div className="surface-1" style={{ padding: "6px 8px", borderRadius: "6px" }}>
                  <div style={{ color: "var(--text-muted)", fontSize: "10px" }}>RENDER</div>
                  <div style={{ fontWeight: 700 }}>{job.budget?.render_min ?? 30} min</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Right column: Questions Inbox & Live Agent Run Log */}
        <div className="webdesk-detail-col">
          {/* Questions Inbox */}
          {openQuestions.length > 0 && (
            <div
              className="surface-1" style={{
                padding: "14px",
                borderRadius: "var(--radius, 14px)",
                border: "1px solid var(--accent-text-violet)",
              }}
              data-testid="questions-inbox-section"
            >
              <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}>
                <span style={{ fontSize: "14px" }}>❓</span>
                <span style={{ fontSize: "13px", fontWeight: 700, color: "var(--accent-text-violet)" }}>
                  Agent Question Requiring Input ({openQuestions.length})
                </span>
              </div>
              <AgentInbox
                questions={job.questions}
                onAnswer={(qId, optId) => answerQuestion(job.id, qId, optId)}
              />
            </div>
          )}

          {/* Live Agent Run Log */}
          <div
            className="surface-1" style={{
              padding: "14px",
              borderRadius: "var(--radius, 14px)",
              flex: 1,
              display: "flex",
              flexDirection: "column",
              gap: "8px",
              minHeight: "240px",
            }}
            data-testid="live-agent-run-log"
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <span style={{ fontSize: "13px", fontWeight: 700 }}>
                Live Agent Execution Log ({job.runLog.length} events)
              </span>
              <span style={{ fontSize: "10px", color: "var(--text-muted)" }}>
                Audited & Tier-Enforced (AF9)
              </span>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: "6px", overflowY: "auto", flex: 1 }}>
              {job.runLog.map((log) => (
                <div
                  key={log.id}
                  className="surface-1" style={{
                    padding: "8px 10px",
                    borderRadius: "var(--radius-sm, 6px)",
                    fontSize: "12px",
                    display: "flex",
                    flexDirection: "column",
                    gap: "4px",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                      <span style={{ fontFamily: "monospace", fontWeight: 600, color: "var(--accent-text)" }}>
                        {log.tool}
                      </span>
                      <span
                        style={{
                          fontSize: "9px",
                          fontWeight: 700,
                          padding: "1px 4px",
                          borderRadius: "3px",
                          border: "1px solid var(--glass-border)",
                          textTransform: "uppercase",
                          color: "var(--text-muted)",
                        }}
                      >
                        {log.tier}
                      </span>
                    </div>
                    <span style={{ fontSize: "10px", color: "var(--text-muted)" }}>{log.actor}</span>
                  </div>
                  {log.rationale && (
                    <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                      {log.rationale}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
