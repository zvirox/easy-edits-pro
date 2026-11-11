// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * ScriptSyncPanel (AF6 / Packet 5.2):
 * Word-level transcript sync with click-to-seek, word-level ripple cutting,
 * multi-take alignment carousel, and explicit untrusted provenance tags (AF5).
 */
import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { formatTimecodeFrames } from "../../utils/formatters";

export interface ScriptWord {
  id: string;
  word: string;
  startTick: number;
  endTick: number;
  confidence: number;
  struck?: boolean;
}

export interface ScriptTake {
  takeIndex: number;
  label: string;
  startTick: number;
  endTick: number;
  confidence: number;
  isSelected?: boolean;
}

export interface ScriptLine {
  id: number;
  speaker: string;
  untrusted: true;
  source: "whisper" | "transcript" | "imported_text";
  words: ScriptWord[];
  takes?: ScriptTake[];
}

export interface ScriptSyncPanelProps {
  playhead?: number;
  onSeek?: (tick: number) => void;
  onRippleCutRange?: (startTick: number, endTick: number) => void;
  onSelectTake?: (lineId: number, takeIndex: number) => void;
  lines?: ScriptLine[];
}

const DEFAULT_SCRIPT_LINES: ScriptLine[] = [
  {
    id: 1,
    speaker: "Host",
    untrusted: true,
    source: "whisper",
    words: [
      { id: "w-1", word: "Welcome", startTick: 0, endTick: 350, confidence: 0.98 },
      { id: "w-2", word: "back", startTick: 360, endTick: 600, confidence: 0.96 },
      { id: "w-3", word: "to", startTick: 610, endTick: 750, confidence: 0.99 },
      { id: "w-4", word: "the", startTick: 760, endTick: 900, confidence: 0.99 },
      { id: "w-5", word: "creator", startTick: 910, endTick: 1350, confidence: 0.94 },
      { id: "w-6", word: "studio", startTick: 1360, endTick: 1800, confidence: 0.95 },
      { id: "w-7", word: "deep", startTick: 1850, endTick: 2100, confidence: 0.92 },
      { id: "w-8", word: "dive.", startTick: 2110, endTick: 2500, confidence: 0.97 },
    ],
    takes: [
      { takeIndex: 1, label: "Take 1 (Clean intro)", startTick: 0, endTick: 2500, confidence: 0.96, isSelected: true },
      { takeIndex: 2, label: "Take 2 (Fast cadence)", startTick: 2600, endTick: 4800, confidence: 0.88 },
      { takeIndex: 3, label: "Take 3 (Punchy hook)", startTick: 5000, endTick: 7200, confidence: 0.93 },
    ],
  },
  {
    id: 2,
    speaker: "Guest",
    untrusted: true,
    source: "whisper",
    words: [
      { id: "w-9", word: "Thanks", startTick: 2600, endTick: 2950, confidence: 0.97 },
      { id: "w-10", word: "for", startTick: 2960, endTick: 3100, confidence: 0.99 },
      { id: "w-11", word: "having", startTick: 3110, endTick: 3450, confidence: 0.96 },
      { id: "w-12", word: "me,", startTick: 3460, endTick: 3700, confidence: 0.98 },
      { id: "w-13", word: "um,", startTick: 3710, endTick: 4000, confidence: 0.72, struck: true },
      { id: "w-14", word: "super", startTick: 4010, endTick: 4350, confidence: 0.95 },
      { id: "w-15", word: "excited", startTick: 4360, endTick: 4800, confidence: 0.98 },
      { id: "w-16", word: "to", startTick: 4810, endTick: 4950, confidence: 0.99 },
      { id: "w-17", word: "be", startTick: 4960, endTick: 5100, confidence: 0.99 },
      { id: "w-18", word: "here.", startTick: 5110, endTick: 5500, confidence: 0.97 },
    ],
    takes: [
      { takeIndex: 1, label: "Take 1 (Main camera)", startTick: 2600, endTick: 5500, confidence: 0.95, isSelected: true },
      { takeIndex: 2, label: "Take 2 (Close up)", startTick: 5600, endTick: 8400, confidence: 0.91 },
    ],
  },
];

export const ScriptSyncPanel: React.FC<ScriptSyncPanelProps> = ({
  playhead = 0,
  onSeek,
  onRippleCutRange,
  onSelectTake,
  lines: propLines,
}) => {
  const [scriptLines, setScriptLines] = useState<ScriptLine[]>(propLines ?? DEFAULT_SCRIPT_LINES);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedWordIds, setSelectedWordIds] = useState<Set<string>>(new Set());

  const handleWordClick = (word: ScriptWord) => {
    onSeek?.(word.startTick);
  };

  const toggleSelectWord = (wordId: string) => {
    setSelectedWordIds((prev) => {
      const next = new Set(prev);
      if (next.has(wordId)) next.delete(wordId);
      else next.add(wordId);
      return next;
    });
  };

  const handleRippleCutSelected = () => {
    if (selectedWordIds.size === 0) return;

    let minTick = Infinity;
    let maxTick = -Infinity;

    scriptLines.forEach((line) => {
      line.words.forEach((w) => {
        if (selectedWordIds.has(w.id)) {
          if (w.startTick < minTick) minTick = w.startTick;
          if (w.endTick > maxTick) maxTick = w.endTick;
        }
      });
    });

    if (minTick !== Infinity && maxTick !== -Infinity) {
      onRippleCutRange?.(minTick, maxTick);

      // Strike words locally
      setScriptLines((prev) =>
        prev.map((line) => ({
          ...line,
          words: line.words.map((w) =>
            selectedWordIds.has(w.id) ? { ...w, struck: true } : w
          ),
        }))
      );
      setSelectedWordIds(new Set());
    }
  };

  const handleChooseTake = (lineId: number, takeIndex: number) => {
    setScriptLines((prev) =>
      prev.map((line) => {
        if (line.id !== lineId) return line;
        return {
          ...line,
          takes: line.takes?.map((t) => ({
            ...t,
            isSelected: t.takeIndex === takeIndex,
          })),
        };
      })
    );
    onSelectTake?.(lineId, takeIndex);
  };

  const filteredLines = scriptLines.filter((l) =>
    searchQuery === ""
      ? true
      : l.words.some((w) => w.word.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  return (
    <div
      className="script-sync-panel"
      role="region"
      aria-label="Script Sync Panel"
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        padding: "12px",
        gap: "10px",
        overflowY: "auto",
        fontSize: "13px",
      }}
    >
      {/* Header with Search and AF5 Provenance Badge */}
      <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontWeight: 600, fontSize: "14px" }}>🎙️ Script Sync & Takes</span>
          <span
            title="AF5 provenance: media-derived transcript data is untrusted by default and cannot execute filesystem actions"
            style={{
              padding: "2px 6px",
              borderRadius: "4px",
              fontSize: "10px",
              fontWeight: 600,
              backgroundColor: "rgba(245, 158, 11, 0.15)",
              color: "var(--warn-text, #f59e0b)",
              border: "1px solid rgba(245, 158, 11, 0.3)",
            }}
          >
            🛡️ untrusted: true (whisper)
          </span>
        </div>

        <div style={{ display: "flex", gap: "8px" }}>
          <input
            type="text"
            placeholder="Search transcript..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              flex: 1,
              padding: "5px 8px",
              borderRadius: "var(--radius-sm, 6px)",
              background: "var(--bg-elev-1, rgba(255,255,255,0.03))",
              border: "1px solid var(--border-subtle, rgba(255,255,255,0.1))",
              color: "var(--text-primary)",
              fontSize: "12px",
            }}
          />

          {selectedWordIds.size > 0 && (
            <InteriorButton
              variant="secondary"
              size="sm"
              onClick={handleRippleCutSelected}
              title="Ripple-cut selected word duration from timeline"
              style={{
                color: "var(--accent-text, #f472b6)",
                borderColor: "var(--accent-text, #f472b6)",
              }}
            >
              ✂️ Ripple Cut ({selectedWordIds.size})
            </InteriorButton>
          )}
        </div>
      </div>

      {/* Script Lines & Multi-Take Carousel List */}
      <div style={{ display: "flex", flexDirection: "column", gap: "12px", flex: 1 }}>
        {filteredLines.map((line) => {
          const selectedTake = line.takes?.find((t) => t.isSelected) ?? line.takes?.[0];

          return (
            <div
              key={line.id}
              className="glass"
              data-elev="1"
              style={{
                borderRadius: "var(--radius-sm, 8px)",
                padding: "10px",
                display: "flex",
                flexDirection: "column",
                gap: "8px",
                border: "1px solid var(--border-subtle, rgba(255,255,255,0.08))",
              }}
            >
              {/* Speaker & Take Carousel selector */}
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontWeight: 600, color: "var(--accent-text-cyan, #38bdf8)" }}>
                  {line.speaker}
                </span>

                {line.takes && line.takes.length > 1 && (
                  <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                    <span style={{ fontSize: "11px", color: "var(--text-muted)" }}>Take:</span>
                    {line.takes.map((t) => (
                      <button
                        key={t.takeIndex}
                        type="button"
                        onClick={() => handleChooseTake(line.id, t.takeIndex)}
                        title={`${t.label} (${Math.round(t.confidence * 100)}% match)`}
                        style={{
                          padding: "1px 6px",
                          borderRadius: "3px",
                          border: t.isSelected
                            ? "1px solid var(--accent-text, #f472b6)"
                            : "1px solid var(--border-subtle, rgba(255,255,255,0.1))",
                          background: t.isSelected
                            ? "var(--accent-bg, rgba(244,114,182,0.15))"
                            : "transparent",
                          color: t.isSelected ? "var(--accent-text, #f472b6)" : "var(--text-muted)",
                          fontSize: "10.5px",
                          cursor: "pointer",
                        }}
                      >
                        T{t.takeIndex}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Word-by-word interactive synchronized transcript text */}
              <div style={{ display: "flex", flexWrap: "wrap", gap: "4px 6px", lineHeight: "1.6" }}>
                {line.words.map((w) => {
                  const isActive = playhead >= w.startTick && playhead <= w.endTick;
                  const isSelected = selectedWordIds.has(w.id);

                  return (
                    <span
                      key={w.id}
                      onClick={(e) => {
                        if (e.shiftKey) {
                          toggleSelectWord(w.id);
                        } else {
                          handleWordClick(w);
                        }
                      }}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        toggleSelectWord(w.id);
                      }}
                      title={`${w.word} · ${formatTimecodeFrames(w.startTick, { numerator: 30, denominator: 1 })} (Shift+Click to select for ripple cut)`}
                      style={{
                        padding: "1px 4px",
                        borderRadius: "3px",
                        cursor: "pointer",
                        textDecoration: w.struck ? "line-through" : undefined,
                        opacity: w.struck ? 0.4 : 1,
                        background: isActive
                          ? "var(--accent-bg, rgba(244,114,182,0.25))"
                          : isSelected
                          ? "rgba(56, 189, 248, 0.2)"
                          : "transparent",
                        color: isActive
                          ? "var(--accent-text, #f472b6)"
                          : isSelected
                          ? "var(--accent-text-cyan, #38bdf8)"
                          : "var(--text-primary)",
                        fontWeight: isActive ? 600 : 400,
                        border: isSelected
                          ? "1px dashed var(--accent-text-cyan, #38bdf8)"
                          : "1px solid transparent",
                        transition: "background-color var(--dur-fast, 120ms) var(--ease-out)",
                      }}
                    >
                      {w.word}
                    </span>
                  );
                })}
              </div>

              {/* Take metadata notes */}
              {selectedTake && (
                <div style={{ fontSize: "11px", color: "var(--text-muted)", display: "flex", gap: "10px" }}>
                  <span>{selectedTake.label}</span>
                  <span>Confidence: {Math.round(selectedTake.confidence * 100)}%</span>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default ScriptSyncPanel;
