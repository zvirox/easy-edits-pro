import React, { useState, useEffect } from "react";
import type { MultiCamClip } from "../engine/playbackEngine";

interface LiveMulticamSwitcherProps {
  multicamClip?: MultiCamClip | null;
  isPlaying: boolean;
  onCutAngle: (angleId: number) => void;
  onSelectPreviewAngle: (angleId: number) => void;
}

export const LiveMulticamSwitcher: React.FC<LiveMulticamSwitcherProps> = ({
  multicamClip,
  isPlaying,
  onCutAngle,
  onSelectPreviewAngle,
}) => {
  const [liveCutMode, setLiveCutMode] = useState(true);
  const [audioFollowVideo, setAudioFollowVideo] = useState(false);
  const [previewAngleId, setPreviewAngleId] = useState<number>(2);

  const angles = multicamClip?.angles || [
    { id: 1, name: "Cam A (Wide Main)", media_id: 1, sync_offset_ticks: 0 },
    { id: 2, name: "Cam B (Close-up)", media_id: 2, sync_offset_ticks: 0 },
    { id: 3, name: "Cam C (Audience/Reaction)", media_id: 3, sync_offset_ticks: 0 },
    { id: 4, name: "Cam D (Overhead / Detail)", media_id: 4, sync_offset_ticks: 0 },
  ];

  const pgmAngleId = multicamClip?.active_angle_id || 1;

  // Keyboard shortcut listener for keys 1-9
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't intercept if user is typing in an input or textarea
      if (
        document.activeElement?.tagName === "INPUT" ||
        document.activeElement?.tagName === "TEXTAREA"
      ) {
        return;
      }

      const keyNum = parseInt(e.key, 10);
      if (keyNum >= 1 && keyNum <= angles.length) {
        e.preventDefault();
        const targetAngle = angles[keyNum - 1];
        if (targetAngle) {
          if (liveCutMode || isPlaying) {
            onCutAngle(targetAngle.id);
          } else {
            setPreviewAngleId(targetAngle.id);
            onSelectPreviewAngle(targetAngle.id);
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [angles, liveCutMode, isPlaying, onCutAngle, onSelectPreviewAngle]);

  const handleAngleClick = (angleId: number) => {
    if (liveCutMode || isPlaying) {
      onCutAngle(angleId);
    } else {
      setPreviewAngleId(angleId);
      onSelectPreviewAngle(angleId);
    }
  };

  return (
    <div
      className="live-multicam-switcher"
      style={{
        background: "#180d17",
        border: "1px solid #462d40",
        borderRadius: 8,
        padding: 14,
        color: "#e2e8f0",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 18 }}>🎥</span>
          <div>
            <h4 style={{ margin: 0, fontSize: 13, fontWeight: 600 }}>Live Multi-Cam Switcher</h4>
            <span style={{ fontSize: 11, color: "#a0aec0" }}>
              Real-time multi-angle switcher with PGM Red &amp; PVW Green tallies
            </span>
          </div>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, color: liveCutMode ? "#e53e3e" : "#a0aec0", cursor: "pointer", fontWeight: 600 }}>
            <input
              type="checkbox"
              checked={liveCutMode}
              onChange={(e) => setLiveCutMode(e.target.checked)}
            />
            🔴 Live Cut Mode (Keys 1-9)
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, color: "#a0aec0", cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={audioFollowVideo}
              onChange={(e) => setAudioFollowVideo(e.target.checked)}
            />
            AFV (Audio Follows Video)
          </label>
        </div>
      </div>

      {/* Camera angle grid */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: angles.length <= 4 ? "repeat(2, 1fr)" : "repeat(3, 1fr)",
          gap: 10,
          marginBottom: 10,
        }}
      >
        {angles.map((angle, idx) => {
          const isPgm = angle.id === pgmAngleId;
          const isPvw = angle.id === previewAngleId && !isPgm;

          return (
            <div
              key={angle.id}
              onClick={() => handleAngleClick(angle.id)}
              style={{
                position: "relative",
                background: "#110910",
                border: "2px solid",
                borderColor: isPgm ? "#e53e3e" : isPvw ? "#38a169" : "#462d40",
                borderRadius: 6,
                padding: "10px 12px",
                cursor: "pointer",
                transition: "all 0.15s ease",
                boxShadow: isPgm
                  ? "0 0 12px rgba(229, 62, 62, 0.4)"
                  : isPvw
                  ? "0 0 10px rgba(56, 161, 105, 0.3)"
                  : "none",
              }}
            >
              {/* Tally Badge Header */}
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <span
                  style={{
                    fontSize: 10,
                    fontWeight: 700,
                    padding: "2px 6px",
                    borderRadius: 3,
                    background: isPgm ? "#e53e3e" : isPvw ? "#38a169" : "#100810",
                    color: "#ffffff",
                    letterSpacing: 0.5,
                  }}
                >
                  {isPgm ? "🔴 PGM" : isPvw ? "🟢 PVW" : `CAM ${idx + 1}`}
                </span>
                <span style={{ fontSize: 11, fontWeight: 700, color: "#8b7284" }}>
                  Key [{idx + 1}]
                </span>
              </div>

              {/* Angle Name */}
              <div style={{ fontSize: 12, fontWeight: 600, color: isPgm ? "#fff" : "#cbd5e0", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                {angle.name}
              </div>

              {/* Status info */}
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#8b7284", marginTop: 4 }}>
                <span>{audioFollowVideo && isPgm ? "🔊 Audio Active" : "Locked Mic A"}</span>
                <span>{isPgm && isPlaying ? "ON AIR" : "Standby"}</span>
              </div>
            </div>
          );
        })}
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 11, color: "#8b7284" }}>
        <span>
          💡 Tip: During playback, press keys <strong>1-{angles.length}</strong> to perform live cuts on the beat.
        </span>
        <span>
          Active: <strong style={{ color: "#e53e3e" }}>CAM {angles.findIndex((a) => a.id === pgmAngleId) + 1}</strong>
        </span>
      </div>
    </div>
  );
};
