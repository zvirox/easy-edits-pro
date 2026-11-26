// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { Clock, Film, Gauge } from "lucide-react";
import { StatCard } from "../components/linear";
import { formatTime } from "../utils/formatters";

export interface DeliverStatsStripProps {
  rangeDuration: number;
  width: number;
  height: number;
  fpsNumerator: number;
  fpsDenominator: number;
  exportFormat: "mp4" | "webm";
  hasHardwareEncoder: boolean;
  preferredEncoder: string;
  useHardwareEncoding: boolean;
}

export function DeliverStatsStrip({
  rangeDuration,
  width,
  height,
  fpsNumerator,
  fpsDenominator,
  exportFormat,
  hasHardwareEncoder,
  preferredEncoder,
  useHardwareEncoding,
}: DeliverStatsStripProps) {
  const fpsDisplay =
    fpsDenominator === 1
      ? `${fpsNumerator}`
      : (fpsNumerator / Math.max(1, fpsDenominator)).toFixed(2);
  const hwActive = hasHardwareEncoder && useHardwareEncoding && exportFormat === "mp4";

  return (
    <div className="grid grid-cols-3 gap-2.5 my-3 min-w-0">
      <StatCard
        compact
        title="Duration"
        value={formatTime(rangeDuration)}
        unit="TC"
        icon={<Clock className="size-3.5" aria-hidden="true" />}
        iconBgClass="bg-primary-tint border border-primary"
        iconColorClass="text-primary-text"
      />
      <StatCard
        compact
        title="Frame Size"
        value={`${width}×${height}`}
        unit={`${fpsDisplay} FPS`}
        icon={<Film className="size-3.5" aria-hidden="true" />}
        iconBgClass="bg-chip-sky border border-sky"
        iconColorClass="text-sky-text"
      />
      <StatCard
        compact
        title="Encoder Speed"
        value={hwActive ? preferredEncoder.toUpperCase() : "x264"}
        unit={hwActive ? "GPU" : "CPU"}
        icon={<Gauge className="size-3.5" aria-hidden="true" />}
        iconBgClass={
          hwActive
            ? "bg-chip-sky border border-sky"
            : "bg-surface-2 border border-amber"
        }
        iconColorClass={
          hwActive ? "text-sky-text" : "text-amber"
        }
        delta={
          hwActive
            ? { value: "HW", positive: true, comparison: "Accelerated" }
            : { value: "SW", positive: false, comparison: "Fallback" }
        }
      />
    </div>
  );
}
