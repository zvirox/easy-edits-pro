// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import React, { useEffect, useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import type { AgentQuestion } from "./types";

export interface AgentInboxProps {
  questions: AgentQuestion[];
  onAnswer: (questionId: string, optionId: string) => void;
}

export const AgentInbox: React.FC<AgentInboxProps> = ({ questions, onAnswer }) => {
  const activeQuestions = questions.filter((q) => q.status === "open");
  const [selectedOptions, setSelectedOptions] = useState<Record<string, string>>({});

  useEffect(() => {
    // Set initial defaults
    const defaults: Record<string, string> = {};
    for (const q of activeQuestions) {
      if (!selectedOptions[q.id]) {
        const defaultOpt = q.options.find((o) => o.is_default) || q.options[0];
        if (defaultOpt) {
          defaults[q.id] = defaultOpt.id;
        }
      }
    }
    if (Object.keys(defaults).length > 0) {
      setSelectedOptions((prev) => ({ ...prev, ...defaults }));
    }
  }, [activeQuestions]);

  // Keyboard shortcut listener: numbers 1..9 select options for first active question
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (activeQuestions.length === 0) return;
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable) {
        return;
      }
      const num = parseInt(e.key, 10);
      if (!isNaN(num) && num >= 1 && num <= 9) {
        const currentQ = activeQuestions[0];
        const optIndex = num - 1;
        if (optIndex < currentQ.options.length) {
          e.preventDefault();
          const chosenOpt = currentQ.options[optIndex];
          setSelectedOptions((prev) => ({ ...prev, [currentQ.id]: chosenOpt.id }));
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeQuestions]);

  if (activeQuestions.length === 0) {
    return null;
  }

  return (
    <div
      className="agent-inbox"
      role="region"
      aria-label="Agent Question Inbox"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "10px",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <span style={{ fontSize: "12px", fontWeight: 700, color: "var(--warn-text, #f59e0b)" }}>
          📬 Inbox: Decisions Needed ({activeQuestions.length})
        </span>
        <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>
          Keys 1–{Math.min(9, activeQuestions[0].options.length)} select
        </span>
      </div>

      {activeQuestions.map((q) => {
        const selectedId = selectedOptions[q.id] || q.default_option_id;

        return (
          <div
            key={q.id}
            className="glass"
            data-elev="2"
            style={{
              padding: "10px 12px",
              borderRadius: "var(--radius-sm, 10px)",
              border: "1px solid var(--warn-text, #f59e0b)",
              display: "flex",
              flexDirection: "column",
              gap: "8px",
            }}
          >
            <div style={{ fontWeight: 600, fontSize: "13px" }}>{q.question}</div>
            {q.context && (
              <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>{q.context}</div>
            )}

            {/* Options list */}
            <div style={{ display: "flex", flexDirection: "column", gap: "6px", marginTop: "2px" }}>
              {q.options.map((opt, idx) => {
                const isSelected = selectedId === opt.id;
                const isDefault = opt.is_default || opt.id === q.default_option_id;

                return (
                  <div
                    key={opt.id}
                    onClick={() => setSelectedOptions((prev) => ({ ...prev, [q.id]: opt.id }))}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      padding: "6px 10px",
                      borderRadius: "var(--radius-xs, 6px)",
                      cursor: "pointer",
                      background: isSelected
                        ? "var(--surface-sunken, rgba(245,158,11,0.15))"
                        : "var(--surface-sunken, rgba(0,0,0,0.2))",
                      border: isSelected
                        ? "1px solid var(--warn-text, #f59e0b)"
                        : "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                      <span
                        style={{
                          width: "18px",
                          height: "18px",
                          borderRadius: "50%",
                          display: "inline-flex",
                          alignItems: "center",
                          justifyContent: "center",
                          fontSize: "10px",
                          fontWeight: 700,
                          background: isSelected ? "var(--warn-text, #f59e0b)" : "var(--border-subtle)",
                          color: isSelected ? "var(--text-inverted, #000)" : "var(--text-primary)",
                        }}
                      >
                        {idx + 1}
                      </span>
                      <div>
                        <div style={{ fontSize: "12px", fontWeight: isSelected ? 600 : 400 }}>
                          {opt.label}
                        </div>
                        {opt.description && (
                          <div style={{ fontSize: "10px", color: "var(--text-muted)" }}>
                            {opt.description}
                          </div>
                        )}
                      </div>
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                      {isDefault && (
                        <span
                          style={{
                            fontSize: "9px",
                            padding: "1px 4px",
                            borderRadius: "3px",
                            background: "var(--surface-sunken)",
                            color: "var(--success-text, #10b981)",
                            border: "1px solid var(--success-text, #10b981)",
                          }}
                        >
                          Safe Default
                        </span>
                      )}
                      {opt.thumbnail && (
                        <img
                          src={opt.thumbnail}
                          alt={opt.label}
                          style={{ width: "24px", height: "24px", borderRadius: "3px", objectFit: "cover" }}
                        />
                      )}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Answer button */}
            <div style={{ display: "flex", justifyContent: "flex-end", marginTop: "4px" }}>
              <InteriorButton
                size="sm"
                variant="primary"
                onClick={() => onAnswer(q.id, selectedId)}
                title="Submit decision and resume agent execution"
              >
                Confirm Choice & Resume
              </InteriorButton>
            </div>
          </div>
        );
      })}
    </div>
  );
};
