// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { GlassDialog } from "../../motion/GlassDialog";

export interface AgentSettings {
  provider: "anthropic" | "openai" | "ollama" | "custom_daemon";
  endpoint: string;
  defaultMode: string;
  defaultGrants: string[];
  autoMergePolicy: "manual" | "verified_pass";
  auditLogPath: string;
}

export interface AgentSettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave?: (settings: AgentSettings) => void;
  onRunDemoTask?: () => void;
}

export const AgentSettingsModal: React.FC<AgentSettingsModalProps> = ({
  isOpen,
  onClose,
  onSave,
  onRunDemoTask,
}) => {
  const [provider, setProvider] = useState<AgentSettings["provider"]>("anthropic");
  const [endpoint, setEndpoint] = useState("http://127.0.0.1:4404");
  const [defaultMode, setDefaultMode] = useState("fine-cut");
  const [grants, setGrants] = useState<string[]>(["read", "edit"]);
  const [autoMergePolicy, setAutoMergePolicy] = useState<"manual" | "verified_pass">("manual");
  const [auditLogPath, setAuditLogPath] = useState("~/.config/easy-edits-pro/audit.jsonl");
  const [demoNotice, setDemoNotice] = useState<string | null>(null);

  const toggleGrant = (tier: string) => {
    if (grants.includes(tier)) {
      setGrants(grants.filter((g) => g !== tier));
    } else {
      setGrants([...grants, tier]);
    }
  };

  const handleSave = () => {
    onSave?.({
      provider,
      endpoint,
      defaultMode,
      defaultGrants: grants,
      autoMergePolicy,
      auditLogPath,
    });
    onClose();
  };

  const handleDemo = () => {
    setDemoNotice("Sample podcast cleanup launched on demo sequence.");
    onRunDemoTask?.();
    setTimeout(() => setDemoNotice(null), 3000);
  };

  return (
    <GlassDialog
      open={isOpen}
      cardClassName="modal-dialog-card glass agent-settings-modal"
      onBackdropClick={onClose}
      cardProps={{
        style: { maxWidth: "580px", width: "100%", maxHeight: "85vh", overflowY: "auto" },
        onClick: (e) => e.stopPropagation(),
      }}
    >
      <div
        className="modal-header"
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "16px",
          borderBottom: "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
          paddingBottom: "8px",
        }}
      >
        <h2 style={{ margin: 0, fontSize: "16px", fontWeight: 600 }}>🤖 Agent & Co-Editing Settings</h2>
        <InteriorButton type="button" className="close-btn" onClick={onClose}>✕</InteriorButton>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "16px", fontSize: "13px" }}>
        {/* Provider and Daemon endpoint */}
        <div>
          <label style={{ display: "block", fontWeight: 600, marginBottom: "6px" }}>
            Model Provider / Daemon Connection
          </label>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px", marginBottom: "8px" }}>
            {(["anthropic", "openai", "ollama", "custom_daemon"] as const).map((p) => (
              <label
                key={p}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  padding: "6px 10px",
                  borderRadius: "var(--radius-sm, 6px)",
                  background: provider === p ? "var(--bg-elev-2, rgba(255,255,255,0.08))" : "var(--bg-elev-1, rgba(255,255,255,0.03))",
                  border: provider === p ? "1px solid var(--accent-text, #f472b6)" : "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
                  cursor: "pointer",
                }}
              >
                <input
                  type="radio"
                  name="provider"
                  value={p}
                  checked={provider === p}
                  onChange={() => setProvider(p)}
                />
                <span style={{ textTransform: "capitalize" }}>
                  {p === "custom_daemon" ? "Local Daemon (UDS/TCP)" : p}
                </span>
              </label>
            ))}
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>Endpoint / Socket Path</span>
            <input
              type="text"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="http://127.0.0.1:4404 or /tmp/saturn-daemon.sock"
              style={{
                width: "100%",
                padding: "6px 8px",
                borderRadius: "var(--radius-sm, 6px)",
                background: "var(--bg-elev-1, rgba(255,255,255,0.03))",
                border: "1px solid var(--border-subtle, rgba(255,255,255,0.1))",
                color: "var(--text-primary)",
                fontFamily: "monospace",
                fontSize: "12px",
              }}
            />
          </div>
        </div>

        {/* Default session mode */}
        <div>
          <label style={{ display: "block", fontWeight: 600, marginBottom: "6px" }}>
            Default Session Profile
          </label>
          <select
            value={defaultMode}
            onChange={(e) => setDefaultMode(e.target.value)}
            style={{
              width: "100%",
              padding: "6px 8px",
              borderRadius: "var(--radius-sm, 6px)",
              background: "var(--bg-elev-1, rgba(255,255,255,0.03))",
              border: "1px solid var(--border-subtle, rgba(255,255,255,0.1))",
              color: "var(--text-primary)",
              fontSize: "13px",
            }}
          >
            <option value="fine-cut">Fine-Cut (Dialogue cleanup, match cuts, transient alignment)</option>
            <option value="rough-cut">Rough-Cut (Transcript indexing, silence removal, multi-take assembly)</option>
            <option value="review-only">Review-Only (Non-destructive inspection and measured verification)</option>
          </select>
        </div>

        {/* Default Grants */}
        <div>
          <label style={{ display: "block", fontWeight: 600, marginBottom: "6px" }}>
            Default Security Grants (AF2 / AF5 Origin Tracking)
          </label>
          <div style={{ display: "flex", flexWrap: "wrap", gap: "8px" }}>
            {[
              { id: "read", label: "Read (Sequence, scopes, audio telemetry)" },
              { id: "edit", label: "Edit (Non-destructive clip cuts & branch creation)" },
              { id: "filesystem", label: "Filesystem (Export & render artifacts)" },
              { id: "admin", label: "Admin (Direct project settings mutation)" },
            ].map(({ id, label }) => (
              <label
                key={id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "6px",
                  padding: "4px 8px",
                  borderRadius: "var(--radius-sm, 6px)",
                  background: grants.includes(id) ? "var(--bg-elev-2, rgba(255,255,255,0.06))" : "var(--bg-elev-1, rgba(255,255,255,0.02))",
                  border: "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
                  cursor: "pointer",
                  fontSize: "12px",
                }}
              >
                <input
                  type="checkbox"
                  checked={grants.includes(id)}
                  onChange={() => toggleGrant(id)}
                />
                <span>{label}</span>
              </label>
            ))}
          </div>
          <p style={{ margin: "4px 0 0 0", fontSize: "11px", color: "var(--text-muted)" }}>
            Note: Untrusted media text (captions, OCR, transcripts) cannot escalate permissions or execute destructive operations without explicit approval.
          </p>
        </div>

        {/* Auto-merge policy */}
        <div>
          <label style={{ display: "block", fontWeight: 600, marginBottom: "6px" }}>
            Auto-Merge Policy
          </label>
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <label style={{ display: "flex", alignItems: "center", gap: "8px", cursor: "pointer" }}>
              <input
                type="radio"
                name="autoMergePolicy"
                value="manual"
                checked={autoMergePolicy === "manual"}
                onChange={() => setAutoMergePolicy("manual")}
              />
              <span>Manual review required (Recommended — human review via Review Card)</span>
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: "8px", cursor: "pointer" }}>
              <input
                type="radio"
                name="autoMergePolicy"
                value="verified_pass"
                checked={autoMergePolicy === "verified_pass"}
                onChange={() => setAutoMergePolicy("verified_pass")}
              />
              <span>Auto-merge only when all <code>verify.run</code> checks pass and confidence is high</span>
            </label>
          </div>
        </div>

        {/* Audit Log Path */}
        <div>
          <label style={{ display: "block", fontWeight: 600, marginBottom: "4px" }}>
            Audit Log Location (JSONL)
          </label>
          <input
            type="text"
            value={auditLogPath}
            onChange={(e) => setAuditLogPath(e.target.value)}
            style={{
              width: "100%",
              padding: "6px 8px",
              borderRadius: "var(--radius-sm, 6px)",
              background: "var(--bg-elev-1, rgba(255,255,255,0.03))",
              border: "1px solid var(--border-subtle, rgba(255,255,255,0.1))",
              color: "var(--text-primary)",
              fontFamily: "monospace",
              fontSize: "12px",
            }}
          />
        </div>

        {/* First-Run Onboarding Demo */}
        <div
          style={{
            padding: "10px",
            borderRadius: "var(--radius-sm, 6px)",
            background: "var(--bg-elev-1, rgba(255,255,255,0.03))",
            border: "1px dashed var(--border-subtle, rgba(255,255,255,0.12))",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: "4px" }}>First-Run Onboarding</div>
          <p style={{ margin: "0 0 8px 0", fontSize: "12px", color: "var(--text-muted)" }}>
            Run a closed-loop sample task (podcast dialogue cleanup & match cut alignment) on a synthetic CC0 demo project to test daemon connectivity and verification.
          </p>
          <InteriorButton
            variant="secondary"
            size="sm"
            onClick={handleDemo}
          >
            🚀 Run Sample Task on Demo Project
          </InteriorButton>
          {demoNotice && (
            <span style={{ marginLeft: "10px", fontSize: "11px", color: "var(--accent-text, #f472b6)" }}>
              {demoNotice}
            </span>
          )}
        </div>
      </div>

      <div
        style={{
          display: "flex",
          justifyContent: "flex-end",
          gap: "8px",
          marginTop: "20px",
          borderTop: "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
          paddingTop: "12px",
        }}
      >
        <InteriorButton variant="secondary" onClick={onClose}>
          Cancel
        </InteriorButton>
        <InteriorButton variant="primary" onClick={handleSave}>
          Save Settings
        </InteriorButton>
      </div>
    </GlassDialog>
  );
};
