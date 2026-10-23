import { useEffect, useRef, useState } from "react";
import { audioClockMaster } from "../../engine/playbackEngine";
import type { LoudnessReport } from "../../app/projectTypes";

// G4 (plan v4): the 92px master meter card that closes the persistent NLE
// row — toolbox | timeline | meter. Part of the persistent skeleton: once
// mounted by WorkspaceShell it never unmounts on a workspace switch.
const POLL_INTERVAL_MS = 80;
const BAR_FLOOR_DB = -60;
const BAR_CEIL_DB = 0;

interface MasterLevels {
  left: number;
  right: number;
  peakDb: number;
}

const SILENT_REPORT: LoudnessReport = {
  momentaryLufs: -70,
  shortTermLufs: -70,
  integratedLufs: -70,
  loudnessRangeLu: 0,
  truePeakDbtp: -70,
  targetLufs: -23,
  isCompliant: true,
  gainRecommendationDb: 0,
};

function toPercent(linear: number): number {
  if (!Number.isFinite(linear) || linear <= 0) return 0;
  const db = 20 * Math.log10(linear);
  const clamped = Math.max(BAR_FLOOR_DB, Math.min(BAR_CEIL_DB, db));
  return Math.round(((clamped - BAR_FLOOR_DB) / (BAR_CEIL_DB - BAR_FLOOR_DB)) * 100);
}

function formatDb(value: number): string {
  if (!Number.isFinite(value)) return "-inf";
  return value.toFixed(1);
}

export function MasterMeter() {
  const [levels, setLevels] = useState<MasterLevels>({ left: 0, right: 0, peakDb: -60 });
  const [report, setReport] = useState<LoudnessReport>(SILENT_REPORT);
  const frameRef = useRef(0);

  useEffect(() => {
    let lastPoll = 0;
    const poll = (now: number) => {
      if (now - lastPoll > POLL_INTERVAL_MS) {
        lastPoll = now;
        setLevels(audioClockMaster.getMasterLevels());
        setReport(audioClockMaster.getLoudnessReport());
      }
      frameRef.current = requestAnimationFrame(poll);
    };
    frameRef.current = requestAnimationFrame(poll);
    return () => cancelAnimationFrame(frameRef.current);
  }, []);

  return (
    <aside className="shell-meter" aria-label="Master audio meter">
      <span className="shell-meter-title">Audio</span>
      <div className="shell-meter-bars" aria-hidden="true">
        <div className="shell-meter-bar">
          <span className="shell-meter-fill" style={{ height: `${toPercent(levels.left)}%` }} />
        </div>
        <div className="shell-meter-bar">
          <span className="shell-meter-fill" style={{ height: `${toPercent(levels.right)}%` }} />
        </div>
      </div>
      <code className="shell-meter-readout">
        M {formatDb(report.momentaryLufs)}
        <br />I {formatDb(report.integratedLufs)}
        <br />TP {formatDb(report.truePeakDbtp)}
      </code>
    </aside>
  );
}
