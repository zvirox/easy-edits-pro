import type { PointerEvent } from "react";
import { Icon } from "../components/Icon";
import { formatTime, getClipWaveform, TICKS_PER_SECOND } from "../utils/formatters";
import type { MediaAsset, SelectedClip, Track } from "../types";

interface FairlightTimelineTrayProps {
  tracks: Track[];
  media: MediaAsset[];
  playhead: number;
  timelineZoom: number;
  timelineWidth: number;
  selectedClip: SelectedClip | null;
  setSelectedClip: (clip: SelectedClip | null) => void;
  setSelectedClips: (clips: SelectedClip[]) => void;
  zoomTimeline: (factor: number) => void;
  beginTimelineScrub: (event: PointerEvent<HTMLDivElement>) => void;
  moveTimelineScrub: (event: PointerEvent<HTMLDivElement>) => void;
  finishTimelineScrub: (event: PointerEvent<HTMLDivElement>) => void;
  toggleTrackMute: (track: Track) => Promise<void> | void;
  toggleTrackSolo: (track: Track) => Promise<void> | void;
}

export function FairlightTimelineTray(props: FairlightTimelineTrayProps) {
  const {
    tracks, media, playhead, timelineZoom, timelineWidth, selectedClip,
    setSelectedClip, setSelectedClips, zoomTimeline, beginTimelineScrub,
    moveTimelineScrub, finishTimelineScrub, toggleTrackMute, toggleTrackSolo,
  } = props;
  return (
      <section className="audio-timeline-tray" style={{ height: "220px", minHeight: "220px" }}>
        <div className="audio-tray-header">
          <div className="audio-tray-title">
            <strong>AUDIO SEQUENCE TIMELINE &amp; WAVEFORMS</strong>
            <code>{formatTime(playhead)}</code>
          </div>
          <div className="audio-tray-tools">
            <button className="timeline-icon-button" onClick={() => zoomTimeline(1 / 1.2)} title="Zoom Out Timeline">
              <Icon name="zoomOut" />
            </button>
            <button className="timeline-icon-button" onClick={() => zoomTimeline(1.2)} title="Zoom In Timeline">
              <Icon name="zoomIn" />
            </button>
          </div>
        </div>
        <div className="audio-tray-scroller">
          <div className="timeline-content" style={{ width: `${timelineWidth}px`, minWidth: `${timelineWidth}px` }}>
            <div className="timeline-ruler">
              <span className="track-spacer">Time</span>
              <div
                className="ruler-lane"
                onPointerDown={beginTimelineScrub}
                onPointerMove={moveTimelineScrub}
                onPointerUp={finishTimelineScrub}
                onPointerCancel={finishTimelineScrub}
              >
                {Array.from({ length: Math.ceil(timelineWidth / (timelineZoom * 5)) + 1 }, (_, index) => (
                  <span key={index} style={{ left: `${index * timelineZoom * 5}px` }}>
                    {formatTime(index * TICKS_PER_SECOND * 5)}
                  </span>
                ))}
                <div
                  className="playhead-grip ruler-playhead-grip"
                  style={{ left: `${(playhead / TICKS_PER_SECOND) * timelineZoom}px` }}
                  role="slider"
                  aria-label="Audio timeline playhead"
                >
                  <i className="playhead" />
                </div>
              </div>
            </div>
            {tracks.map((track) => (
              <div
                className={`track-row ${track.locked ? "is-locked" : ""} ${track.muted ? "is-muted" : ""}`}
                key={`audio-tray-track-${track.id}`}
              >
                <div className="track-label">
                  <div className="track-label-top">
                    <b>{track.name}</b>
                    <span>{track.kind}</span>
                  </div>
                  <div className="track-label-btns">
                    <button
                      type="button"
                      className={`track-mini-btn mute-btn ${track.muted ? "active" : ""}`}
                      onClick={() => void toggleTrackMute(track)}
                    >
                      M
                    </button>
                    <button
                      type="button"
                      className={`track-mini-btn solo-btn ${track.solo ? "active" : ""}`}
                      onClick={() => void toggleTrackSolo(track)}
                    >
                      S
                    </button>
                  </div>
                </div>
                <div className="track-lane">
                  {track.clips.map((clip, index) => {
                    const item = media.find((entry) => entry.id === clip.media_id);
                    const selected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
                    return (
                      <div
                        className={`timeline-clip ${selected ? "selected" : ""} ${track.kind}`}
                        key={`audio-tray-clip-${track.id}-${clip.media_id}-${index}`}
                        style={{
                          left: `${(clip.timeline_start / TICKS_PER_SECOND) * timelineZoom}px`,
                          width: `${Math.max(34, (clip.duration / TICKS_PER_SECOND) * timelineZoom)}px`,
                        }}
                        onClick={() => {
                          setSelectedClip({ track_id: track.id, clip_index: index });
                          setSelectedClips([{ track_id: track.id, clip_index: index }]);
                        }}
                      >
                        {(() => {
                          const wf = getClipWaveform(item, index);
                          if (!wf || wf.length === 0) return null;
                          const clipStart = clip.timeline_start;
                          const clipDuration = Math.max(1, clip.duration);
                          const isCurrentClip = playhead >= clipStart && playhead < clipStart + clipDuration;
                          const progressRatio = isCurrentClip
                            ? Math.max(0, Math.min(1, (playhead - clipStart) / clipDuration))
                            : playhead >= clipStart + clipDuration
                            ? 1
                            : 0;
                          const progressPct = (progressRatio * 100).toFixed(1);
                          const gradId = `wf-grad-tray-${track.id}-${clip.media_id}-${index}`;
                          const isAudio = track.kind === "audio" || item?.kind === "audio";
                          const activeColor = isAudio ? "#34d399" : "#38bdf8";
                          const inactiveColor = isAudio ? "rgba(52, 211, 153, 0.42)" : "rgba(56, 189, 248, 0.42)";

                          return (
                            <div className="clip-waveform-container" aria-hidden="true">
                              <svg className="clip-waveform-svg" viewBox="0 0 100 32" preserveAspectRatio="none">
                                <defs>
                                  <linearGradient id={gradId} x1="0%" y1="0%" x2="100%" y2="0%">
                                    <stop offset="0%" stopColor={activeColor} />
                                    <stop offset={`${progressPct}%`} stopColor={activeColor} />
                                    <stop offset={`${progressPct}%`} stopColor={inactiveColor} />
                                    <stop offset="100%" stopColor={inactiveColor} />
                                  </linearGradient>
                                </defs>
                                <path
                                  d={wf.map((peak, i) => {
                                    const norm = Math.max(1, (peak / 255) * 14);
                                    const x = (i / wf.length) * 100;
                                    return `M ${x.toFixed(1)} ${(16 - norm).toFixed(1)} L ${x.toFixed(1)} ${(16 + norm).toFixed(1)}`;
                                  }).join(" ")}
                                  stroke={`url(#${gradId})`}
                                  strokeWidth="1.2"
                                  strokeLinecap="round"
                                />
                                {isCurrentClip && (
                                  <line
                                    x1={`${progressPct}%`}
                                    y1="0"
                                    x2={`${progressPct}%`}
                                    y2="32"
                                    stroke="#ffffff"
                                    strokeWidth="1.5"
                                    opacity="0.95"
                                  />
                                )}
                              </svg>
                            </div>
                          );
                        })()}
                        <span className="clip-kind-icon">{track.kind === "audio" ? "♪" : "▶"}</span>
                        <strong>{item?.name ?? "Clip"}</strong>
                      </div>
                    );
                  })}
                  <div
                    className="playhead-grip"
                    style={{ left: `${(playhead / TICKS_PER_SECOND) * timelineZoom}px` }}
                  >
                    <i className="playhead" />
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>
  );
}
