import { useEffect, useState } from "react";
import { InteriorButton } from "../../components/interior/interior-button";
import { GlassDialog } from "../../motion/GlassDialog";
import type { Clip } from "../../app/projectTypes";
import { TICKS_PER_SECOND } from "../../app/projectTypes";
import { formatTime } from "../../utils/formatters";
import type { SpeedDurationApply } from "../../app/useTimelineOperations";

export interface SpeedDurationModalProps {
  isOpen: boolean;
  onClose: () => void;
  clip: Clip | null;
  mediaName: string;
  onApply: (patch: SpeedDurationApply) => Promise<boolean>;
}

/**
 * Speed/Duration changes the selected clip in place. Duration and speed are two
 * views of the same stretch, and linked audio can follow the same target speed
 * when the clip belongs to a linked group.
 */
export function SpeedDurationModal({
  isOpen,
  onClose,
  clip,
  mediaName,
  onApply,
}: SpeedDurationModalProps) {
  const [mode, setMode] = useState<"speed" | "duration">("speed");
  const [speedPercent, setSpeedPercent] = useState("100");
  const [durationSeconds, setDurationSeconds] = useState("0");
  const [linked, setLinked] = useState(true);

  const clipSpeed = clip?.speed ?? 0;
  const oldSpeed = clipSpeed > 0 ? clipSpeed : 1;
  const clipDuration = clip?.duration ?? 0;

  useEffect(() => {
    if (!isOpen || !clip) return;
    setMode("speed");
    setSpeedPercent(String(Math.round(oldSpeed * 100)));
    setDurationSeconds((clip.duration / TICKS_PER_SECOND).toFixed(3));
    setLinked(clip.group_id != null);
  }, [isOpen, clip, oldSpeed]);

  const parsedPercent = Number(speedPercent);
  const parsedSeconds = Number(durationSeconds);
  const targetSpeed =
    mode === "speed" ? parsedPercent / 100 : 0;
  const targetDuration =
    mode === "duration"
      ? Math.round(parsedSeconds * TICKS_PER_SECOND)
      : Math.round((clipDuration * oldSpeed) / Math.max(targetSpeed, 0.0001));
  const resultingSpeed =
    mode === "speed"
      ? targetSpeed
      : targetDuration > 0
        ? (clipDuration * oldSpeed) / targetDuration
        : 0;
  const hasLinkedGroup = clip?.group_id != null;
  const valid =
    clip !== null &&
    Number.isFinite(parsedPercent) &&
    Number.isFinite(parsedSeconds) &&
    resultingSpeed >= 0.05 &&
    resultingSpeed <= 20 &&
    targetDuration > 0;

  const submit = () => {
    if (!clip || !valid) return;
    const patch: SpeedDurationApply =
      mode === "speed"
        ? { speed: targetSpeed, linked: linked && hasLinkedGroup }
        : { new_duration: targetDuration, linked: linked && hasLinkedGroup };
    void onApply(patch).then((applied) => {
      if (applied) onClose();
    });
  };

  return (
    <GlassDialog
      as="form"
      open={isOpen}
      cardClassName="modal-dialog-card glass speed-duration-dialog"
      cardProps={{
        style: { maxWidth: "460px" },
        onSubmit: (event) => {
          event.preventDefault();
          submit();
        },
      }}
    >
      <div className="modal-header">
        <div>
          <h2>Speed / Duration</h2>
          <p className="modal-desc">
            {mediaName || "Selected clip"} · {formatTime(clipDuration)}
          </p>
        </div>
        <InteriorButton type="button" className="close-btn" onClick={onClose}>
          ✕
        </InteriorButton>
      </div>

      <div className="speed-duration-modes" role="group" aria-label="Speed or duration">
        <InteriorButton
          type="button"
          className={mode === "speed" ? "active" : ""}
          onClick={() => setMode("speed")}
        >
          Speed
        </InteriorButton>
        <InteriorButton
          type="button"
          className={mode === "duration" ? "active" : ""}
          onClick={() => setMode("duration")}
        >
          Duration
        </InteriorButton>
      </div>

      {mode === "speed" ? (
        <label className="speed-duration-field">
          Speed
          <div className="speed-duration-input-row">
            <input
              autoFocus
              aria-label="Speed percent"
              type="number"
              min="5"
              max="2000"
              step="5"
              value={speedPercent}
              onChange={(event) => setSpeedPercent(event.target.value)}
            />
            <span>%</span>
          </div>
        </label>
      ) : (
        <label className="speed-duration-field">
          Duration
          <div className="speed-duration-input-row">
            <input
              autoFocus
              aria-label="Duration seconds"
              type="number"
              min="0.001"
              step="0.1"
              value={durationSeconds}
              onChange={(event) => setDurationSeconds(event.target.value)}
            />
            <span>sec</span>
          </div>
        </label>
      )}

      <div className="speed-duration-summary">
        <span>
          Result <strong>{formatTime(Math.max(0, targetDuration))}</strong>
        </span>
        <span>
          Speed <strong>{Number.isFinite(resultingSpeed) ? resultingSpeed.toFixed(2) : "0.00"}×</strong>
        </span>
      </div>

      {!valid && (
        <p className="speed-duration-error" role="status">
          {clip
            ? "Speed must stay between 0.05× and 20×."
            : "Select a timeline clip first."}
        </p>
      )}

      <label
        className={`speed-duration-linked ${hasLinkedGroup ? "" : "is-disabled"}`}
        title={hasLinkedGroup ? undefined : "This clip is not in a linked group"}
      >
        <input
          type="checkbox"
          checked={linked && hasLinkedGroup}
          disabled={!hasLinkedGroup}
          onChange={(event) => setLinked(event.target.checked)}
        />
        Include linked audio
      </label>

      <div className="dialog-actions">
        <InteriorButton type="button" onClick={onClose}>
          Cancel
        </InteriorButton>
        <InteriorButton className="primary" type="submit" disabled={!valid}>
          Apply
        </InteriorButton>
      </div>
    </GlassDialog>
  );
}
