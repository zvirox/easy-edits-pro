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
        iconBgClass="bg-[var(--tint-blue-bg)] border border-[var(--tint-blue-border)]"
        iconColorClass="text-[var(--tint-blue-fg)]"
      />
      <StatCard
        compact
        title="Frame Size"
        value={`${width}×${height}`}
        unit={`${fpsDisplay} FPS`}
        icon={<Film className="size-3.5" aria-hidden="true" />}
        iconBgClass="bg-[var(--tint-teal-bg)] border border-[var(--tint-teal-border)]"
        iconColorClass="text-[var(--tint-teal-fg)]"
      />
      <StatCard
        compact
        title="Encoder Speed"
        value={hwActive ? preferredEncoder.toUpperCase() : "x264"}
        unit={hwActive ? "GPU" : "CPU"}
        icon={<Gauge className="size-3.5" aria-hidden="true" />}
        iconBgClass={
          hwActive
            ? "bg-[var(--tint-green-bg)] border border-[var(--tint-green-border)]"
            : "bg-[var(--tint-amber-bg)] border border-[var(--tint-amber-border)]"
        }
        iconColorClass={
          hwActive ? "text-[var(--tint-green-fg)]" : "text-[var(--tint-amber-fg)]"
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
