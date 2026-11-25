// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
/**
 * Web Desk New Job Modal (TS8).
 * Enables creating a task sheet job from the 3 flagship recipes with media and deliverable selection.
 */

import React, { useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { FLAGSHIP_TEMPLATES } from "../tasksheet/types";
import type { useWebDeskStore } from "./webDeskStore";
import type { SheetDeliverableState, SheetInputState } from "./types";

interface NewJobModalProps {
  store: ReturnType<typeof useWebDeskStore>;
  onClose: () => void;
}

export const NewJobModal: React.FC<NewJobModalProps> = ({ store, onClose }) => {
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("tutorial_screen");

  // Initial state derived from default tutorial template
  const defaultTemplate = FLAGSHIP_TEMPLATES.find((t) => t.id === "tutorial_screen")?.template;

  const [title, setTitle] = useState(defaultTemplate?.title ?? "Tutorial & Screen Capture Clean Cut");
  const [requester, setRequester] = useState(defaultTemplate?.requester ?? "human:video_producer");
  const [inputs, setInputs] = useState<SheetInputState[]>(
    defaultTemplate?.inputs ?? [
      { id: "in_raw_screen", media: "media/screen_recording_01.mp4", role: "main_video", trust: "trusted" },
      { id: "in_screen_events", media: "media/ssp_capture_events.cet", role: "capture_event_track", trust: "trusted" },
      { id: "in_transcript", media: "media/whisper_transcript.json", role: "transcript", trust: "untrusted" },
    ]
  );
  const [deliverables, setDeliverables] = useState<SheetDeliverableState[]>(
    defaultTemplate?.deliverables ?? [
      {
        id: "deliv_master_clean",
        name: "Master Tutorial Cut (1080p60)",
        preset: "youtube-1080p",
        duration: { target_s: 300, tolerance_s: 30 },
        loudness_lufs: -14,
        true_peak_dbtp: -1,
        captions: true,
      },
    ]
  );
  const [budget, setBudget] = useState({
    wall_clock_min: defaultTemplate?.budget?.wall_clock_min ?? 45,
    model_cost_usd: defaultTemplate?.budget?.model_cost_usd ?? 3.0,
    render_min: defaultTemplate?.budget?.render_min ?? 20,
  });

  const [validationNotice, setValidationNotice] = useState<string | null>(null);
  const [validationError, setValidationError] = useState<string | null>(null);

  const handleSelectTemplate = (templateId: string) => {
    setSelectedTemplateId(templateId);
    const tmpl = FLAGSHIP_TEMPLATES.find((t) => t.id === templateId)?.template;
    if (tmpl) {
      setTitle(tmpl.title);
      setRequester(tmpl.requester);
      setInputs([...(tmpl.inputs ?? [])]);
      setDeliverables([...tmpl.deliverables]);
      setBudget({
        wall_clock_min: tmpl.budget?.wall_clock_min ?? 60,
        model_cost_usd: tmpl.budget?.model_cost_usd ?? 5.0,
        render_min: tmpl.budget?.render_min ?? 30,
      });
      setValidationNotice(null);
      setValidationError(null);
    }
  };

  const handleValidate = () => {
    setValidationError(null);
    setValidationNotice(null);

    if (!title.trim()) {
      setValidationError("Job title is required.");
      return;
    }
    if (inputs.length === 0) {
      setValidationError("At least one input media asset is required.");
      return;
    }
    if (deliverables.length === 0) {
      setValidationError("At least one deliverable target is required.");
      return;
    }

    setValidationNotice(`Valid sheet: ${deliverables.length} deliverables configured.`);
  };

  const handleSubmit = async () => {
    if (!title.trim()) {
      setValidationError("Job title is required.");
      return;
    }
    if (deliverables.length === 0) {
      setValidationError("At least one deliverable target is required.");
      return;
    }

    await store.createJob({
      title,
      templateId: selectedTemplateId,
      requester,
      inputs,
      deliverables,
      budget,
    });
    store.setActiveView("detail");
  };

  return (
    <div
      className="webdesk-modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="new-job-title" data-testid="new-job-modal"
    >
      <div className="webdesk-modal surface-1">
        {/* Modal Header */}
        <div className="webdesk-modal-header">
          <h2 id="new-job-title" className="webdesk-modal-title">
            Create New Task Sheet Job
          </h2>
          <InteriorButton size="sm" variant="ghost" onClick={onClose} aria-label="Close modal">
            ✕
          </InteriorButton>
        </div>

        {/* Flagship Recipe Picker */}
        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <label style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-muted)" }}>
            Select Flagship Recipe Template:
          </label>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "8px" }}>
            <button
              type="button" className={`webdesk-nav-btn ${selectedTemplateId === "podcast_social" ? "active" : ""}`}
              style={{ justifyContent: "center", height: "36px", textAlign: "center" }}
              onClick={() => handleSelectTemplate("podcast_social")}
              data-testid="template-btn-podcast"
            >
              Podcast & Social Clips
            </button>
            <button
              type="button" className={`webdesk-nav-btn ${selectedTemplateId === "tutorial_screen" ? "active" : ""}`}
              style={{ justifyContent: "center", height: "36px", textAlign: "center" }}
              onClick={() => handleSelectTemplate("tutorial_screen")}
              data-testid="template-btn-tutorial"
            >
              Tutorial & Screen Capture
            </button>
            <button
              type="button" className={`webdesk-nav-btn ${selectedTemplateId === "webinar_recap" ? "active" : ""}`}
              style={{ justifyContent: "center", height: "36px", textAlign: "center" }}
              onClick={() => handleSelectTemplate("webinar_recap")}
              data-testid="template-btn-webinar"
            >
              Webinar Recap & Training
            </button>
          </div>
        </div>

        {/* General Metadata */}
        <div style={{ display: "grid", gridTemplateColumns: "1.5fr 1fr", gap: "10px" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
              Job Title
            </label>
            <input
              type="text" className="webdesk-search-input" value={title}
              onChange={(e) => setTitle(e.target.value)}
              aria-label="Job Title" data-testid="job-title-input"
            />
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
              Requester / Owner
            </label>
            <input
              type="text" className="webdesk-search-input" value={requester}
              onChange={(e) => setRequester(e.target.value)}
              aria-label="Requester" data-testid="job-requester-input"
            />
          </div>
        </div>

        {/* Media Asset Inputs */}
        <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <label style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-muted)" }}>
              Media Inputs ({inputs.length})
            </label>
            <InteriorButton
              size="sm" variant="ghost" onClick={() =>
                setInputs([
                  ...inputs,
                  {
                    id: `in_${Date.now()}`,
                    media: "media/extra_broll.mp4",
                    role: "b_roll",
                    trust: "trusted",
                  },
                ])
              }
            >
              + Add Input
            </InteriorButton>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "6px", maxHeight: "140px", overflowY: "auto" }}>
            {inputs.map((inp, idx) => (
              <div
                key={inp.id}
                className="surface-1" style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "6px 10px",
                  borderRadius: "var(--radius-sm, 6px)",
                  fontSize: "12px",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span style={{ fontWeight: 600 }}>{inp.media}</span>
                  <span style={{ color: "var(--text-muted)", fontSize: "11px" }}>({inp.role})</span>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <span
                    style={{
                      fontSize: "10px",
                      fontWeight: 600,
                      padding: "1px 6px",
                      borderRadius: "4px",
                      border: "1px solid var(--glass-border)",
                      color: inp.trust === "untrusted" ? "var(--accent-text)" : "var(--accent-text-hover)",
                    }}
                  >
                    {inp.trust === "untrusted" ? "UNTRUSTED" : "TRUSTED"}
                  </span>
                  <InteriorButton
                    size="sm" variant="ghost" onClick={() => setInputs(inputs.filter((_, i) => i !== idx))}
                  >
                    ✕
                  </InteriorButton>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Deliverables */}
        <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <label style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-muted)" }}>
              Deliverable Targets ({deliverables.length})
            </label>
            <InteriorButton
              size="sm" variant="ghost" onClick={() =>
                setDeliverables([
                  ...deliverables,
                  {
                    id: `deliv_${Date.now()}`,
                    name: "Social Clip 9:16",
                    preset: "vertical-9x16",
                    duration: { target_s: 60, tolerance_s: 10 },
                    loudness_lufs: -14,
                    captions: true,
                  },
                ])
              }
            >
              + Add Target
            </InteriorButton>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "6px", maxHeight: "140px", overflowY: "auto" }}>
            {deliverables.map((deliv, idx) => (
              <div
                key={deliv.id}
                className="surface-1" style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "6px 10px",
                  borderRadius: "var(--radius-sm, 6px)",
                  fontSize: "12px",
                }}
              >
                <div>
                  <span style={{ fontWeight: 600 }}>{deliv.name}</span>
                  <span style={{ color: "var(--text-muted)", marginLeft: "6px" }}>
                    ({deliv.preset}, {deliv.duration?.target_s ?? 60}s, {deliv.loudness_lufs ?? -14} LUFS)
                  </span>
                </div>
                <InteriorButton
                  size="sm" variant="ghost" onClick={() => setDeliverables(deliverables.filter((_, i) => i !== idx))}
                >
                  ✕
                </InteriorButton>
              </div>
            ))}
          </div>
        </div>

        {/* Budget Allocation */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "10px" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
              Wall-Clock (min)
            </label>
            <input
              type="number" className="webdesk-search-input" value={budget.wall_clock_min}
              onChange={(e) => setBudget({ ...budget, wall_clock_min: Number(e.target.value) })}
            />
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
              Model Budget ($)
            </label>
            <input
              type="number" step="0.5" className="webdesk-search-input" value={budget.model_cost_usd}
              onChange={(e) => setBudget({ ...budget, model_cost_usd: Number(e.target.value) })}
            />
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <label style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-muted)" }}>
              Render (min)
            </label>
            <input
              type="number" className="webdesk-search-input" value={budget.render_min}
              onChange={(e) => setBudget({ ...budget, render_min: Number(e.target.value) })}
            />
          </div>
        </div>

        {/* Validation notices */}
        {validationNotice && (
          <div
            style={{
              padding: "8px 12px",
              borderRadius: "var(--radius-sm, 6px)",
              background: "var(--card-inset)",
              color: "var(--accent-text-hover)",
              border: "1px solid var(--glass-border)",
              fontSize: "12px",
              fontWeight: 600,
            }}
            data-testid="validation-notice"
          >
            ✓ {validationNotice}
          </div>
        )}
        {validationError && (
          <div
            style={{
              padding: "8px 12px",
              borderRadius: "var(--radius-sm, 6px)",
              background: "var(--card-inset)",
              color: "var(--accent-text)",
              border: "1px solid var(--glass-border)",
              fontSize: "12px",
              fontWeight: 600,
            }}
            data-testid="validation-error"
          >
            {validationError}
          </div>
        )}

        {/* Modal Footer Actions */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: "8px" }}>
          <InteriorButton
            size="md" variant="secondary" onClick={handleValidate}
            data-testid="validate-job-btn"
          >
            Validate
          </InteriorButton>

          <div style={{ display: "flex", gap: "8px" }}>
            <InteriorButton size="md" variant="ghost" onClick={onClose}>
              Cancel
            </InteriorButton>
            <InteriorButton
              size="md" variant="primary" onClick={handleSubmit}
              data-testid="submit-job-btn"
            >
              Submit Job
            </InteriorButton>
          </div>
        </div>
      </div>
    </div>
  );
};
