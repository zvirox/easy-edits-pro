import { InteriorButton } from "./interior/interior-button";
import { useState, useRef, useEffect, useCallback, type MouseEvent } from "react";
import type { MediaAsset } from "../types";
import { formatTime, getClipWaveform, TICKS_PER_SECOND } from "../utils/formatters";

export interface SourceAudioVisualizerProps {
  item: MediaAsset;
  src: string;
}

export function SourceAudioVisualizer({ item, src }: SourceAudioVisualizerProps) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTimeSec, setCurrentTimeSec] = useState(0);
  const [durationSec, setDurationSec] = useState(() => (item.duration ? item.duration / TICKS_PER_SECOND : 0));
  const [volume, setVolume] = useState(1.0);
  const [isMuted, setIsMuted] = useState(false);
  const [isLooping, setIsLooping] = useState(false);
  const waveform = getClipWaveform(item);

  const togglePlay = useCallback(() => {
    const el = audioRef.current;
    if (!el) return;
    if (el.paused) {
      void el.play().catch(() => {});
    } else {
      el.pause();
    }
  }, []);

  const seekRelative = useCallback((deltaSec: number) => {
    const el = audioRef.current;
    if (!el) return;
    el.currentTime = Math.max(0, Math.min(el.duration || 999999, el.currentTime + deltaSec));
  }, []);

  const handleWaveformClick = (e: MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const targetSec = ratio * (durationSec || 1);
    if (audioRef.current) {
      audioRef.current.currentTime = targetSec;
    }
    setCurrentTimeSec(targetSec);
  };

  useEffect(() => {
    const el = audioRef.current;
    if (!el) return;
    setIsPlaying(!el.paused);
    setCurrentTimeSec(el.currentTime);
  }, [src]);

  const progressPct = durationSec > 0 ? (currentTimeSec / durationSec) * 100 : 0;
  const currentTicks = Math.round(currentTimeSec * TICKS_PER_SECOND);
  const totalTicks = Math.round(durationSec * TICKS_PER_SECOND);

  return (
    <div className="source-audio-stage">
      {/* Hidden Audio Player */}
      <audio
        ref={audioRef}
        key={src}
        src={src}
        loop={isLooping}
        onTimeUpdate={(e) => setCurrentTimeSec(e.currentTarget.currentTime)}
        onDurationChange={(e) => {
          if (e.currentTarget.duration && !Number.isNaN(e.currentTarget.duration)) {
            setDurationSec(e.currentTarget.duration);
          }
        }}
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onEnded={() => setIsPlaying(false)}
      />

      {/* Top Metadata Header */}
      <div className="source-audio-top-info">
        <div className="source-audio-badge">
          <span>♪ AUDIO CLIP</span>
          <span style={{ color: "var(--text-muted)", fontSize: "9px" }}>
            {item.name}
          </span>
        </div>
        <div className="source-audio-tc">
          {formatTime(currentTicks)} / {formatTime(totalTicks)}
        </div>
      </div>

      {/* Interactive Waveform Visualizer Display */}
      <div
        className="source-audio-waveform-wrapper"
        onClick={handleWaveformClick}
        title="Click or scrub to seek audio position"
      >
        <svg
          className="source-audio-waveform-svg"
          viewBox="0 0 100 40"
          preserveAspectRatio="none"
        >
          <defs>
            <linearGradient id="sourceAudioGrad" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stopColor="#be185d" />
              <stop offset={`${progressPct.toFixed(1)}%`} stopColor="#be185d" />
              <stop offset={`${progressPct.toFixed(1)}%`} stopColor="rgba(159, 18, 73, 0.35)" />
              <stop offset="100%" stopColor="rgba(159, 18, 73, 0.35)" />
            </linearGradient>
          </defs>
          <path
            d={waveform.map((peak, i) => {
              const norm = Math.max(1, (peak / 255) * 18);
              const x = (i / waveform.length) * 100;
              return `M ${x.toFixed(1)} ${(20 - norm).toFixed(1)} L ${x.toFixed(1)} ${(20 + norm).toFixed(1)}`;
            }).join(" ")}
            stroke="url(#sourceAudioGrad)"
            strokeWidth="1.4"
            strokeLinecap="round"
          />
        </svg>

        {/* Playhead Marker */}
        <div
          className="source-audio-playhead"
          style={{ left: `${progressPct.toFixed(2)}%` }}
        />
      </div>

      {/* Integrated Audio Transport Controls */}
      <div className="source-audio-transport">
        <div className="source-audio-btns">
          <InteriorButton
            type="button"
            className="source-transport-btn"
            onClick={() => seekRelative(-5)}
            title="Rewind 5s (J)"
          >
            ◀◀ -5s
          </InteriorButton>
          <InteriorButton
            type="button"
            className="source-transport-btn"
            onClick={() => seekRelative(-1)}
            title="Step Back 1s"
          >
            ◀|
          </InteriorButton>
          <InteriorButton
            type="button"
            className="source-transport-btn play"
            onClick={togglePlay}
            title="Play / Pause (Space)"
          >
            {isPlaying ? "⏸ Pause" : "▶ Play"}
          </InteriorButton>
          <InteriorButton
            type="button"
            className="source-transport-btn"
            onClick={() => seekRelative(1)}
            title="Step Forward 1s"
          >
            |▶
          </InteriorButton>
          <InteriorButton
            type="button"
            className="source-transport-btn"
            onClick={() => seekRelative(5)}
            title="Fast Forward 5s (L)"
          >
            +5s ▶▶
          </InteriorButton>
          <InteriorButton
            type="button"
            className={`source-transport-btn ${isLooping ? "active-toggle" : ""}`}
            onClick={() => setIsLooping((v) => !v)}
            title="Toggle Loop Playback"
          >
            🔁 {isLooping ? "Loop On" : "Loop"}
          </InteriorButton>
        </div>

        {/* Volume & Mute Controls */}
        <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          <InteriorButton
            type="button"
            className="source-transport-btn"
            onClick={() => {
              if (audioRef.current) {
                audioRef.current.muted = !isMuted;
                setIsMuted(!isMuted);
              }
            }}
            title={isMuted ? "Unmute" : "Mute"}
          >
            {isMuted ? "🔇" : "🔊"}
          </InteriorButton>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={isMuted ? 0 : volume}
            onChange={(e) => {
              const val = Number(e.target.value);
              setVolume(val);
              setIsMuted(false);
              if (audioRef.current) {
                audioRef.current.volume = val;
                audioRef.current.muted = false;
              }
            }}
            style={{ width: "70px", accentColor: "var(--accent-primary)" }}
            title={`Volume: ${Math.round(volume * 100)}%`}
          />
        </div>
      </div>
    </div>
  );
}
