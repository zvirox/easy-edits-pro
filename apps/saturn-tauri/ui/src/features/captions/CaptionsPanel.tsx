import { useState } from "react";
import type { CaptionTrack, SubtitleItem } from "../../app/projectTypes";
import { formatTimecodeFrames } from "../../utils/formatters";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import { activeCue, frameTicks, sortedCues } from "./captionsList";

export interface CaptionsPanelProps {
  /** All caption tracks; the panel edits `track` (the selection). */
  captionTracks: readonly CaptionTrack[];
  activeTrackId: number | null;
  onTrackChange: (id: number) => void;
  onTrackAdd: () => void;
  onTrackRename: (trackId: number, currentName: string) => void;
  onTrackRemove: (trackId: number) => void;
  track: CaptionTrack | null;
  cues: readonly SubtitleItem[];
  playhead: number;
  frameRate: { numerator: number; denominator: number };
  selectedCueId: number | null;
  onSelect: (id: number | null) => void;
  onAdd: () => void;
  onEditText: (id: number, text: string) => void;
  onEditSpeaker: (id: number, speaker: string | null) => void;
  onNudge: (id: number, deltaTicks: number) => void;
  onDelete: (id: number) => void;
  onImport: () => void;
  onExport: (format: "srt" | "vtt") => void;
}

/** Cue editor row: timecode jump, inline text + speaker, frame nudge, delete. */
function CueRow({
  cue,
  isActive,
  isSelected,
  frameRate,
  frameStep,
  onSelect,
  onEditText,
  onEditSpeaker,
  onNudge,
  onDelete,
}: {
  cue: SubtitleItem;
  isActive: boolean;
  isSelected: boolean;
  frameRate: { numerator: number; denominator: number };
  frameStep: number;
  onSelect: (id: number) => void;
  onEditText: (id: number, text: string) => void;
  onEditSpeaker: (id: number, speaker: string | null) => void;
  onNudge: (id: number, deltaTicks: number) => void;
  onDelete: (id: number) => void;
}) {
  const [text, setText] = useState(cue.text);
  const [speaker, setSpeaker] = useState(cue.speaker ?? "");

  const commitText = () => {
    if (text.trim() === "") {
      setText(cue.text);
      return;
    }
    if (text !== cue.text) onEditText(cue.id, text.trim());
  };

  const commitSpeaker = () => {
    if (speaker !== (cue.speaker ?? "")) {
      onEditSpeaker(cue.id, speaker.trim() === "" ? null : speaker.trim());
    }
  };

  return (
    <li
      className={`captions-row${isActive ? " is-active" : ""}${isSelected ? " is-selected" : ""}`}
    >
      <button
        type="button"
        className="captions-row-timestamp"
        onClick={() => onSelect(cue.id)}
        title="Seek to cue start"
      >
        {formatTimecodeFrames(cue.start_ticks, frameRate)}
      </button>
      <input
        className="captions-row-text"
        value={text}
        onChange={(event) => setText(event.target.value)}
        onBlur={commitText}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
        aria-label={`Cue text at ${formatTimecodeFrames(cue.start_ticks, frameRate)}`}
      />
      <input
        className="captions-row-speaker"
        value={speaker}
        placeholder="Speaker"
        onChange={(event) => setSpeaker(event.target.value)}
        onBlur={commitSpeaker}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
        aria-label={`Cue speaker at ${formatTimecodeFrames(cue.start_ticks, frameRate)}`}
      />
      <span className="captions-row-nudge">
        <InteriorButton
          size="sm"
          onClick={() => onNudge(cue.id, -frameStep)}
          title="Nudge one frame earlier"
        >
          −1f
        </InteriorButton>
        <InteriorButton
          size="sm"
          onClick={() => onNudge(cue.id, frameStep)}
          title="Nudge one frame later"
        >
          +1f
        </InteriorButton>
      </span>
      <InteriorButton size="sm" onClick={() => onDelete(cue.id)} title="Delete cue">
        <Icon name="close" />
      </InteriorButton>
    </li>
  );
}

/**
 * Caption cue list for the default caption track: playhead-synced, inline
 * text/speaker editing, frame-accurate nudging, and SRT/VTT import/export.
 * Timing and text invariants are enforced by the engine commands.
 */
export function CaptionsPanel({
  captionTracks,
  onTrackChange,
  onTrackAdd,
  onTrackRename,
  onTrackRemove,
  track,
  cues,
  playhead,
  frameRate,
  selectedCueId,
  onSelect,
  onAdd,
  onEditText,
  onEditSpeaker,
  onNudge,
  onDelete,
  onImport,
  onExport,
}: CaptionsPanelProps) {
  const ordered = sortedCues(cues);
  const active = activeCue(ordered, playhead);
  const frameStep = frameTicks(frameRate);

  const renameTrack = () => {
    if (!track) return;
    const value = window.prompt("Rename caption track", track.name);
    if (value === null) return;
    const next = value.trim();
    if (next && next !== track.name) onTrackRename(track.id, next);
  };

  return (
    <section className="captions-panel" aria-label="Captions">
      <div className="captions-trackbar">
        <select
          className="captions-track-select"
          aria-label="Caption track"
          value={(track ?? captionTracks[0])?.id ?? ""}
          onChange={(event) => onTrackChange(Number(event.currentTarget.value))}
        >
          {(captionTracks.length > 0 ? captionTracks : []).map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.name} ({entry.language})
            </option>
          ))}
        </select>
        <InteriorButton size="sm" onClick={onTrackAdd} title="Add a caption track">
          <Icon name="add" />
        </InteriorButton>
        <InteriorButton
          size="sm"
          onClick={renameTrack}
          disabled={!track}
          title="Rename this caption track"
        >
          <Icon name="pen" />
        </InteriorButton>
        <InteriorButton
          size="sm"
          onClick={() => track && onTrackRemove(track.id)}
          disabled={!track || captionTracks.length <= 1}
          title={
            captionTracks.length <= 1
              ? "The last caption track cannot be removed"
              : "Remove this caption track"
          }
        >
          <Icon name="remove" />
        </InteriorButton>
      </div>
      <div className="captions-panel-toolbar">
        <InteriorButton size="sm" onClick={onAdd} title="Add a caption cue at the playhead">
          <Icon name="add" /> Cue
        </InteriorButton>
        <span className="captions-panel-spacer" />
        <InteriorButton size="sm" onClick={onImport} title="Import SRT/VTT into this track">
          <Icon name="import" />
        </InteriorButton>
        <InteriorButton size="sm" onClick={() => onExport("srt")} title="Export as SubRip (.srt)">
          SRT
        </InteriorButton>
        <InteriorButton size="sm" onClick={() => onExport("vtt")} title="Export as WebVTT (.vtt)">
          VTT
        </InteriorButton>
      </div>
      {track ? (
        <p className="captions-panel-track">
          {track.name} · <span className="captions-track-language">{track.language}</span>
        </p>
      ) : null}
      {ordered.length === 0 ? (
        <p className="captions-panel-empty">No caption cues yet — add one at the playhead.</p>
      ) : (
        <ul className="captions-panel-list">
          {ordered.map((cue) => (
            <CueRow
              key={cue.id}
              cue={cue}
              isActive={active?.id === cue.id}
              isSelected={selectedCueId === cue.id}
              frameRate={frameRate}
              frameStep={frameStep}
              onSelect={onSelect}
              onEditText={onEditText}
              onEditSpeaker={onEditSpeaker}
              onNudge={onNudge}
              onDelete={onDelete}
            />
          ))}
        </ul>
      )}
    </section>
  );
}
