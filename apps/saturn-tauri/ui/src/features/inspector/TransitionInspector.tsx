import type { Dispatch, SetStateAction } from "react";
import { formatTime } from "../../utils/formatters";
import { TICKS_PER_SECOND } from "../../engine/playback/constants";
import type { Transition, TransitionKind } from "../../app/projectTypes";

export interface TransitionInspectorProps {
  selectedTransition: { track_id: number; transition: Transition };
  setSelectedTransition: Dispatch<SetStateAction<{ track_id: number; transition: Transition } | null>>;
  updateTransition: (kind: TransitionKind, duration: number) => Promise<void>;
  removeSelectedTransition: () => Promise<void>;
}

export function TransitionInspector({
  selectedTransition,
  setSelectedTransition,
  updateTransition,
  removeSelectedTransition,
}: TransitionInspectorProps) {
  const { transition, track_id } = selectedTransition;
  const durationSeconds = transition.duration / TICKS_PER_SECOND;

  return (
    <>
      <h2>Transition inspector</h2>
      <p>Style and timing for track transitions.</p>
      <div className="control">Track <span>Track {track_id}</span></div>
      <div className="control">Cut position <span>{formatTime(transition.cut_at)}</span></div>
      <label>
        Transition type
        <select
          value={transition.kind}
          onChange={(e) => void updateTransition(e.target.value as TransitionKind, transition.duration)}
        >
          <option value="crossfade">Crossfade (Smooth Blend)</option>
          <option value="dip_to_black">Dip to Black</option>
          <option value="dip_to_white">Dip to White</option>
          <option value="wipe_left">Wipe Left</option>
          <option value="wipe_right">Wipe Right</option>
          <option value="slide_left">Slide Left</option>
          <option value="slide_right">Slide Right</option>
        </select>
      </label>
      <label>
        Duration ({durationSeconds.toFixed(2)}s)
        <input
          type="range"
          min="0.2"
          max="4.0"
          step="0.1"
          value={durationSeconds}
          onChange={(e) => {
            const durTicks = Math.round(Number(e.target.value) * TICKS_PER_SECOND);
            setSelectedTransition({
              track_id,
              transition: { ...transition, duration: durTicks },
            });
          }}
          onPointerUp={(e) => {
            const durTicks = Math.round(Number(e.currentTarget.value) * TICKS_PER_SECOND);
            void updateTransition(transition.kind, durTicks);
          }}
        />
      </label>
      <button className="danger-button" onClick={() => void removeSelectedTransition()}>Remove transition</button>
    </>
  );
}
