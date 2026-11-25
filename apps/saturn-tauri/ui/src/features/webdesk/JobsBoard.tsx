// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Web Desk Jobs Board (TS8).
 * Displays jobs in Kanban columns or table view with filters, metrics, and lifecycle actions.
 */

import React, { useMemo } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import type { useWebDeskStore } from "./webDeskStore";
import type { SheetStatus, WebDeskJob } from "./types";

interface JobsBoardProps {
  store: ReturnType<typeof useWebDeskStore>;
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

interface KanbanColumnConfig {
  id: string;
  title: string;
  statuses: SheetStatus[];
}

const KANBAN_COLUMNS: KanbanColumnConfig[] = [
  {
    id: "ready",
    title: "Draft & Ready",
    statuses: ["draft", "ready"],
  },
  {
    id: "running",
    title: "Running & Input",
    statuses: ["planning", "awaiting_plan_approval", "running", "needs_input"],
  },
  {
    id: "review",
    title: "In Review",
    statuses: ["in_review", "revision_requested"],
  },
  {
    id: "delivered",
    title: "Approved & Delivered",
    statuses: ["approved", "delivered"],
  },
];

export const JobsBoard: React.FC<JobsBoardProps> = ({ store }) => {
  const {
    jobs,
    activeJobId,
    selectJob,
    setActiveView,
    boardMode,
    setBoardMode,
    filter,
    setFilter,
    metrics,
  } = store;

  // Filtered jobs
  const filteredJobs = useMemo(() => {
    return jobs.filter((job) => {
      if (filter.status !== "all" && job.status !== filter.status) {
        return false;
      }
      if (filter.requester !== "all" && job.requester !== filter.requester) {
        return false;
      }
      if (filter.search.trim()) {
        const q = filter.search.toLowerCase();
        const titleMatch = job.title.toLowerCase().includes(q);
        const reqMatch = job.requester.toLowerCase().includes(q);
        const templateMatch = (job.templateId ?? "").toLowerCase().includes(q);
        if (!titleMatch && !reqMatch && !templateMatch) {
          return false;
        }
      }
      return true;
    });
  }, [jobs, filter]);

  const handleCardClick = (job: WebDeskJob) => {
    selectJob(job.id);
    setActiveView("detail");
  };

  const handleReviewClick = (e: React.MouseEvent, job: WebDeskJob) => {
    e.stopPropagation();
    selectJob(job.id);
    setActiveView("review");
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", gap: "var(--gap, 12px)" }}>
      {/* Metrics strip */}
      <div className="webdesk-metrics-strip" data-testid="webdesk-metrics-strip">
        <div className="webdesk-metric-card surface-1">
          <span className="webdesk-metric-label">Total Jobs</span>
          <span className="webdesk-metric-value">{metrics.totalJobs}</span>
        </div>
        <div className="webdesk-metric-card surface-1">
          <span className="webdesk-metric-label">Active Running</span>
          <span className="webdesk-metric-value">{metrics.activeJobs}</span>
        </div>
        <div className="webdesk-metric-card surface-1">
          <span className="webdesk-metric-label">In Review / Input</span>
          <span className="webdesk-metric-value">{metrics.inReviewCount + metrics.needsInputCount}</span>
        </div>
        <div className="webdesk-metric-card surface-1">
          <span className="webdesk-metric-label">Delivered</span>
          <span className="webdesk-metric-value">{metrics.completedJobs}</span>
        </div>
        <div className="webdesk-metric-card surface-1">
          <span className="webdesk-metric-label">Est. Model Cost</span>
          <span className="webdesk-metric-value">${metrics.totalModelCostUsd.toFixed(2)}</span>
        </div>
        <div className="webdesk-metric-card surface-1">
          <span className="webdesk-metric-label">Wall-Clock Time</span>
          <span className="webdesk-metric-value">{metrics.totalWallClockMin}m</span>
        </div>
      </div>

      {/* Controls & Filter Bar */}
      <div className="webdesk-board-controls">
        <div className="webdesk-filter-group">
          <input
            type="text" className="webdesk-search-input" placeholder="Search jobs" value={filter.search}
            onChange={(e) => setFilter({ ...filter, search: e.target.value })}
            data-testid="search-jobs-input" aria-label="Search jobs"
          />
          <select
            className="webdesk-select" value={filter.status}
            onChange={(e) => setFilter({ ...filter, status: e.target.value as SheetStatus | "all" })}
            data-testid="filter-status-select" aria-label="Filter jobs by status"
          >
            <option value="all">All States</option>
            <option value="draft">Draft</option>
            <option value="ready">Ready</option>
            <option value="planning">Planning</option>
            <option value="awaiting_plan_approval">Awaiting Plan</option>
            <option value="running">Running</option>
            <option value="needs_input">Needs Input</option>
            <option value="in_review">In Review</option>
            <option value="revision_requested">Revision Requested</option>
            <option value="approved">Approved</option>
            <option value="delivered">Delivered</option>
          </select>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <div style={{ display: "flex", borderRadius: "var(--radius-sm, 8px)", overflow: "hidden", border: "1px solid var(--glass-border)" }}>
            <button
              type="button" className={`webdesk-nav-btn ${boardMode === "kanban" ? "active" : ""}`}
              onClick={() => setBoardMode("kanban")}
              data-testid="view-kanban-btn"
            >
              Columns
            </button>
            <button
              type="button" className={`webdesk-nav-btn ${boardMode === "list" ? "active" : ""}`}
              onClick={() => setBoardMode("list")}
              data-testid="view-list-btn"
            >
              Table
            </button>
          </div>

          <InteriorButton
            size="sm" variant="primary" onClick={() => setActiveView("new_job")}
            data-testid="create-job-btn"
          >
            New Job
          </InteriorButton>
        </div>
      </div>

      {/* Main Board Area */}
      {filteredJobs.length === 0 ? (
        <div
          className="surface-1" style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: "12px",
            borderRadius: "var(--radius, 14px)",
            padding: "32px",
            textAlign: "center",
          }}
          data-testid="empty-jobs-board"
        >
          <div style={{ fontSize: "15px", fontWeight: 500 }}>No jobs yet</div>
          <div style={{ fontSize: "12px", color: "var(--text-muted)", maxWidth: "340px" }}>
            Start from a recipe to create your first job.
          </div>
          <InteriorButton
            size="md" variant="primary" onClick={() => setActiveView("new_job")}
          >
            Create a job
          </InteriorButton>
        </div>
      ) : boardMode === "kanban" ? (
        /* Kanban View */
        <div className="webdesk-kanban-grid" data-testid="kanban-grid">
          {KANBAN_COLUMNS.map((col) => {
            const colJobs = filteredJobs.filter((j) => col.statuses.includes(j.status));
            return (
              <div key={col.id} className="webdesk-kanban-col surface-1">
                <div className="webdesk-kanban-col-header">
                  <span>{col.title}</span>
                  <span className="webdesk-col-count">{colJobs.length}</span>
                </div>
                <div className="webdesk-kanban-col-body">
                  {colJobs.map((job) => {
                    const statusColor = getStatusColor(job.status);
                    return (
                      <div
                        key={job.id}
                        className={`webdesk-job-card surface-1 ${job.id === activeJobId ? "active" : ""}`} onClick={() => handleCardClick(job)}
                        data-testid={`job-card-${job.id}`}
                      >
                        <div className="webdesk-job-card-header">
                          <span className="webdesk-job-card-title">{job.title}</span>
                          <span
                            className="status-chip" style={{
                              padding: "2px 6px",
                              borderRadius: "4px",
                              fontSize: "10px",
                              fontWeight: 700,
                              color: statusColor,
                              border: `1px solid ${statusColor}`,
                              textTransform: "uppercase",
                              whiteSpace: "nowrap",
                            }}
                          >
                            {job.status.replace(/_/g, " ").toUpperCase()}
                          </span>
                        </div>

                        <div className="webdesk-job-card-meta">
                          <span> {job.requester}</span>
                          <span> {job.deliverables.length} deliv</span>
                        </div>

                        {job.templateId && (
                          <div style={{ fontSize: "11px", color: "var(--accent-text)" }}>
                             {job.templateId.replace(/_/g, " ")}
                          </div>
                        )}

                        <div className="webdesk-job-card-actions">
                          <InteriorButton
                            size="sm" variant="ghost" onClick={(e) => {
                              e.stopPropagation();
                              handleCardClick(job);
                            }}
                          >
                            Detail
                          </InteriorButton>
                          {(job.status === "in_review" || job.status === "approved" || job.status === "delivered" || (job.rounds && job.rounds.length > 0)) && (
                            <InteriorButton
                              size="sm" variant="secondary" onClick={(e) => handleReviewClick(e, job)}
                              data-testid={`review-btn-${job.id}`}
                            >
                               Review
                            </InteriorButton>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        /* Table / List View */
        <div className="webdesk-table-container surface-1">
          <table className="webdesk-jobs-table">
            <thead>
              <tr>
                <th>Job Title</th>
                <th>Status</th>
                <th>Recipe</th>
                <th>Requester</th>
                <th>Deliverables</th>
                <th>Est. Cost</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredJobs.map((job) => {
                const statusColor = getStatusColor(job.status);
                return (
                  <tr key={job.id} onClick={() => handleCardClick(job)}>
                    <td style={{ fontWeight: 600 }}>{job.title}</td>
                    <td>
                      <span
                        className="status-chip" style={{
                          padding: "2px 6px",
                          borderRadius: "4px",
                          fontSize: "10px",
                          fontWeight: 700,
                          color: statusColor,
                          border: `1px solid ${statusColor}`,
                          textTransform: "uppercase",
                        }}
                      >
                        {job.status.replace(/_/g, " ").toUpperCase()}
                      </span>
                    </td>
                    <td>{job.templateId ? job.templateId.replace(/_/g, " ") : "Custom"}</td>
                    <td style={{ color: "var(--text-muted)" }}>{job.requester}</td>
                    <td>{job.deliverables.length}</td>
                    <td>${(job.budget?.model_cost_usd ?? 0).toFixed(2)}</td>
                    <td>
                      <div style={{ display: "flex", gap: "6px" }}>
                        <InteriorButton
                          size="sm" variant="ghost" onClick={(e) => {
                            e.stopPropagation();
                            handleCardClick(job);
                          }}
                        >
                          Detail
                        </InteriorButton>
                        <InteriorButton
                          size="sm" variant="secondary" onClick={(e) => handleReviewClick(e, job)}
                        >
                          Review
                        </InteriorButton>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};
