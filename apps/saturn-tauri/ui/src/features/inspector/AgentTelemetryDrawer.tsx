// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * AgentTelemetryDrawer (AF6 / Packet 5.2):
 * Pinned dock tab/section displaying live MCP tool feed, before/after telemetry deltas,
 * JSONL export, and "Save as skill" action with retained provenance.
 */
import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";

export interface McpToolCallRecord {
  id: string;
  timestamp: string;
  tool: string;
  tier: "read" | "edit" | "filesystem" | "admin";
  durationMs: number;
  status: "ok" | "error";
  summary: string;
  provenanceSource?: string;
}

export interface TelemetryDelta {
  metric: string;
  before: string | number;
  after: string | number;
  unit: string;
  passed: boolean;
}

export interface AgentTelemetryDrawerProps {
  toolCalls?: McpToolCallRecord[];
  deltas?: TelemetryDelta[];
  onExportJsonl?: () => void;
  onSaveAsSkill?: (skillName: string) => void;
}

const DEFAULT_TOOL_CALLS: McpToolCallRecord[] = [
  {
    id: "call-001",
    timestamp: "12:04:10.120",
    tool: "project.brief_get",
    tier: "read",
    durationMs: 8,
    status: "ok",
    summary: "Loaded project brief targets (web-1080p, -14 LUFS)",
  },
  {
    id: "call-002",
    timestamp: "12:04:11.450",
    tool: "audio.telemetry",
    tier: "read",
    durationMs: 42,
    status: "ok",
    summary: "Integrated loudness -18.4 LUFS, peak -0.2 dBTP",
  },
  {
    id: "call-003",
    timestamp: "12:04:13.820",
    tool: "timeline.split_clip",
    tier: "edit",
    durationMs: 15,
    status: "ok",
    summary: "Match cut aligned at transient peak tick 1200",
  },
  {
    id: "call-004",
    timestamp: "12:04:14.900",
    tool: "verify.run",
    tier: "read",
    durationMs: 88,
    status: "ok",
    summary: "Measured self-check: 4 pass, 1 warn, 0 fail",
  },
];

const DEFAULT_DELTAS: TelemetryDelta[] = [
  { metric: "Integrated Loudness", before: "-18.4", after: "-14.2", unit: "LUFS", passed: true },
  { metric: "Max True Peak", before: "+0.4", after: "-1.1", unit: "dBTP", passed: true },
  { metric: "Dead Air / Silence", before: "4.8", after: "0.2", unit: "s", passed: true },
  { metric: "Highlight Clipping", before: "14", after: "0", unit: "frames", passed: true },
  { metric: "Pacing Transients", before: "62%", after: "94%", unit: "aligned", passed: true },
];

export const AgentTelemetryDrawer: React.FC<AgentTelemetryDrawerProps> = ({
  toolCalls = DEFAULT_TOOL_CALLS,
  deltas = DEFAULT_DELTAS,
  onExportJsonl,
  onSaveAsSkill,
}) => {
  const [skillModalOpen, setSkillModalOpen] = useState(false);
  const [skillName, setSkillName] = useState("podcast-dialogue-enhancer");
  const [exportedNotice, setExportedNotice] = useState<string | null>(null);

  const handleExport = () => {
    if (onExportJsonl) {
      onExportJsonl();
    } else {
      // Local download fallback
      const jsonlData = toolCalls
        .map((call) =>
          JSON.stringify({
            timestamp: call.timestamp,
            tool: call.tool,
            tier: call.tier,
            duration_ms: call.durationMs,
            status: call.status,
            summary: call.summary,
            provenance: call.provenanceSource ? { untrusted: true, source: call.provenanceSource } : null,
          })
        )
        .join("\n");

      const blob = new Blob([jsonlData], { type: "application/x-ndjson" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `agent-audit-${Date.now()}.jsonl`;
      a.click();
      URL.revokeObjectURL(url);
    }
    setExportedNotice("Audit log exported to JSONL");
    setTimeout(() => setExportedNotice(null), 3000);
  };

  const handleSaveSkill = () => {
    if (!skillName.trim()) return;
    onSaveAsSkill?.(skillName.trim());
    setSkillModalOpen(false);
    setExportedNotice(`Saved workflow as skill: ${skillName}`);
    setTimeout(() => setExportedNotice(null), 3000);
  };

  return (
    <div
      className="agent-telemetry-drawer"
      role="region"
      aria-label="Agent Telemetry and Audit Drawer"
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        padding: "12px",
        gap: "12px",
        overflowY: "auto",
        fontSize: "12px",
      }}
    >
      {/* Top Header & Actions */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <span style={{ fontWeight: 600, fontSize: "13px" }}>⚡ Telemetry & Tool Audit</span>
          <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            AF2 Actor Audit · AF3 Measured Deltas
          </div>
        </div>

        <div style={{ display: "flex", gap: "6px" }}>
          <InteriorButton
            variant="secondary"
            size="sm"
            onClick={handleExport}
            title="Download session audit logs in JSONL format"
          >
            📥 Export JSONL
          </InteriorButton>
          <InteriorButton
            variant="secondary"
            size="sm"
            onClick={() => setSkillModalOpen(true)}
            title="Save successful recipe sequence as a reusable agent skill"
          >
            💾 Save as Skill
          </InteriorButton>
        </div>
      </div>

      {exportedNotice && (
        <div
          style={{
            padding: "4px 8px",
            borderRadius: "4px",
            background: "rgba(16, 185, 129, 0.15)",
            color: "var(--success-text, #10b981)",
            fontSize: "11px",
          }}
        >
          {exportedNotice}
        </div>
      )}

      {/* Before / After Telemetry Deltas */}
      <div
        className="glass"
        data-elev="1"
        style={{
          borderRadius: "var(--radius-sm, 8px)",
          padding: "10px",
          border: "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
        }}
      >
        <span style={{ fontWeight: 600, fontSize: "12px", display: "block", marginBottom: "8px" }}>
          Measured Telemetry Deltas
        </span>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "6px" }}>
          {deltas.map((d, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                flexDirection: "column",
                gap: "2px",
                padding: "6px 8px",
                borderRadius: "4px",
                background: "var(--bg-elev-1, rgba(255,255,255,0.02))",
                border: "1px solid var(--border-subtle, rgba(255,255,255,0.05))",
              }}
            >
              <span style={{ fontSize: "10px", color: "var(--text-muted)" }}>{d.metric}</span>
              <div style={{ display: "flex", alignItems: "baseline", gap: "4px", fontSize: "12px", fontWeight: 600 }}>
                <span style={{ color: "var(--text-muted)", textDecoration: "line-through" }}>
                  {d.before}
                </span>
                <span style={{ color: "var(--text-muted)" }}>→</span>
                <span style={{ color: d.passed ? "var(--success-text, #10b981)" : "var(--warn-text, #f59e0b)" }}>
                  {d.after} {d.unit}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Live MCP Tool Call Feed */}
      <div style={{ display: "flex", flexDirection: "column", gap: "6px", flex: 1 }}>
        <span style={{ fontWeight: 600, fontSize: "12px" }}>Live MCP Tool Invocations</span>

        <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          {toolCalls.map((c) => {
            const tierColor =
              c.tier === "admin" || c.tier === "filesystem"
                ? "var(--warn-text, #f59e0b)"
                : c.tier === "edit"
                ? "var(--accent-text, #f472b6)"
                : "var(--accent-text-cyan, #38bdf8)";

            return (
              <div
                key={c.id}
                className="glass"
                data-elev="1"
                style={{
                  padding: "8px 10px",
                  borderRadius: "6px",
                  border: "1px solid var(--border-subtle, rgba(255,255,255,0.06))",
                  display: "flex",
                  flexDirection: "column",
                  gap: "4px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                    <code style={{ fontWeight: 600, color: "var(--text-primary)" }}>{c.tool}</code>
                    <span
                      style={{
                        fontSize: "9px",
                        padding: "1px 4px",
                        borderRadius: "3px",
                        border: `1px solid ${tierColor}`,
                        color: tierColor,
                        textTransform: "uppercase",
                      }}
                    >
                      {c.tier}
                    </span>
                  </div>
                  <div style={{ fontSize: "10px", color: "var(--text-muted)", display: "flex", gap: "6px" }}>
                    <span>{c.durationMs}ms</span>
                    <span>{c.timestamp}</span>
                  </div>
                </div>

                <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>{c.summary}</div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Save As Skill Mini Dialog */}
      {skillModalOpen && (
        <div
          style={{
            padding: "10px",
            borderRadius: "6px",
            background: "var(--bg-elev-2, rgba(255,255,255,0.06))",
            border: "1px solid var(--border-subtle, rgba(255,255,255,0.12))",
            display: "flex",
            flexDirection: "column",
            gap: "8px",
          }}
        >
          <span style={{ fontWeight: 600 }}>Save Workflow as Skill</span>
          <p style={{ margin: 0, fontSize: "11px", color: "var(--text-muted)" }}>
            Persist successful sequence recipes with retained AF5 provenance into <code>.agents/skills/</code>.
          </p>
          <input
            type="text"
            value={skillName}
            onChange={(e) => setSkillName(e.target.value)}
            style={{
              padding: "4px 8px",
              borderRadius: "4px",
              background: "var(--bg-elev-1, rgba(255,255,255,0.03))",
              border: "1px solid var(--border-subtle, rgba(255,255,255,0.1))",
              color: "var(--text-primary)",
              fontSize: "12px",
            }}
          />
          <div style={{ display: "flex", justifyContent: "flex-end", gap: "6px" }}>
            <InteriorButton variant="secondary" size="sm" onClick={() => setSkillModalOpen(false)}>
              Cancel
            </InteriorButton>
            <InteriorButton variant="primary" size="sm" onClick={handleSaveSkill}>
              Save Skill
            </InteriorButton>
          </div>
        </div>
      )}
    </div>
  );
};

export default AgentTelemetryDrawer;
