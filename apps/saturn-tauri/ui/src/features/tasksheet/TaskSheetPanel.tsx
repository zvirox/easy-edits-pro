// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import {
  FLAGSHIP_TEMPLATES,
  type SheetDeliverableState,
  type SheetStatus,
  type TaskSheetState,
} from "./types";

export interface TaskSheetPanelProps {
  sheet: TaskSheetState;
  onUpdateSheet: (sheet: TaskSheetState) => void;
  onSubmitSheet: (reason?: string) => void;
  onLoadTemplate: (templateId: string) => void;
  activeStatus?: SheetStatus;
}

export const TaskSheetPanel: React.FC<TaskSheetPanelProps> = ({
  sheet,
  onUpdateSheet,
  onSubmitSheet,
  onLoadTemplate,
  activeStatus = sheet.status,
}) => {
  const [collapsed, setCollapsed] = useState(false);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("");
  const [validationMsg, setValidationMsg] = useState<{ valid: boolean; text: string } | null>(null);

  const isEditable = activeStatus === "draft" || activeStatus === "ready";

  const handleTemplateChange = (tmplId: string) => {
    setSelectedTemplateId(tmplId);
    if (tmplId) {
      onLoadTemplate(tmplId);
      setValidationMsg({ valid: true, text: "Template loaded successfully." });
    }
  };

  const handleValidate = () => {
    if (!sheet.title.trim()) {
      setValidationMsg({ valid: false, text: "Title cannot be empty." });
      return;
    }
    if (sheet.deliverables.length === 0) {
      setValidationMsg({ valid: false, text: "Must have at least one deliverable." });
      return;
    }
    for (const d of sheet.deliverables) {
      if (!d.name.trim()) {
        setValidationMsg({ valid: false, text: `Deliverable ${d.id} has empty name.` });
        return;
      }
      if (d.duration && d.duration.target_s <= 0) {
        setValidationMsg({ valid: false, text: `Deliverable ${d.name} target duration must be positive.` });
        return;
      }
    }
    setValidationMsg({ valid: true, text: `Valid sheet: ${sheet.deliverables.length} deliverables configured.` });
  };

  const handleAddDeliverable = () => {
    const nextId = `deliv_${sheet.deliverables.length + 1}`;
    const newDeliv: SheetDeliverableState = {
      id: nextId,
      name: `New Deliverable ${sheet.deliverables.length + 1}`,
      preset: "web-1080p",
      duration: { target_s: 60, tolerance_s: 10 },
      loudness_lufs: -14,
      true_peak_dbtp: -1,
      captions: true,
      must_include: [],
    };
    onUpdateSheet({
      ...sheet,
      deliverables: [...sheet.deliverables, newDeliv],
    });
  };

  const handleRemoveDeliverable = (id: string) => {
    if (sheet.deliverables.length <= 1) return;
    onUpdateSheet({
      ...sheet,
      deliverables: sheet.deliverables.filter((d) => d.id !== id),
    });
  };

  const handleDeliverableChange = (id: string, patch: Partial<SheetDeliverableState>) => {
    onUpdateSheet({
      ...sheet,
      deliverables: sheet.deliverables.map((d) => (d.id === id ? { ...d, ...patch } : d)),
    });
  };

  const getStatusColor = (status: SheetStatus) => {
    switch (status) {
      case "ready":
      case "approved":
      case "delivered":
        return "var(--success-text, #10b981)";
      case "running":
      case "planning":
        return "var(--accent-text, #f472b6)";
      case "needs_input":
      case "awaiting_plan_approval":
      case "revision_requested":
        return "var(--warn-text, #f59e0b)";
      case "failed":
      case "cancelled":
        return "var(--error-text, #ef4444)";
      default:
        return "var(--text-muted, #9ca3af)";
    }
  };

  // Narrow or user-collapsed summary row
  if (collapsed) {
    return (
      <div
        className="tasksheet-panel-collapsed glass"
        data-elev="1"
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 12px",
          borderRadius: "var(--radius-sm, 10px)",
          margin: "8px",
          fontSize: "12px",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <span style={{ fontWeight: 600 }}>📋 {sheet.id}</span>
          <span
            style={{
              padding: "2px 6px",
              borderRadius: "4px",
              fontSize: "11px",
              fontWeight: 600,
              color: getStatusColor(activeStatus),
              border: `1px solid ${getStatusColor(activeStatus)}`,
            }}
          >
            {activeStatus.replace(/_/g, " ").toUpperCase()}
          </span>
          <span style={{ color: "var(--text-muted)" }}>
            {sheet.title} · {sheet.deliverables.length} deliv
          </span>
        </div>
        <InteriorButton size="sm" variant="ghost" onClick={() => setCollapsed(false)}>
          Expand
        </InteriorButton>
      </div>
    );
  }

  return (
    <div
      className="tasksheet-panel"
      role="region"
      aria-label="Task Sheet Work Order Panel"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "12px",
        padding: "12px",
        height: "100%",
        overflowY: "auto",
        fontSize: "13px",
      }}
    >
      {/* Header bar */}
      <div
        className="glass"
        data-elev="2"
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 12px",
          borderRadius: "var(--radius-sm, 10px)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <span style={{ fontWeight: 700, fontSize: "14px" }}>📋 Task Sheet</span>
          <span
            className="status-chip"
            style={{
              padding: "2px 8px",
              borderRadius: "4px",
              fontSize: "11px",
              fontWeight: 600,
              color: getStatusColor(activeStatus),
              border: `1px solid ${getStatusColor(activeStatus)}`,
            }}
          >
            {activeStatus.replace(/_/g, " ").toUpperCase()}
          </span>
        </div>
        <InteriorButton size="sm" variant="ghost" onClick={() => setCollapsed(true)} title="Collapse Sheet">
          Minimize
        </InteriorButton>
      </div>

      {/* Template picker */}
      {isEditable && (
        <div
          className="glass"
          data-elev="1"
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "6px",
            padding: "8px 12px",
            borderRadius: "var(--radius-sm, 10px)",
          }}
        >
          <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
            LOAD RECIPE TEMPLATE
          </label>
          <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
            {FLAGSHIP_TEMPLATES.map((tmpl) => (
              <InteriorButton
                key={tmpl.id}
                size="sm"
                variant={selectedTemplateId === tmpl.id ? "primary" : "secondary"}
                onClick={() => handleTemplateChange(tmpl.id)}
                title={tmpl.description}
              >
                {tmpl.label}
              </InteriorButton>
            ))}
          </div>
        </div>
      )}

      {/* Title & Requester */}
      <div
        className="glass"
        data-elev="1"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "8px",
          padding: "10px 12px",
          borderRadius: "var(--radius-sm, 10px)",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", gap: "8px" }}>
          <div style={{ flex: 1 }}>
            <label style={{ fontSize: "11px", color: "var(--text-muted)" }}>SHEET TITLE</label>
            <input
              type="text"
              value={sheet.title}
              disabled={!isEditable}
              onChange={(e) => onUpdateSheet({ ...sheet, title: e.target.value })}
              style={{
                width: "100%",
                background: "var(--surface-sunken, rgba(0,0,0,0.2))",
                border: "1px solid var(--border-subtle, rgba(255,255,255,0.1))",
                borderRadius: "var(--radius-xs, 6px)",
                color: "var(--text-primary)",
                padding: "4px 8px",
                fontSize: "13px",
              }}
            />
          </div>
          <div style={{ width: "120px" }}>
            <label style={{ fontSize: "11px", color: "var(--text-muted)" }}>REQUESTER</label>
            <div style={{ fontSize: "12px", fontWeight: 600, paddingTop: "4px" }}>
              {sheet.requester}
            </div>
          </div>
        </div>
      </div>

      {/* Deliverables Section */}
      <div
        className="glass"
        data-elev="1"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "8px",
          padding: "10px 12px",
          borderRadius: "var(--radius-sm, 10px)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
            DELIVERABLES ({sheet.deliverables.length})
          </label>
          {isEditable && (
            <InteriorButton size="sm" variant="ghost" onClick={handleAddDeliverable}>
              + Add
            </InteriorButton>
          )}
        </div>

        {sheet.deliverables.map((deliv, idx) => (
          <div
            key={deliv.id}
            style={{
              padding: "8px",
              background: "var(--surface-sunken, rgba(0,0,0,0.15))",
              border: "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
              borderRadius: "var(--radius-xs, 6px)",
              display: "flex",
              flexDirection: "column",
              gap: "6px",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontWeight: 600, fontSize: "12px" }}>
                #{idx + 1} {deliv.name}
              </span>
              {isEditable && sheet.deliverables.length > 1 && (
                <button
                  type="button"
                  onClick={() => handleRemoveDeliverable(deliv.id)}
                  style={{
                    background: "transparent",
                    border: "none",
                    color: "var(--warn-text, #f59e0b)",
                    cursor: "pointer",
                    fontSize: "11px",
                  }}
                >
                  ✕
                </button>
              )}
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "6px", fontSize: "11px" }}>
              <div>
                <span style={{ color: "var(--text-muted)" }}>Preset: </span>
                <select
                  value={deliv.preset}
                  disabled={!isEditable}
                  onChange={(e) => handleDeliverableChange(deliv.id, { preset: e.target.value })}
                  style={{
                    background: "var(--surface-sunken)",
                    color: "var(--text-primary)",
                    border: "1px solid var(--border-subtle)",
                    borderRadius: "4px",
                    padding: "2px",
                  }}
                >
                  <option value="vertical-9x16">Vertical 9:16</option>
                  <option value="web-1080p">Web 1080p</option>
                  <option value="broadcast-r128">Broadcast R128</option>
                </select>
              </div>

              <div>
                <span style={{ color: "var(--text-muted)" }}>Duration: </span>
                <span>{deliv.duration?.target_s ?? 60}s {deliv.duration?.tolerance_s ? `±${deliv.duration.tolerance_s}s` : ""}</span>
              </div>

              <div>
                <span style={{ color: "var(--text-muted)" }}>Target LUFS: </span>
                <span>{deliv.loudness_lufs ?? -14}</span>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Inputs Section */}
      <div
        className="glass"
        data-elev="1"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: "8px",
          padding: "10px 12px",
          borderRadius: "var(--radius-sm, 10px)",
        }}
      >
        <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
          INPUTS & ASSET ROLES ({sheet.inputs.length})
        </label>
        {sheet.inputs.length === 0 ? (
          <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>No inputs attached.</div>
        ) : (
          sheet.inputs.map((inp) => (
            <div
              key={inp.id}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                padding: "4px 8px",
                background: "var(--surface-sunken, rgba(0,0,0,0.1))",
                borderRadius: "4px",
                fontSize: "12px",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                <span>📁</span>
                <span>{inp.media}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>{inp.role}</span>
                {inp.trust === "untrusted" ? (
                  <span
                    style={{
                      fontSize: "10px",
                      color: "var(--warn-text, #f59e0b)",
                      padding: "1px 4px",
                      borderRadius: "3px",
                      border: "1px solid var(--warn-text, #f59e0b)",
                    }}
                  >
                    untrusted
                  </span>
                ) : (
                  <span
                    style={{
                      fontSize: "10px",
                      color: "var(--success-text, #10b981)",
                      padding: "1px 4px",
                      borderRadius: "3px",
                      border: "1px solid var(--success-text, #10b981)",
                    }}
                  >
                    trusted
                  </span>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Budget & Acceptance Section */}
      <div
        className="glass"
        data-elev="1"
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: "10px",
          padding: "10px 12px",
          borderRadius: "var(--radius-sm, 10px)",
        }}
      >
        <div>
          <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
            BUDGET LIMITS
          </label>
          <div style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginTop: "4px" }}>
            <div>⏱ Wall clock: {sheet.budget?.wall_clock_min ?? 60}m</div>
            <div>💲 Model cost: ${sheet.budget?.model_cost_usd?.toFixed(2) ?? "5.00"}</div>
            <div>⚡ Render limit: {sheet.budget?.render_min ?? 30}m</div>
          </div>
        </div>

        <div>
          <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
            ACCEPTANCE GATES
          </label>
          <div style={{ fontSize: "11px", display: "flex", flexDirection: "column", gap: "4px", marginTop: "4px" }}>
            {sheet.acceptance?.auto?.slice(0, 3).map((gate) => (
              <div key={gate} style={{ color: "var(--success-text, #10b981)" }}>✓ {gate}</div>
            )) ?? <div>Default QC gates</div>}
          </div>
        </div>
      </div>

      {/* Validation Message */}
      {validationMsg && (
        <div
          style={{
            padding: "6px 10px",
            borderRadius: "4px",
            fontSize: "12px",
            background: validationMsg.valid
              ? "var(--surface-sunken, rgba(16,185,129,0.1))"
              : "var(--surface-sunken, rgba(239,68,68,0.1))",
            color: validationMsg.valid
              ? "var(--success-text, #10b981)"
              : "var(--error-text, #ef4444)",
            border: `1px solid ${validationMsg.valid ? "var(--success-text, #10b981)" : "var(--error-text, #ef4444)"}`,
          }}
        >
          {validationMsg.text}
        </div>
      )}

      {/* Footer Action Bar */}
      <div style={{ display: "flex", gap: "8px", marginTop: "auto", paddingTop: "8px" }}>
        <InteriorButton size="sm" variant="secondary" onClick={handleValidate} title="Validate Sheet Integrity">
          Validate
        </InteriorButton>
        {isEditable && (
          <InteriorButton
            size="sm"
            variant="primary"
            onClick={() => onSubmitSheet("Submitted by operator")}
            title="Submit Sheet to Ready State"
            disabled={activeStatus === "ready"}
          >
            {activeStatus === "ready" ? "Ready for Agent" : "Submit Sheet"}
          </InteriorButton>
        )}
      </div>
    </div>
  );
};
