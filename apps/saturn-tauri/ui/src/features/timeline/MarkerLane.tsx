import type { CSSProperties } from "react";
import type { Marker } from "../../app/projectTypes";
import { formatTime } from "../../utils/formatters";
import {
  MARKER_KINDS,
  isRangeMarker,
  markerColor,
  markerLeftPx,
  markerWidthPx,
  sortedMarkers,
} from "../markers/markerList";

export interface MarkerLaneProps {
  markers: readonly Marker[];
  selectedMarkerId: number | null;
  /** Timeline zoom in pixels per second. */
  pixelsPerSecond: number;
  onSelect: (marker: Marker) => void;
  onOpenPanel: () => void;
}

function kindLabel(kind: Marker["kind"]): string {
  return MARKER_KINDS.find((entry) => entry.value === kind)?.label ?? "Comment";
}

/**
 * Marker lane above the ruler. Point markers render as pins, range markers as
 * bars; both sit on the same x-axis as clips and the playhead. No glass or
 * motion here — this is timeline media surface.
 */
export function MarkerLane({
  markers,
  selectedMarkerId,
  pixelsPerSecond,
  onSelect,
  onOpenPanel,
}: MarkerLaneProps) {
  const ordered = sortedMarkers(markers);
  return (
    <div className="marker-lane" aria-label="Sequence markers">
      {ordered.length === 0 && (
        <span className="marker-lane-hint">M adds a marker · Shift+M marks a range</span>
      )}
      {ordered.map((marker) => {
        const range = isRangeMarker(marker);
        const left = markerLeftPx(marker, pixelsPerSecond);
        const width = markerWidthPx(marker, pixelsPerSecond);
        const style = {
          left: `${left}px`,
          width: range ? `${width}px` : undefined,
          "--marker-color": markerColor(marker),
        } as CSSProperties;
        const title = `${marker.name} · ${kindLabel(marker.kind)} · ${formatTime(marker.start_ticks)}`;
        return (
          <button
            key={marker.id}
            type="button"
            className={[
              "marker-pin",
              range ? "is-range" : "is-point",
              marker.id === selectedMarkerId ? "is-selected" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            style={style}
            title={title}
            aria-label={title}
            aria-pressed={marker.id === selectedMarkerId}
            onPointerDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation();
              onSelect(marker);
            }}
            onDoubleClick={(event) => {
              event.stopPropagation();
              onOpenPanel();
            }}
          >
            <i className="marker-pin-cap" aria-hidden="true" />
            <span className="marker-pin-label">{marker.name}</span>
          </button>
        );
      })}
    </div>
  );
}
