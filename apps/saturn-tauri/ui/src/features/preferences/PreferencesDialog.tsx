import { useEffect, useState } from "react";
import { GlassDialog } from "../../motion/GlassDialog";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import type { GlassMode, Preferences, TrackHeightPreference } from "./preferences";

type TabId = "general" | "appearance" | "timeline" | "media" | "audio" | "export" | "schema";

interface TabSpec {
  id: TabId;
  label: string;
  reason?: string;
}

const TABS: TabSpec[] = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "timeline", label: "Timeline" },
  { id: "media", label: "Media", reason: "Ships with the media engine in Phase 4." },
  { id: "audio", label: "Audio", reason: "Ships with the Fairlight audio phase." },
  { id: "export", label: "Export", reason: "Ships with the deliver phase." },
  { id: "schema", label: "Schema" },
];

interface PrefSchemaRow {
  key: string;
  type: string;
  values?: unknown[];
  default?: unknown;
}

export interface PreferencesDialogProps {
  open: boolean;
  onClose: () => void;
  preferences: Preferences;
  onChange: (patch: Partial<Preferences>) => void;
  onReset: () => void;
  snapEnabled: boolean;
  onSnapChange: (enabled: boolean) => void;
  trackHeight: TrackHeightPreference;
  onTrackHeightChange: (height: TrackHeightPreference) => void;
  /** Reads the engine's preference keys for the Schema tab (prefs.schema). */
  runCommand?: <T = unknown>(id: string, params?: unknown) => Promise<T>;
}

function Segmented<T extends string | number | boolean | null>({
  label,
  hint,
  value,
  options,
  onChange,
}: {
  label: string;
  hint?: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="preferences-field">
      <div className="preferences-field-head">
        <span className="preferences-field-label">{label}</span>
        {hint ? <span className="preferences-field-hint">{hint}</span> : null}
      </div>
      <div className="preferences-segment" role="group" aria-label={label}>
        {options.map((option) => (
          <button
            key={String(option.value)}
            type="button"
            className={`preferences-segment-option ${value === option.value ? "active" : ""}`}
            aria-pressed={value === option.value}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export function PreferencesDialog({
  open,
  onClose,
  preferences,
  onChange,
  onReset,
  snapEnabled,
  onSnapChange,
  trackHeight,
  onTrackHeightChange,
  runCommand,
}: PreferencesDialogProps) {
  const [tab, setTab] = useState<TabId>("appearance");
  const [schema, setSchema] = useState<PrefSchemaRow[]>([]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  useEffect(() => {
    if (!open || !runCommand) return;
    let active = true;
    runCommand<{ schema: PrefSchemaRow[] }>("prefs.schema")
      .then((result) => {
        if (active) setSchema(result.schema ?? []);
      })
      .catch(() => {
        if (active) setSchema([]);
      });
    return () => {
      active = false;
    };
  }, [open, runCommand]);

  return (
    <GlassDialog
      open={open}
      cardClassName="modal-dialog-card glass preferences-dialog"
      onBackdropClick={onClose}
      cardProps={{
        role: "dialog",
        "aria-modal": true,
        "aria-labelledby": "preferences-dialog-title",
        onClick: (event) => event.stopPropagation(),
      }}
    >
      <header className="preferences-header">
        <div>
          <h2 id="preferences-dialog-title">Preferences</h2>
          <p>Settings are stored on this machine and apply immediately.</p>
        </div>
        <InteriorButton type="button" className="close-btn" onClick={onClose} aria-label="Close">
          <Icon name="close" />
        </InteriorButton>
      </header>

      <div className="preferences-body">
        <nav className="preferences-tabs" role="tablist" aria-label="Preference sections" aria-orientation="vertical">
          {TABS.map((entry) => {
            const disabled = Boolean(entry.reason);
            return (
              <button
                key={entry.id}
                type="button"
                role="tab"
                aria-selected={tab === entry.id}
                aria-controls={`preferences-panel-${entry.id}`}
                className={`preferences-tab ${tab === entry.id ? "active" : ""}`}
                disabled={disabled}
                title={entry.reason}
                onClick={() => setTab(entry.id)}
              >
                <span>{entry.label}</span>
                {disabled ? <span className="preferences-tab-soon">Soon</span> : null}
              </button>
            );
          })}
        </nav>

        <section
          className="preferences-panel"
          role="tabpanel"
          id={`preferences-panel-${tab}`}
          aria-label={TABS.find((entry) => entry.id === tab)?.label}
        >
          {tab === "general" ? (
            <>
              <dl className="preferences-facts">
                <div>
                  <dt>Project format</dt>
                  <dd>.eep · versioned with forward migrations</dd>
                </div>
                <div>
                  <dt>Autosave recovery</dt>
                  <dd>Every 30 seconds while a project is open</dd>
                </div>
                <div>
                  <dt>Shortcut overrides</dt>
                  <dd>Help ▸ Keyboard Shortcuts · stored on this machine</dd>
                </div>
              </dl>
              <p className="preferences-note">
                Project, media, and export settings ship with their engines. Only the
                settings this build can honour appear here.
              </p>
            </>
          ) : null}

          {tab === "appearance" ? (
            <>
              <Segmented
                label="Theme"
                value={preferences.theme}
                options={[
                  { value: "dark", label: "Dark" },
                  { value: "light", label: "Light" },
                ]}
                onChange={(theme) => onChange({ theme })}
              />
              <Segmented<GlassMode>
                label="Glass surfaces"
                hint="Preview, timeline, scopes, and waveforms never take glass."
                value={preferences.glass}
                options={[
                  { value: "full", label: "Full" },
                  { value: "lite", label: "Lite" },
                  { value: "off", label: "Off" },
                ]}
                onChange={(glass) => onChange({ glass })}
              />
              <Segmented<boolean | null>
                label="Motion"
                hint="Follow system honours your desktop's reduced-motion setting."
                value={preferences.reducedMotion}
                options={[
                  { value: null, label: "Follow system" },
                  { value: true, label: "Reduce" },
                  { value: false, label: "Allow" },
                ]}
                onChange={(reducedMotion) => onChange({ reducedMotion })}
              />
            </>
          ) : null}

          {tab === "timeline" ? (
            <>
              <div className="preferences-field">
                <div className="preferences-field-head">
                  <span className="preferences-field-label">Snapping</span>
                  <span className="preferences-field-hint">
                    Snaps edits to clip edges, markers, and the playhead.
                  </span>
                </div>
                <button
                  type="button"
                  className={`preferences-toggle ${snapEnabled ? "active" : ""}`}
                  aria-pressed={snapEnabled}
                  onClick={() => onSnapChange(!snapEnabled)}
                >
                  <span className="preferences-toggle-track" aria-hidden="true">
                    <span className="preferences-toggle-thumb" />
                  </span>
                  {snapEnabled ? "Snapping on" : "Snapping off"}
                </button>
              </div>
              <Segmented<TrackHeightPreference>
                label="Track height"
                value={trackHeight}
                options={[
                  { value: "compact", label: "Compact" },
                  { value: "normal", label: "Normal" },
                  { value: "expanded", label: "Expanded" },
                ]}
                onChange={(height) => onTrackHeightChange(height)}
              />
            </>
          ) : null}
        </section>
      </div>

      {tab === "schema" ? (
        <div className="preferences-schema" data-testid="prefs-schema">
          <p className="preferences-schema-note">
            Keys the engine recognises in <code>prefs.set</code>; absent keys
            fall back to the default.
          </p>
          <table className="preferences-schema-table">
            <thead>
              <tr>
                <th scope="col">Key</th>
                <th scope="col">Values</th>
                <th scope="col">Default</th>
              </tr>
            </thead>
            <tbody>
              {schema.map((row) => (
                <tr key={row.key}>
                  <td className="preferences-schema-key">{row.key}</td>
                  <td>
                    {(row.values ?? []).map((value) => JSON.stringify(value)).join(" · ")}
                  </td>
                  <td>{row.default === null ? "system" : JSON.stringify(row.default)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      <footer className="preferences-footer">
        <span>Preferences · this machine</span>
        <InteriorButton type="button" className="preferences-reset" onClick={onReset}>
          Reset all preferences
        </InteriorButton>
      </footer>
    </GlassDialog>
  );
}
