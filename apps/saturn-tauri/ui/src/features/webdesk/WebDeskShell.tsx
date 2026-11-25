// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Web Desk Shell (TS8).
 * Provides the application shell, navigation, daemon connection indicator,
 * and prominent AGPL §13 Source and Licence notice.
 */

import React from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { useWebDeskStore } from "./webDeskStore";
import { JobsBoard } from "./JobsBoard";
import { NewJobModal } from "./NewJobModal";
import { JobDetailView } from "./JobDetailView";
import { WebReviewView } from "./WebReviewView";

export const WebDeskShell: React.FC = () => {
  const store = useWebDeskStore();
  const {
    activeView,
    setActiveView,
    activeJob,
    isAgplNoticeOpen,
    setIsAgplNoticeOpen,
  } = store;

  return (
    <div className="webdesk-root" data-testid="webdesk-root">
      {/* Shell Header */}
      <header className="webdesk-header surface-1">
        <div className="webdesk-brand">
          <span className="webdesk-title"> Easy Edits Pro</span>
          <span className="webdesk-badge">Web Desk (B2B Self-Host)</span>
          <div className="webdesk-daemon-indicator" title="Connected to saturn-control daemon">
            <span className="webdesk-daemon-dot" />
            <span>Daemon: Connected</span>
          </div>
        </div>

        {/* View Navigation */}
        <nav className="webdesk-nav" aria-label="Web Desk Navigation">
          <button
            type="button" className={`webdesk-nav-btn ${activeView === "jobs" ? "active" : ""}`}
            onClick={() => setActiveView("jobs")}
            data-testid="nav-jobs"
          >
             Jobs Board
          </button>
          <button
            type="button" className={`webdesk-nav-btn ${activeView === "new_job" ? "active" : ""}`}
            onClick={() => setActiveView("new_job")}
            data-testid="nav-new-job"
          >
            New Job
          </button>
          {activeJob && (
            <>
              <button
                type="button" className={`webdesk-nav-btn ${activeView === "detail" ? "active" : ""}`}
                onClick={() => setActiveView("detail")}
                data-testid="nav-detail"
              >
                 Job Detail
              </button>
              <button
                type="button" className={`webdesk-nav-btn ${activeView === "review" ? "active" : ""}`}
                onClick={() => setActiveView("review")}
                data-testid="nav-review"
              >
                 Review
              </button>
            </>
          )}
        </nav>

        {/* Header Actions & AGPL Notice Link */}
        <div className="webdesk-header-actions">
          <a
            href="#agpl-license" role="link" className="webdesk-agpl-link" onClick={(e) => {
              e.preventDefault();
              setIsAgplNoticeOpen(true);
            }}
            data-testid="agpl-license-link"
          >
            Source and licence
          </a>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="webdesk-content">
        {activeView === "jobs" && <JobsBoard store={store} />}
        {activeView === "new_job" && (
          <NewJobModal
            store={store}
            onClose={() => setActiveView("jobs")}
          />
        )}
        {activeView === "detail" && activeJob && (
          <JobDetailView store={store} job={activeJob} />
        )}
        {activeView === "review" && activeJob && (
          <WebReviewView store={store} job={activeJob} />
        )}
        {/* If in detail or review but no job selected, return to jobs */}
        {(activeView === "detail" || activeView === "review") && !activeJob && (
          <JobsBoard store={store} />
        )}
      </main>

      {/* AGPL §13 Source and Licence Modal */}
      {isAgplNoticeOpen && (
        <div
          className="webdesk-modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="agpl-modal-title" data-testid="agpl-license-modal"
        >
          <div className="webdesk-modal surface-1">
            <div className="webdesk-modal-header">
              <h2 id="agpl-modal-title" className="webdesk-modal-title">
                ⚖ Source and Licence (AGPL §13)
              </h2>
              <InteriorButton
                size="sm" variant="ghost" onClick={() => setIsAgplNoticeOpen(false)}
                aria-label="Close Licence Notice"
              >
                ✕
              </InteriorButton>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: "12px", fontSize: "13px", lineHeight: "1.5" }}>
              <p>
                <strong>Easy Edits Pro (EEP) Web Desk</strong> is free software licensed under the{" "}
                <strong>GNU Affero General Public License version 3 (AGPL-3.0-or-later)</strong>.
              </p>

              <div
                className="surface-1" style={{
                  padding: "10px 14px",
                  borderRadius: "var(--radius-sm, 8px)",
                  border: "1px solid var(--glass-border)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "6px",
                }}
              >
                <div style={{ fontWeight: 600, color: "var(--accent-text)" }}>
                  AGPL §13 Remote Network Interaction Requirement:
                </div>
                <div style={{ color: "var(--text-muted)" }}>
                  If you run a modified version of this software on a server and provide access to users over a computer network,
                  you must make the Corresponding Source available to all users through a network service, without charge.
                </div>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                <span style={{ fontWeight: 600 }}>Source Code Repository:</span>
                <a
                  href="https://github.com/zvirox/easy-edits-pro" target="_blank" rel="noreferrer" style={{ color: "var(--accent-text)", wordBreak: "break-all" }}
                >
                  https://github.com/zvirox/easy-edits-pro
                </a>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                <span style={{ fontWeight: 600 }}>Commercial Self-Hosting / Dual Licensing:</span>
                <span style={{ color: "var(--text-muted)" }}>
                  Enterprise and commercial licensees who require private modifications or proprietary integrations
                  may obtain a commercial license under the EEP Commercial Licence. See LICENSING.md for terms.
                </span>
              </div>
            </div>

            <div style={{ display: "flex", justifyContent: "flex-end", marginTop: "12px" }}>
              <InteriorButton
                size="md" variant="primary" onClick={() => setIsAgplNoticeOpen(false)}
              >
                Understood
              </InteriorButton>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
