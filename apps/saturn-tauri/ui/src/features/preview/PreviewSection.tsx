import { InteriorButton } from "../../components/interior/interior-button";
import type { CSSProperties, RefObject, MutableRefObject } from "react";
import { formatTime } from "../../utils/formatters";
import { audioClockMaster, type MultiCamClip } from "../../engine/playbackEngine";
import type { MediaAsset, ProjectDocument, TitleProperties } from "../../app/projectTypes";
import { SourceAudioVisualizer } from "../../components/SourceAudioVisualizer";
import { LiveMulticamSwitcher } from "../../components/LiveMulticamSwitcher";
import type { TimelinePlaybackSnapshot as DeckSnapshot, ActiveAudioClip } from "../../engine/playbackEngine";

export interface PreviewSectionProps {
  selectedMedia: MediaAsset | null;
  getMediaUrl: (path: string) => string;
  sourcePreviewError: string | null;
  setSourcePreviewError: (error: string | null) => void;
  preparedPreviews: Record<number, string>;
  preparingPreviews: Set<number>;
  previewPreparationErrors: Record<number, string>;
  mediaServerPort: number;
  deckSnapshot: DeckSnapshot | null;
  deckARef: RefObject<HTMLVideoElement | null>;
  deckBRef: RefObject<HTMLVideoElement | null>;
  programPreviewError: string | null;
  setProgramPreviewError: (error: string | null) => void;
  showTrackingBox: boolean;
  audioPlayersRef: MutableRefObject<Map<string, HTMLAudioElement>>;
  programPlaying: boolean;
  toggleProgramPlayback: () => void;
  stepProgramFrame: (delta: number) => void;
  shuttleSpeed: number;
  playhead: number;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  multicamMode: boolean;
  setMulticamMode: (mode: boolean) => void;
  multicamActiveAngle: number;
  setMulticamActiveAngle: (angle: number) => void;
  switchMulticamAngle: (angle: number) => Promise<void>;
  syncMulticamWaveforms: () => Promise<void>;
}

export function PreviewSection({
  selectedMedia,
  getMediaUrl,
  sourcePreviewError,
  setSourcePreviewError,
  preparedPreviews,
  preparingPreviews,
  previewPreparationErrors,
  mediaServerPort,
  deckSnapshot,
  deckARef,
  deckBRef,
  programPreviewError,
  setProgramPreviewError,
  showTrackingBox,
  audioPlayersRef,
  programPlaying,
  toggleProgramPlayback,
  stepProgramFrame,
  shuttleSpeed,
  playhead,
  project,
  media,
  multicamMode,
  setMulticamMode,
  multicamActiveAngle,
  setMulticamActiveAngle,
  switchMulticamAngle,
  syncMulticamWaveforms,
}: PreviewSectionProps) {
  const renderTitleOverlay = (titleProps: TitleProperties | undefined) => {
    if (!titleProps) return null;
    return (
      <div
        className={`title-overlay-box style-${titleProps.style || "lower_third"}`}
        style={{
          fontFamily: titleProps.font_family || "Inter",
          color: titleProps.color || "#ffffff",
          backgroundColor: titleProps.bg_color || "rgba(15, 23, 42, 0.75)",
        }}
      >
        <div className="title-overlay-accent" />
        <div className="title-overlay-text">
          <h3 style={{ fontSize: `${Math.max(14, (titleProps.font_size || 42) * 0.45)}px` }}>
            {titleProps.text}
          </h3>
          {titleProps.subtitle && <p>{titleProps.subtitle}</p>}
        </div>
      </div>
    );
  };

  const renderProgramMonitor = () => {
    const isPreparing = deckSnapshot?.deckA.mediaId ? preparingPreviews.has(deckSnapshot.deckA.mediaId) : false;
    const currentError = programPreviewError;

    return (
      <div className="program-stage">
        {deckSnapshot && (deckSnapshot.hasVideoMedia || deckSnapshot.inTransition) && (
          <div className={`program-deck-indicator ${deckSnapshot.inTransition ? "transitioning" : ""}`}>
            <span className="deck-dot" />
            <span>
              {deckSnapshot.inTransition
                ? `TRANSITION: ${(deckSnapshot.transitionKind ?? "crossfade").toUpperCase().replace("_", " ")}`
                : "DECK A/B LIVE"}
            </span>
          </div>
        )}

        <div className="program-deck-canvas">
          {/* Deck A Video Player */}
          <video
            ref={deckARef}
            className={`program-deck deck-a ${deckSnapshot?.deckA.visible ? "" : "deck-hidden"}`}
            style={deckSnapshot?.deckA.style}
            muted={!deckSnapshot?.deckA.visible}
            playsInline
            preload="auto"
            crossOrigin="anonymous"
            onCanPlay={() => setProgramPreviewError(null)}
          />
          {deckSnapshot?.deckA.vignette && deckSnapshot.deckA.visible && (
            <div
              className="deck-vignette-overlay deck-a-vignette"
              style={{
                position: "absolute",
                inset: 0,
                pointerEvents: "none",
                zIndex: 2,
                background: `radial-gradient(ellipse at center, transparent ${(deckSnapshot.deckA.vignette.radius ?? 0.8) * 40}%, rgba(0,0,0,${deckSnapshot.deckA.vignette.intensity ?? 0.5}) ${(deckSnapshot.deckA.vignette.radius ?? 0.8) * 40 + (deckSnapshot.deckA.vignette.softness ?? 0.5) * 50}%)`,
              }}
            />
          )}

          {/* Deck B Video Player */}
          <video
            ref={deckBRef}
            className={`program-deck deck-b ${deckSnapshot?.deckB.visible ? "" : "deck-hidden"}`}
            style={deckSnapshot?.deckB.style}
            muted={!deckSnapshot?.deckB.visible}
            playsInline
            preload="auto"
            crossOrigin="anonymous"
            onCanPlay={() => setProgramPreviewError(null)}
          />
          {deckSnapshot?.deckB.vignette && deckSnapshot.deckB.visible && (
            <div
              className="deck-vignette-overlay deck-b-vignette"
              style={{
                position: "absolute",
                inset: 0,
                pointerEvents: "none",
                zIndex: 2,
                background: `radial-gradient(ellipse at center, transparent ${(deckSnapshot.deckB.vignette.radius ?? 0.8) * 40}%, rgba(0,0,0,${deckSnapshot.deckB.vignette.intensity ?? 0.5}) ${(deckSnapshot.deckB.vignette.radius ?? 0.8) * 40 + (deckSnapshot.deckB.vignette.softness ?? 0.5) * 50}%)`,
              }}
            />
          )}

          {/* Title Overlay */}
          {renderTitleOverlay(deckSnapshot?.activeTitleClip?.title)}

          {/* On-Screen Subtitle / Closed Caption Overlay */}
          {deckSnapshot?.activeSubtitle && (
            <div className="program-subtitle-overlay" aria-live="polite">
              <div className="program-subtitle-text">{deckSnapshot.activeSubtitle.text}</div>
            </div>
          )}

          {/* Dedicated Audio Track Players for Multi-Track Audio */}
          {deckSnapshot?.activeAudioClips
            .filter((c: ActiveAudioClip) => !c.track.muted)
            .map((audioItem: ActiveAudioClip) => {
              const audioKey = `audio-track-${audioItem.track.id}-${audioItem.clip.media_id}`;
              return (
                <audio
                  key={audioKey}
                  src={audioItem.src}
                  autoPlay={programPlaying}
                  preload="auto"
                  crossOrigin="anonymous"
                  ref={(el) => {
                    if (el) {
                      audioPlayersRef.current.set(audioKey, el);
                      audioClockMaster.connectMediaElement(audioItem.track.id, el);
                      el.volume = Math.max(0, Math.min(1, 10 ** (audioItem.gainDb / 20)));
                      if (Math.abs(el.currentTime - audioItem.currentTime) > 0.25) {
                        try {
                          el.currentTime = audioItem.currentTime;
                        } catch {}
                      }
                      if (programPlaying && el.paused) void el.play().catch(() => {});
                      else if (!programPlaying && !el.paused) el.pause();
                    } else {
                      audioPlayersRef.current.delete(audioKey);
                    }
                  }}
                />
              );
            })}

          {/* Motion Tracking Crosshair Box Overlay (Phase 7.3) */}
          {showTrackingBox && (
            <div className="tracking-box-overlay" title="Motion Tracking Target Box">
              <div className="tracking-box-crosshair" />
            </div>
          )}

          {isPreparing && (
            <div className="preview-preparing-banner" role="status">
              <span className="preview-preparing-dot" />
              <span>Generating preview proxy…</span>
            </div>
          )}

          {currentError && !isPreparing && (
            <div className="preview-error" role="alert">
              <strong>Preview notice</strong>
              <span>{currentError}</span>
            </div>
          )}
        </div>
      </div>
    );
  };

  const renderMulticamMonitor = () => {
    const videoAssets = media.filter((m) => m.kind === "video" || m.kind === "audio");
    const activeMultiCam: MultiCamClip = project?.sequence.multicam_clips?.[0] || {
      id: 1,
      name: "Multicam Sequence",
      angles: [
        { id: 1, name: "Cam A (Wide Main)", media_id: videoAssets[0]?.id ?? 1, sync_offset_ticks: 0 },
        { id: 2, name: "Cam B (Close-up)", media_id: videoAssets[1]?.id ?? 2, sync_offset_ticks: 0 },
        { id: 3, name: "Cam C (Medium)", media_id: videoAssets[2]?.id ?? 3, sync_offset_ticks: 0 },
        { id: 4, name: "Cam D (Detail)", media_id: videoAssets[3]?.id ?? 4, sync_offset_ticks: 0 },
      ],
      active_angle_id: multicamActiveAngle,
    };

    return (
      <div className="multicam-stage-container">
        <LiveMulticamSwitcher
          multicamClip={activeMultiCam}
          isPlaying={programPlaying}
          onCutAngle={(angleId) => {
            void switchMulticamAngle(angleId);
          }}
          onSelectPreviewAngle={(angleId) => {
            setMulticamActiveAngle(angleId);
          }}
        />
        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            padding: "8px 12px",
            background: "#110910",
            borderTop: "1px solid #221220",
          }}
        >
          <InteriorButton
            type="button"
            className="multicam-sync-btn"
            onClick={() => void syncMulticamWaveforms()}
            title="Auto-synchronize multi-angle media via audio waveform cross-correlation"
          >
            <span>⚡ Sync Waveforms</span>
          </InteriorButton>
        </div>
      </div>
    );
  };

  const renderPreview = (item: MediaAsset | null) => {
    if (!item) {
      return (
        <div className="monitor-stage">
          <strong>Source monitor</strong>
          <small>Select imported media to preview it here</small>
        </div>
      );
    }
    const isPreparing = item.kind === "video" && preparingPreviews.has(item.id) && !preparedPreviews[item.id];
    const sourcePath = item.kind === "video" && preparedPreviews[item.id] ? preparedPreviews[item.id] : item.path;
    const source = getMediaUrl(sourcePath);
    const currentError = sourcePreviewError;
    const setCurrentError = setSourcePreviewError;
    const visualStyle: CSSProperties = { width: "100%", height: "100%", objectFit: "contain" };
    const isAudioFile = item.kind === "audio" || /\.(mp3|wav|m4a|flac|aac|ogg|wma)$/i.test(item.path || item.name);
    if (isAudioFile) {
      return (
        <div className="preview-stage audio-preview" style={{ width: "100%", height: "100%", background: "#050505" }}>
          <SourceAudioVisualizer item={item} src={source} />
        </div>
      );
    }
    if (item.kind === "video")
      return (
        <div className="preview-stage">
          <video
            key={`source-${item.id}-${sourcePath}`}
            src={mediaServerPort > 0 ? source : undefined}
            style={visualStyle}
            controls
            playsInline
            preload="auto"
            crossOrigin="anonymous"
            onCanPlay={() => setCurrentError(null)}
            onError={() => {
              if (previewPreparationErrors[item.id]) {
                setCurrentError(`Preview unavailable: ${previewPreparationErrors[item.id]}`);
              } else if (isPreparing) {
                // Proxy preparation is in flight, keep quiet
              } else {
                setCurrentError(
                  "This video could not be loaded. Check that the file exists and uses a supported codec."
                );
              }
            }}
          />
          {isPreparing && (
            <div className="preview-preparing-banner" role="status">
              <span className="preview-preparing-dot" />
              <span>Generating preview proxy…</span>
            </div>
          )}
          {currentError && !isPreparing && (
            <div className="preview-error" role="alert">
              <strong>Preview notice</strong>
              <span>{currentError}</span>
            </div>
          )}
        </div>
      );
    if (item.kind === "image")
      return (
        <div className="preview-stage">
          <img src={source} alt={item.name} style={visualStyle} />
        </div>
      );
    return null;
  };

  return (
    <div className="monitors">
      <section className="monitor">
        <div className="panel-tabs">
          <span className="selected">Source Monitor</span>
        </div>
        <div className="monitor-name">{selectedMedia?.name ?? "No source selected"}</div>
        {renderPreview(selectedMedia)}
      </section>
      <section className="monitor">
        <div className="panel-tabs">
          <span
            className={!multicamMode ? "selected" : ""}
            onClick={() => setMulticamMode(false)}
            style={{ cursor: "pointer" }}
          >
            Program Timeline
          </span>
          <span
            className={multicamMode ? "selected" : ""}
            onClick={() => setMulticamMode(true)}
            style={{ cursor: "pointer" }}
          >
            🎥 Multi-Cam Quad (4 Angles)
          </span>
        </div>
        <div className="monitor-name">
          {multicamMode
            ? `Multi-Cam Quad · Live Angle ${multicamActiveAngle}`
            : project?.sequence.name ?? "Sequence 1"}
        </div>
        {multicamMode ? renderMulticamMonitor() : renderProgramMonitor()}
        <div className="transport">
          <InteriorButton
            aria-label="Previous frame"
            data-tooltip="Previous Frame (Left Arrow)"
            data-tooltip-pos="top"
            onClick={() => stepProgramFrame(-1)}
          >
            ◀
          </InteriorButton>
          <InteriorButton
            className={programPlaying ? "active-toggle" : ""}
            aria-label={programPlaying ? "Pause playback" : "Play clip"}
            data-tooltip={programPlaying ? "Pause Playback (Space / K)" : "Play Timeline (Space / L)"}
            data-tooltip-pos="top"
            onClick={() => toggleProgramPlayback()}
          >
            {programPlaying ? "Ⅱ" : "▶"}
          </InteriorButton>
          <InteriorButton
            aria-label="Next frame"
            data-tooltip="Next Frame (Right Arrow)"
            data-tooltip-pos="top"
            onClick={() => stepProgramFrame(1)}
          >
            ▶|
          </InteriorButton>
          {shuttleSpeed !== 1 && programPlaying && (
            <span className="transport-shuttle" title="Shuttle speed">
              {shuttleSpeed}x
            </span>
          )}
          <code>{formatTime(playhead)}</code>
        </div>
      </section>
    </div>
  );
}
