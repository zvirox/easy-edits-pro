import { InteriorButton } from "../../components/interior/interior-button";
import type { CSSProperties, RefObject, MutableRefObject } from "react";
import { useEffect, useRef, useState } from "react";
import { formatTime } from "../../utils/formatters";
import { TICKS_PER_SECOND } from "../../engine/playback/constants";
import { audioClockMaster, type MultiCamClip } from "../../engine/playbackEngine";
import type { MediaAsset, ProjectDocument, TitleProperties } from "../../app/projectTypes";
import { SourceAudioVisualizer } from "../../components/SourceAudioVisualizer";
import { LiveMulticamSwitcher } from "../../components/LiveMulticamSwitcher";
import { CanvasGraphicsLayer, type GraphicsEditing } from "../graphics/CanvasGraphicsLayer";
import type { TimelinePlaybackSnapshot as DeckSnapshot, ActiveAudioClip } from "../../engine/playbackEngine";

/** What the unified monitor's Source tab should currently show. */
export interface SourcePreviewFocus {
  asset: MediaAsset;
  /** Seconds into the source to land on (clip In point for timeline picks). */
  startSeconds: number;
  /** Changes whenever the source focus changes; drives video remount/seek. */
  key: string;
}

export type MonitorMode = "program" | "multicam" | "source";
export type ThreePointMode = "insert" | "overwrite";

export interface PreviewSectionProps {  selectedMedia: MediaAsset | null;
  sourcePreview: SourcePreviewFocus | null;
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
  /** Send a marked source range to the timeline as an insert or overwrite. */
  onThreePointEdit?: (
    mode: ThreePointMode,
    mediaId: number,
    sourceInTicks: number,
    durationTicks: number,
  ) => void | Promise<void>;
  /** On-canvas graphics editing (slice 4); absent = passive overlay only. */
  graphicsEditing?: GraphicsEditing;
}

export function PreviewSection({
  selectedMedia,
  sourcePreview,
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
  setMulticamMode,
  multicamActiveAngle,
  setMulticamActiveAngle,
  switchMulticamAngle,
  syncMulticamWaveforms,
  onThreePointEdit,
  graphicsEditing,
}: PreviewSectionProps) {
  const [monitorMode, setMonitorMode] = useState<MonitorMode>("program");
  const sourceVideoRef = useRef<HTMLVideoElement>(null);
  const [sourceTime, setSourceTime] = useState(0);
  const [sourceMarks, setSourceMarks] = useState<{
    inTicks: number;
    outTicks: number;
  } | null>(null);

  // Clicking a clip in the timeline or an item in the media pool bumps the
  // shared sourcePreview focus — jump the monitor to its Source tab.
  useEffect(() => {
    if (sourcePreview) setMonitorMode("source");
  }, [sourcePreview?.key]);

  useEffect(() => {
    setSourceMarks(null);
    setSourceTime((sourcePreview?.startSeconds ?? 0) * TICKS_PER_SECOND);
  }, [sourcePreview?.key]);

  const sourceTicks = () =>
    Math.round(
      (sourceVideoRef.current?.currentTime ?? sourcePreview?.startSeconds ?? 0) *
        TICKS_PER_SECOND,
    );

  const markSourceIn = () => {
    if (!sourcePreview) return;
    const at = sourceTicks();
    setSourceMarks((current) => ({
      inTicks: at,
      outTicks:
        current?.outTicks && current.outTicks > at
          ? current.outTicks
          : Math.max(at + 1, sourcePreview.asset.duration ?? TICKS_PER_SECOND * 5),
    }));
  };

  const markSourceOut = () => {
    if (!sourcePreview) return;
    const at = sourceTicks();
    setSourceMarks((current) => ({
      inTicks: current?.inTicks ?? 0,
      outTicks: at,
    }));
  };

  const applyThreePointEdit = (mode: ThreePointMode) => {
    if (!sourcePreview) {
      setSourcePreviewError("Open a source clip in the monitor first");
      return;
    }
    const duration = sourcePreview.asset.duration ?? TICKS_PER_SECOND * 5;
    const sourceIn = sourceMarks?.inTicks ?? 0;
    const sourceOut = sourceMarks?.outTicks ?? duration;
    if (sourceOut <= sourceIn) {
      setSourcePreviewError("The Out point must be later than the In point");
      return;
    }
    setSourcePreviewError(null);
    void onThreePointEdit?.(
      mode,
      sourcePreview.asset.id,
      sourceIn,
      sourceOut - sourceIn,
    );
  };

  // I/O mark the source while the Source tab is visible; comma/period send the
  // marked range to the playhead. Capture phase keeps the global timeline
  // shortcuts from stealing these keys.
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (monitorMode !== "source" || !sourcePreview) return;
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      const target = event.target as HTMLElement | null;
      if (target && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))) {
        return;
      }
      const key = event.key.toLowerCase();
      if (key === "i") {
        event.preventDefault();
        event.stopPropagation();
        markSourceIn();
      } else if (key === "o") {
        event.preventDefault();
        event.stopPropagation();
        markSourceOut();
      } else if (event.key === "," || event.key === ".") {
        event.preventDefault();
        event.stopPropagation();
        applyThreePointEdit(event.key === "," ? "insert" : "overwrite");
      }
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [monitorMode, sourcePreview, sourceMarks]);

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

          {/* Essential Graphics overlay (Phase 5): timeline graphics active
              at the playhead. Slice 4 adds canvas tools and drag/resize when
              the program is paused. The export path rasterizes them through
              saturn-text in a later slice. */}
          <CanvasGraphicsLayer
            graphics={project?.sequence.graphics ?? []}
            playhead={playhead}
            frame={project?.settings ?? { width: 1920, height: 1080 }}
            editing={graphicsEditing}
            programPlaying={programPlaying}
          />

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
              const audioKey = `audio-track-${audioItem.track.id}-${audioItem.clip.timeline_start}-${audioItem.clip.media_id}`;
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
                      el.volume = 1;
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

  const renderPreview = (item: MediaAsset | null, startSeconds = 0) => {
    if (!item) {
      return (
        <div className="monitor-stage">
          <strong>Source monitor</strong>
          <small>Select imported media or click a timeline clip to preview it here</small>
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
        <div className="preview-stage audio-preview" style={{ width: "100%", height: "100%", background: "var(--bg-canvas)" }}>
          <SourceAudioVisualizer item={item} src={source} />
        </div>
      );
    }
    if (item.kind === "video")
      return (
        <div className="preview-stage">
          <video
            key={`source-${item.id}-${sourcePath}-${startSeconds}`}
            ref={sourceVideoRef}
            src={mediaServerPort > 0 ? source : undefined}
            style={visualStyle}
            controls
            playsInline
            preload="auto"
            crossOrigin="anonymous"
            onTimeUpdate={(event) =>
              setSourceTime(Math.round(event.currentTarget.currentTime * TICKS_PER_SECOND))
            }
            onCanPlay={() => setCurrentError(null)}
            onLoadedMetadata={(event) => {
              // Timeline picks land on the clip's In point, like FilmCraft's source viewer.
              if (startSeconds > 0.01) {
                try {
                  event.currentTarget.currentTime = startSeconds;
                } catch {}
              }
            }}
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

  const sourceName = sourcePreview?.asset.name ?? selectedMedia?.name;

  return (
    <div className="monitors">
      <section className="monitor monitor-unified">
        <div className="panel-tabs">
          <span
            className={monitorMode === "program" ? "selected" : ""}
            onClick={() => {
              setMonitorMode("program");
              setMulticamMode(false);
            }}
            style={{ cursor: "pointer" }}
          >
            Program Timeline
          </span>
          <span
            className={monitorMode === "multicam" ? "selected" : ""}
            onClick={() => {
              setMonitorMode("multicam");
              setMulticamMode(true);
            }}
            style={{ cursor: "pointer" }}
          >
            🎥 Multi-Cam Quad (4 Angles)
          </span>
          <span
            className={monitorMode === "source" ? "selected" : ""}
            onClick={() => setMonitorMode("source")}
            style={{ cursor: "pointer" }}
          >
            Source{sourceName ? ` · ${sourceName}` : ""}
          </span>
        </div>
        <div className="monitor-name">
          {monitorMode === "source"
            ? sourcePreview?.asset.name ?? "No source selected"
            : monitorMode === "multicam"
              ? `Multi-Cam Quad · Live Angle ${multicamActiveAngle}`
              : project?.sequence.name ?? "Sequence 1"}
        </div>
        {/* Program/Multi-Cam stay mounted while the Source tab is shown so
            playback keeps running behind the tab switch. */}
        <div className="monitor-tab-panel" hidden={monitorMode === "source"}>
          {monitorMode === "multicam" ? renderMulticamMonitor() : renderProgramMonitor()}
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
        </div>
        <div className="monitor-tab-panel source-tab-panel" hidden={monitorMode !== "source"}>
          <div className="source-edit-bar">
            <InteriorButton
              aria-label="Mark source In"
              data-tooltip="Mark Source In (I)"
              data-tooltip-pos="top"
              disabled={!sourcePreview}
              onClick={markSourceIn}
            >
              Mark In
            </InteriorButton>
            <InteriorButton
              aria-label="Mark source Out"
              data-tooltip="Mark Source Out (O)"
              data-tooltip-pos="top"
              disabled={!sourcePreview}
              onClick={markSourceOut}
            >
              Mark Out
            </InteriorButton>
            <span className="source-edit-range">
              <strong>{formatTime(sourceMarks?.inTicks ?? 0)}</strong>
              <span>→</span>
              <strong>
                {formatTime(sourceMarks?.outTicks ?? sourcePreview?.asset.duration ?? 0)}
              </strong>
              <small>source {formatTime(sourceTime)}</small>
            </span>
            <InteriorButton
              aria-label="Insert source range"
              data-tooltip="Insert Edit at Playhead (,)"
              data-tooltip-pos="top"
              disabled={!sourcePreview}
              onClick={() => applyThreePointEdit("insert")}
            >
              Insert
            </InteriorButton>
            <InteriorButton
              aria-label="Overwrite source range"
              data-tooltip="Overwrite Edit at Playhead (.)"
              data-tooltip-pos="top"
              disabled={!sourcePreview}
              onClick={() => applyThreePointEdit("overwrite")}
            >
              Overwrite
            </InteriorButton>
          </div>
          {renderPreview(sourcePreview?.asset ?? null, sourcePreview?.startSeconds ?? 0)}
        </div>
      </section>
    </div>
  );
}
