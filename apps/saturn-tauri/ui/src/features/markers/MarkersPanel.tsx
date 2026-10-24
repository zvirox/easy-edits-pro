import { useState } from "react";
import type { Marker, MarkerKind } from "../../app/projectTypes";
import { formatTimecodeFrames } from "../../utils/formatters";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import { MARKER_KINDS, markerColor, markerShiftTicks, sortedMarkers } from "./markerList";

export interface MarkersPanelProps {
  markers: readonly Marker[];
  selectedMarkerId: number | null;
  frameRate: { numerator: number; denominator: number };
  onSelect: (id: number | null) => void;
  onSeek: (tick: number) => void;
  onRename: (id: number, name: string) => void;
  onUpdateKind: (id: number, kind: MarkerKind) => void;
  onDelete: (id: number) => void;
  onClear: () => void;
  onRipple: (deltaTicks: number) => void;
  onAdd: () => void;
  onAddRange: () => void;
  onImport: () => void;
  onExport: () => void;
}

/**
 * Marker list with inline rename, kind switching, and jump-to-marker. Covered
 * by the marker lane in the timeline; both surfaces share the same commands.
 */
export function MarkersPanel({
  markers,
  selectedMarkerId,
  frameRate,
  onSelect,
  onSeek,
  onRename,
  onUpdateKind,
  onDelete,
  onClear,
  onRipple,
  onAdd,
  onAddRange,
  onImport,
  onExport,
}: MarkersPanelProps) {
  const ordered = sortedMarkers(markers);
  const [shiftFrames, setShiftFrames] = useState("");

  return (
    <section className="markers-panel" aria-label="Markers">
      <div className="markers-panel-toolbar">
        <InteriorButton size="sm" onClick={onAdd} title="Add a marker at the playhead (M)">
          <Icon name="add" /> Marker
        </InteriorButton>
        <InteriorButton size="sm" onClick={onAddRange} title="Add a range marker (Shift+M)">
          Range
        </InteriorButton>
        <span className="markers-panel-spacer" />
        <InteriorButton size="sm" onClick={onImport} title="Import markers from CSV">
          <Icon name="import" />
        </InteriorButton>
        <InteriorButton size="sm" onClick={onExport} title="Export markers to CSV">
          <Icon name="export" />
        </InteriorButton>
      </div>

      {ordered.length === 0 ? (
        <p className="markers-empty">
          No markers yet. Press <kbd>M</kbd> at the playhead, or import a CSV.
        </p>
      ) : (
        <ol className="marker-list">
          {ordered.map((marker) => (
            <li
              key={marker.id}
              className={`marker-row${marker.id === selectedMarkerId ? " is-selected" : ""}`}
              data-marker-id={marker.id}
            >
              <button
                type="button"
                className="marker-jump"
                title="Go to this marker"
                onClick={() => {
                  onSelect(marker.id);
                  onSeek(marker.start_ticks);
                }}
              >
                <i
                  className="marker-dot"
                  style={{ background: markerColor(marker) }}
                  aria-hidden="true"
                />
                <span className="marker-time">
                  {formatTimecodeFrames(marker.start_ticks, frameRate)}
                </span>
              </button>
              <input
                className="marker-name-input"
                defaultValue={marker.name}
                aria-label={`Marker name for ${marker.name}`}
                onFocus={() => onSelect(marker.id)}
                onBlur={(event) => {
                  const next = event.currentTarget.value.trim();
                  if (next && next !== marker.name) onRename(marker.id, next);
                  if (!next) event.currentTarget.value = marker.name;
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter") event.currentTarget.blur();
                  if (event.key === "Escape") {
                    event.currentTarget.value = marker.name;
                    event.currentTarget.blur();
                  }
                }}
              />
              <select
                className="marker-kind-select"
                value={marker.kind}
                aria-label={`Marker kind for ${marker.name}`}
                onChange={(event) =>
                  onUpdateKind(marker.id, event.currentTarget.value as MarkerKind)
                }
              >
                {MARKER_KINDS.map((kind) => (
                  <option key={kind.value} value={kind.value}>
                    {kind.label}
                  </option>
                ))}
              </select>
              <span className="marker-span">
                {marker.duration_ticks > 0 ? "range" : "point"}
              </span>
              <button
                type="button"
                className="marker-delete"
                title={`Delete ${marker.name}`}
                aria-label={`Delete ${marker.name}`}
                onClick={() => onDelete(marker.id)}
              >
                <Icon name="remove" />
              </button>
            </li>
          ))}
        </ol>
      )}

      {ordered.length > 0 && (
        <div className="markers-panel-footer">
          <form
            className="markers-ripple"
            title="Shift every marker at or after the playhead by a signed frame amount"
            onSubmit={(event) => {
              event.preventDefault();
              const frames = Number(shiftFrames.trim());
              if (!Number.isFinite(frames) || frames === 0) return;
              onRipple(markerShiftTicks(frames, frameRate));
              setShiftFrames("");
            }}
          >
            <label className="markers-ripple-label" htmlFor="markers-ripple-input">
              Shift from playhead
            </label>
            <input
              id="markers-ripple-input"
              className="markers-ripple-input"
              type="number"
              step="1"
              placeholder="+12 / -24"
              value={shiftFrames}
              onChange={(event) => setShiftFrames(event.currentTarget.value)}
            />
            <InteriorButton size="sm" type="submit">
              Shift
            </InteriorButton>
          </form>
          <span className="markers-count">{ordered.length} markers</span>
          <button type="button" className="markers-clear" onClick={onClear}>
            Clear all
          </button>
        </div>
      )}
    </section>
  );
}
