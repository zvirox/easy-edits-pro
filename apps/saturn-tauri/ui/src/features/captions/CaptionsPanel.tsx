import { useEffect, useRef, useState } from "react";
import type { CaptionStyle, CaptionTrack, SubtitleItem } from "../../app/projectTypes";
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
  /** Drives the program overlay; hidden tracks keep their cues. */
  onToggleTrackVisible: (trackId: number, visible: boolean) => void;
  onShowAllTracks: () => void;
  onHideAllTracks: () => void;
  track: CaptionTrack | null;
  /** The track's reusable style; null until first saved. */
  style: CaptionStyle | null;
  onSaveStyle: (patch: Partial<CaptionStyle>) => void;
  cues: readonly SubtitleItem[];
  playhead: number;
  frameRate: { numerator: number; denominator: number };
  /** Cue ids in the multi-cue selection. */
  selectedCueIds: readonly number[];
  /** Bumped to scroll the active cue into view (caption.go_to). */
  revealTick: number;
  onSelect: (id: number | null, additive?: boolean) => void;
  onSelectAll: () => void;
  onAdd: () => void;
  onEditText: (id: number, text: string) => void;
  onEditSpeaker: (id: number, speaker: string | null) => void;
  onNudge: (id: number, deltaTicks: number) => void;
  onDelete: (id: number) => void;
  onDeleteSelected: () => void;
  onSplit: () => void;
  onMerge: () => void;
  onNext: () => void;
  onPrevious: () => void;
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
  onSelect: (id: number, additive: boolean) => void;
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
      data-cue={cue.id}
    >
      <input
        type="checkbox"
        className="captions-row-check"
        checked={isSelected}
        onChange={(event) => onSelect(cue.id, event.currentTarget.checked)}
        aria-label={`Select cue at ${formatTimecodeFrames(cue.start_ticks, frameRate)}`}
      />
      <button
        type="button"
        className="captions-row-timestamp"
        onClick={(event) =>
          onSelect(cue.id, event.shiftKey || event.metaKey || event.ctrlKey)
        }
        title="Seek to cue start (shift-click to multi-select)"
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
  onToggleTrackVisible,
  onShowAllTracks,
  onHideAllTracks,
  track,
  style,
  onSaveStyle,
  cues,
  playhead,
  frameRate,
  selectedCueIds,
  revealTick,
  onSelect,
  onSelectAll,
  onAdd,
  onEditText,
  onEditSpeaker,
  onNudge,
  onDelete,
  onDeleteSelected,
  onSplit,
  onMerge,
  onNext,
  onPrevious,
  onImport,
  onExport,
}: CaptionsPanelProps) {
  const ordered = sortedCues(cues);
  const active = activeCue(ordered, playhead);
  const frameStep = frameTicks(frameRate);
  const listRef = useRef<HTMLUListElement>(null);
  // Latest reveal target, kept out of the reveal effect's dependencies.
  const activeRef = useRef<number | null>(null);
  activeRef.current = active?.id ?? selectedCueIds[0] ?? null;

  const [styleDraft, setStyleDraft] = useState({
    font_size: style?.font_size ?? 42,
    fill_color: style?.fill_color ?? "#ffffff",
    stroke_color: style?.stroke_color ?? "#000000",
    stroke_width: style?.stroke_width ?? 0,
    background_color: style?.background_color ?? "#000000",
  });

  // caption.go_to: bring the active (or first selected) cue row into view.
  // Only `revealTick` is a dependency: reveal is a one-shot gesture, not a
  // follower of playhead moves.
  useEffect(() => {
    if (revealTick === 0) return;
    const activeId = activeRef.current;
    const row = listRef.current?.querySelector(
      activeId != null
        ? `.captions-row[data-cue="${activeId}"]`
        : ".captions-row.is-active",
    );
    row?.scrollIntoView({ block: "nearest" });
  }, [revealTick]);

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
        <span className="captions-trackbar-spacer" />
        <InteriorButton
          size="sm"
          onClick={() => track && onToggleTrackVisible(track.id, !(track.visible ?? true))}
          disabled={!track}
          title={
            (track?.visible ?? true) ? "Hide this caption track" : "Show this caption track"
          }
        >
          <Icon name={(track?.visible ?? true) ? "eye" : "eyeOff"} />
        </InteriorButton>
        <InteriorButton size="sm" onClick={onHideAllTracks} title="Hide all caption tracks">
          <Icon name="eyeOff" /> All
        </InteriorButton>
        <InteriorButton size="sm" onClick={onShowAllTracks} title="Show all caption tracks">
          <Icon name="eye" /> All
        </InteriorButton>
      </div>
      <div className="captions-stylebar" key={style?.id ?? "new"}>
        <span className="captions-style-label">Style</span>
        <label className="captions-style-field">
          Size
          <input
            type="number"
            aria-label="Caption font size"
            min={8}
            max={200}
            value={styleDraft.font_size}
            onChange={(event) =>
              setStyleDraft((draft) => ({
                ...draft,
                font_size: Number(event.currentTarget.value) || 42,
              }))
            }
          />
        </label>
        <label className="captions-style-field">
          Fill
          <input
            type="color"
            aria-label="Caption fill colour"
            value={styleDraft.fill_color}
            onChange={(event) =>
              setStyleDraft((draft) => ({ ...draft, fill_color: event.currentTarget.value }))
            }
          />
        </label>
        <label className="captions-style-field">
          Stroke
          <input
            type="color"
            aria-label="Caption stroke colour"
            value={styleDraft.stroke_color}
            onChange={(event) =>
              setStyleDraft((draft) => ({ ...draft, stroke_color: event.currentTarget.value }))
            }
          />
        </label>
        <label className="captions-style-field">
          W
          <input
            type="number"
            aria-label="Caption stroke width"
            min={0}
            max={20}
            step={0.5}
            value={styleDraft.stroke_width}
            onChange={(event) =>
              setStyleDraft((draft) => ({
                ...draft,
                stroke_width: Number(event.currentTarget.value) || 0,
              }))
            }
          />
        </label>
        <label className="captions-style-field">
          Bg
          <input
            type="color"
            aria-label="Caption background colour"
            value={styleDraft.background_color}
            onChange={(event) =>
              setStyleDraft((draft) => ({ ...draft, background_color: event.currentTarget.value }))
            }
          />
        </label>
        <InteriorButton
          size="sm"
          disabled={!track}
          title="Save this look to the selected caption track"
          onClick={() => onSaveStyle(styleDraft)}
        >
          Apply
        </InteriorButton>
      </div>
      <div className="captions-panel-toolbar">
        <InteriorButton size="sm" onClick={onAdd} title="Add a caption cue at the playhead">
          <Icon name="add" /> Cue
        </InteriorButton>
        <span className="captions-panel-spacer" />
        <InteriorButton size="sm" onClick={onPrevious} title="Go to previous caption segment">
          <Icon name="trackBackward" />
        </InteriorButton>
        <InteriorButton size="sm" onClick={onNext} title="Go to next caption segment">
          <Icon name="trackForward" />
        </InteriorButton>
        <InteriorButton size="sm" onClick={onSplit} title="Split the cue under the playhead">
          <Icon name="split" />
        </InteriorButton>
        <InteriorButton
          size="sm"
          onClick={onMerge}
          disabled={selectedCueIds.length < 2}
          title="Merge the selected cues"
        >
          <Icon name="link" />
        </InteriorButton>
        <InteriorButton
          size="sm"
          onClick={onDeleteSelected}
          disabled={selectedCueIds.length === 0}
          title="Delete the selected cues"
        >
          <Icon name="remove" />
        </InteriorButton>
        <InteriorButton size="sm" onClick={onSelectAll} title="Select all cues on this track">
          <Icon name="select" />
        </InteriorButton>
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
        <ul className="captions-panel-list" ref={listRef}>
          {ordered.map((cue) => (
            <CueRow
              key={cue.id}
              cue={cue}
              isActive={active?.id === cue.id}
              isSelected={selectedCueIds.includes(cue.id)}
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
