// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import type { AgentSessionState, ProjectBriefState } from "./types";

export interface AgentCardProps {
  session: AgentSessionState;
  onStop: () => void;
  onResume: () => void;
  onUpdateBrief: (brief: ProjectBriefState) => void;
  onAllowPermission: (reqId: string, mode: "once" | "session") => void;
  onDenyPermission: (reqId: string) => void;
  onJumpToTick: (tick: number) => void;
  onSendInstruction?: (instruction: string) => void;
}

export const AgentCard: React.FC<AgentCardProps> = ({
  session,
  onStop,
  onResume,
  onUpdateBrief,
  onAllowPermission,
  onDenyPermission,
  onJumpToTick,
  onSendInstruction,
}) => {
  const [instruction, setInstruction] = useState("");
  const [briefOpen, setBriefOpen] = useState(false);
  const [briefDraft, setBriefDraft] = useState<ProjectBriefState>(session.brief);
  const [previewStripOpen, setPreviewStripOpen] = useState(false);

  const isStopped = session.isStopped || session.status === "stopped";
  const isWorking = session.status === "thinking" || session.status === "editing";

  const handleSend = () => {
    if (!instruction.trim()) return;
    onSendInstruction?.(instruction.trim());
    setInstruction("");
  };

  const handleSaveBrief = () => {
    onUpdateBrief(briefDraft);
    setBriefOpen(false);
  };

  return (
    <div
      className="agent-card"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--gap, 12px)",
        padding: "12px",
        height: "100%",
        overflowY: "auto",
        fontSize: "13px",
      }}
    >
      {/* Pinned Top Status Line */}
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
          <span
            style={{
              width: "10px",
              height: "10px",
              borderRadius: "50%",
              backgroundColor: isStopped
                ? "var(--warn-text, #f59e0b)"
                : isWorking
                ? "var(--accent-text, #f472b6)"
                : "var(--text-muted, #9ca3af)",
              boxShadow: isWorking ? "0 0 8px var(--accent-text)" : undefined,
            }}
          />
          <div>
            <div style={{ fontWeight: 600, fontSize: "12px" }}>
              {isStopped
                ? "Session Stopped"
                : isWorking
                ? `Running (Step ${session.stepCurrent}/${session.stepTotal})`
                : "Agent Idle"}
            </div>
            <div style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>
              {session.currentAction} · {session.elapsedSeconds}s
            </div>
          </div>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          {isStopped ? (
            <InteriorButton variant="secondary" size="sm" onClick={onResume}>
              Resume
            </InteriorButton>
          ) : (
            <InteriorButton
              variant="secondary"
              size="sm"
              onClick={onStop}
              title="Immediate Stop: halts active session (≤ 1s, no late apply)"
            >
              Stop
            </InteriorButton>
          )}
        </div>
      </div>

      {/* Permission Requests Banner */}
      {session.pendingPermissions.length > 0 && (
        <div
          className="glass"
          data-elev="3"
          style={{
            padding: "10px 12px",
            borderRadius: "var(--radius-sm, 10px)",
            borderLeft: "3px solid var(--accent-text)",
            display: "flex",
            flexDirection: "column",
            gap: "6px",
          }}
        >
          <div style={{ fontWeight: 600, color: "var(--accent-text)", fontSize: "12px" }}>
            Permission Request
          </div>
          {session.pendingPermissions.map((req) => (
            <div key={req.id} style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
              <div style={{ fontSize: "12px" }}>
                Operation <code>{req.tool}</code> requires <b>{req.tier}</b> permission:
              </div>
              <div style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>
                {req.reason} ({req.argument})
              </div>
              <div style={{ display: "flex", gap: "6px", marginTop: "4px" }}>
                <InteriorButton size="sm" variant="primary" onClick={() => onAllowPermission(req.id, "once")}>
                  Allow once
                </InteriorButton>
                <InteriorButton size="sm" variant="secondary" onClick={() => onAllowPermission(req.id, "session")}>
                  Allow session
                </InteriorButton>
                <InteriorButton size="sm" variant="secondary" onClick={() => onDenyPermission(req.id)}>
                  Deny
                </InteriorButton>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Session Metadata & Badges */}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: "6px",
          fontSize: "11px",
        }}
      >
        <span
          className="glass"
          data-elev="1"
          style={{ padding: "2px 8px", borderRadius: "6px" }}
          title="Active Session Branch"
        >
          🌿 {session.activeBranch}
        </span>
        <span
          className="glass"
          data-elev="1"
          style={{ padding: "2px 8px", borderRadius: "6px" }}
          title="Active Tool Profile"
        >
          🎯 {session.activeMode}
        </span>
        {session.grantedTiers.map((tier) => (
          <span
            key={tier}
            className="glass"
            data-elev="1"
            style={{
              padding: "2px 8px",
              borderRadius: "6px",
              color: tier === "filesystem" || tier === "admin" ? "var(--accent-text)" : undefined,
            }}
            title={`Granted Tier: ${tier}`}
          >
            🔒 {tier}
          </span>
        ))}
      </div>

      {/* Project Brief Section */}
      <div
        className="glass"
        data-elev="1"
        style={{
          padding: "8px 12px",
          borderRadius: "var(--radius-sm, 10px)",
          display: "flex",
          flexDirection: "column",
          gap: "6px",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            cursor: "pointer",
          }}
          onClick={() => setBriefOpen(!briefOpen)}
        >
          <span style={{ fontWeight: 600, fontSize: "12px" }}>📋 Project Brief</span>
          <span style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>
            {briefOpen ? "Collapse" : "Edit brief"}
          </span>
        </div>

        {briefOpen ? (
          <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginTop: "4px" }}>
            <div>
              <label style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>Goal</label>
              <input
                type="text"
                className="input glass"
                data-elev="2"
                style={{ width: "100%", padding: "4px 8px", fontSize: "12px", borderRadius: "6px" }}
                value={briefDraft.goal}
                onChange={(e) => setBriefDraft({ ...briefDraft, goal: e.target.value })}
              />
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "6px" }}>
              <div>
                <label style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>Platform</label>
                <input
                  type="text"
                  className="input glass"
                  data-elev="2"
                  style={{ width: "100%", padding: "4px 8px", fontSize: "12px", borderRadius: "6px" }}
                  value={briefDraft.platform}
                  onChange={(e) => setBriefDraft({ ...briefDraft, platform: e.target.value })}
                />
              </div>
              <div>
                <label style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>Target Loudness</label>
                <input
                  type="number"
                  step="0.5"
                  className="input glass"
                  data-elev="2"
                  style={{ width: "100%", padding: "4px 8px", fontSize: "12px", borderRadius: "6px" }}
                  value={briefDraft.loudness_target_lufs}
                  onChange={(e) => setBriefDraft({ ...briefDraft, loudness_target_lufs: parseFloat(e.target.value) || -14 })}
                />
              </div>
            </div>
            <div>
              <label style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>Style Notes</label>
              <textarea
                className="input glass"
                data-elev="2"
                rows={2}
                style={{ width: "100%", padding: "4px 8px", fontSize: "12px", borderRadius: "6px", resize: "none" }}
                value={briefDraft.style_notes}
                onChange={(e) => setBriefDraft({ ...briefDraft, style_notes: e.target.value })}
              />
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end", gap: "6px" }}>
              <InteriorButton size="sm" variant="secondary" onClick={() => setBriefOpen(false)}>
                Cancel
              </InteriorButton>
              <InteriorButton size="sm" variant="primary" onClick={handleSaveBrief}>
                Save Brief
              </InteriorButton>
            </div>
          </div>
        ) : (
          <div style={{ fontSize: "12px", color: "var(--text-dim, #d1d5db)" }}>
            <b>{session.brief.goal}</b> ({session.brief.platform}, {session.brief.loudness_target_lufs} LUFS)
          </div>
        )}
      </div>

      {/* Numbered Plan */}
      <div
        className="glass"
        data-elev="1"
        style={{
          padding: "8px 12px",
          borderRadius: "var(--radius-sm, 10px)",
          display: "flex",
          flexDirection: "column",
          gap: "8px",
        }}
      >
        <div style={{ fontWeight: 600, fontSize: "12px" }}>🎯 Execution Plan</div>
        <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          {session.plan.map((step) => {
            const isStepCurrent = step.status === "current";
            const isStepDone = step.status === "done";
            return (
              <div
                key={step.id}
                onClick={() => step.targetTick != null && onJumpToTick(step.targetTick)}
                title={step.targetTick != null ? `Jump playhead to tick ${step.targetTick}` : undefined}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "4px 8px",
                  borderRadius: "6px",
                  cursor: step.targetTick != null ? "pointer" : "default",
                  background: isStepCurrent ? "var(--card-inset, rgba(255,255,255,0.05))" : "transparent",
                  borderLeft: isStepCurrent ? "2px solid var(--accent-text)" : "2px solid transparent",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span
                    style={{
                      width: "16px",
                      height: "16px",
                      borderRadius: "50%",
                      display: "grid",
                      placeItems: "center",
                      fontSize: "10px",
                      fontWeight: 700,
                      backgroundColor: isStepDone
                        ? "var(--accent-text-violet, #c084fc)"
                        : isStepCurrent
                        ? "var(--accent-text, #f472b6)"
                        : "var(--card-inset, rgba(255,255,255,0.1))",
                      color: "#fff",
                    }}
                  >
                    {isStepDone ? "✓" : step.id}
                  </span>
                  <span
                    style={{
                      color: isStepDone ? "var(--text-muted, #9ca3af)" : "var(--text-primary)",
                      textDecoration: isStepDone ? "line-through" : "none",
                    }}
                  >
                    {step.title}
                  </span>
                </div>
                {step.targetTick != null && (
                  <span style={{ fontSize: "10px", color: "var(--accent-text)", fontFamily: "monospace" }}>
                    {step.targetTick}t
                  </span>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Collapsed Last-Seen Frame / Telemetry Strip */}
      <div
        className="glass"
        data-elev="1"
        style={{
          padding: "8px 12px",
          borderRadius: "var(--radius-sm, 10px)",
          display: "flex",
          flexDirection: "column",
          gap: "6px",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            cursor: "pointer",
          }}
          onClick={() => setPreviewStripOpen(!previewStripOpen)}
        >
          <span style={{ fontWeight: 600, fontSize: "12px" }}>📸 Last-Seen Telemetry</span>
          <span style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)" }}>
            {previewStripOpen ? "Hide" : "Expand"}
          </span>
        </div>

        {previewStripOpen && (
          <div style={{ fontSize: "11px", color: "var(--text-muted, #9ca3af)", display: "flex", flexDirection: "column", gap: "4px" }}>
            <div>Last verified scope: <b>EBU R128 (-14.2 LUFS)</b></div>
            <div>True Peak: <b>-1.1 dBTP</b> (threshold: -1.0 dBTP)</div>
            <div>Face centroids: <b>Detected frame #36</b></div>
          </div>
        )}
      </div>

      {/* Instruction Box */}
      <div
        style={{
          marginTop: "auto",
          display: "flex",
          flexDirection: "column",
          gap: "6px",
        }}
      >
        <textarea
          rows={2}
          className="input glass"
          data-elev="2"
          placeholder="Instruct agent (e.g. 'Align cut at 12s with the beat')..."
          value={instruction}
          onChange={(e) => setInstruction(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              handleSend();
            }
          }}
          style={{
            width: "100%",
            padding: "8px",
            borderRadius: "var(--radius-sm, 10px)",
            fontSize: "12px",
            resize: "none",
          }}
        />
        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <InteriorButton
            size="sm"
            variant="primary"
            onClick={handleSend}
            disabled={!instruction.trim() || isStopped}
          >
            Send to Agent
          </InteriorButton>
        </div>
      </div>
    </div>
  );
};
