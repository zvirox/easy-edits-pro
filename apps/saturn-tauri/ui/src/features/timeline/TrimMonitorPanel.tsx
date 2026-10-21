import { useEffect, useRef } from "react";
import type { Clip, MediaAsset, ProjectDocument } from "../../app/projectTypes";
import {
  planTrim,
  trimMonitorLabel,
  type TrimSession,
} from "../../app/timelineTrim";
import { formatTimecodeFrames } from "../../utils/formatters";

export interface TrimMonitorPanelProps {
  session: TrimSession;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  getMediaUrl: (path: string) => string;
  onCommit?: () => void;
  onCancel?: () => void;
}

interface FrameSpec {
  clip: Clip | null;
  asset: MediaAsset | null;
  ticks: number;
  role: "outgoing" | "incoming";
}

function RawFrame({
  spec,
  getMediaUrl,
  frameRate,
}: {
  spec: FrameSpec;
  getMediaUrl: (path: string) => string;
  frameRate: { numerator: number; denominator: number };
}) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const seconds = spec.ticks / 254_016_000_000;

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    const seek = () => {
      try {
        video.currentTime = Math.max(0, seconds);
      } catch {
        // Seeking before metadata arrives is harmless; loadedmetadata retries.
      }
    };
    if (video.readyState >= 1) seek();
    else video.addEventListener("loadedmetadata", seek, { once: true });
    return () => video.removeEventListener("loadedmetadata", seek);
  }, [seconds]);

  if (!spec.asset) {
    return (
      <figure className="trim-monitor-frame trim-monitor-frame-empty">
        <span className="trim-monitor-empty-label">No clip on this side</span>
        <figcaption>
          <span>{spec.role === "outgoing" ? "Outgoing" : "Incoming"}</span>
          <code>—</code>
        </figcaption>
      </figure>
    );
  }

  const frame =
    spec.asset.kind === "image" ? (
      <img src={getMediaUrl(spec.asset.path)} alt="" draggable={false} />
    ) : (
      <video
        ref={videoRef}
        src={getMediaUrl(spec.asset.path)}
        muted
        playsInline
        preload="metadata"
        aria-label={`${spec.asset.name} ${spec.role} frame`}
      />
    );

  return (
    <figure className="trim-monitor-frame">
      <div className="trim-monitor-raw">{frame}</div>
      <figcaption>
        <span>{spec.role === "outgoing" ? "Outgoing" : "Incoming"}</span>
        <strong title={spec.asset.name}>{spec.asset.name}</strong>
        <code>{formatTimecodeFrames(spec.ticks, frameRate)}</code>
      </figcaption>
    </figure>
  );
}

/**
 * Phase 2 Trim Monitor: two raw (never glass) frames around the edit point.
 * It reads the same pure trim plan the commit uses, so the displayed source
 * timecodes always match what releasing the pointer will produce.
 */
export function TrimMonitorPanel({
  session,
  project,
  media,
  getMediaUrl,
  onCommit,
  onCancel,
}: TrimMonitorPanelProps) {
  const track = project?.sequence.tracks.find((item) => item.id === session.trackId);
  const clip = track?.clips[session.clipIndex] ?? null;
  const previous = track?.clips[session.clipIndex - 1] ?? null;
  const next = track?.clips[session.clipIndex + 1] ?? null;

  if (!project || !track || !clip) {
    return <p className="trim-monitor-missing">That clip is no longer on the timeline.</p>;
  }

  const plan = planTrim({
    trackId: session.trackId,
    tool: session.tool,
    edge: session.edge,
    deltaTicks: session.deltaTicks,
    clipIndex: session.clipIndex,
    clip,
    previous,
    next,
    mediaDuration: media.find((item) => item.id === clip.media_id)?.duration ?? null,
  });
  const deltaTicks = plan.ok ? plan.plan.deltaTicks : session.deltaTicks;
  const preview = plan.ok ? plan.plan.preview : clip;

  const assetFor = (source: Clip | null): MediaAsset | null =>
    source ? media.find((item) => item.id === source.media_id) ?? null : null;

  const isSameClipPair = session.tool === "slip" || session.tool === "rateStretch";
  const outgoingClip = isSameClipPair
    ? clip
    : session.edge === "start"
      ? previous
      : clip;
  const incomingClip = isSameClipPair
    ? clip
    : session.edge === "start"
      ? clip
      : next;

  let outgoingTicks = 0;
  let incomingTicks = 0;
  if (outgoingClip) {
    if (isSameClipPair) {
      outgoingTicks = outgoingClip.source_in + outgoingClip.duration;
    } else if (session.tool === "rolling" && session.edge === "start") {
      outgoingTicks = outgoingClip.source_in + outgoingClip.duration + deltaTicks;
    } else if (session.tool === "slide") {
      outgoingTicks = outgoingClip.source_in + outgoingClip.duration + deltaTicks;
    } else if (outgoingClip === clip) {
      outgoingTicks = preview.source_in + preview.duration;
    } else {
      outgoingTicks = outgoingClip.source_in + outgoingClip.duration;
    }
  }
  if (incomingClip) {
    if (isSameClipPair) {
      incomingTicks = preview.source_in;
    } else if (session.tool === "rolling" && session.edge === "end") {
      incomingTicks = incomingClip.source_in + deltaTicks;
    } else if (session.tool === "slide") {
      incomingTicks = incomingClip.source_in + deltaTicks;
    } else if (incomingClip === clip) {
      incomingTicks = preview.source_in;
    } else {
      incomingTicks = incomingClip.source_in;
    }
  }

  const frames: FrameSpec[] = [
    {
      clip: outgoingClip,
      asset: assetFor(outgoingClip),
      ticks: outgoingTicks,
      role: "outgoing",
    },
    {
      clip: incomingClip,
      asset: assetFor(incomingClip),
      ticks: incomingTicks,
      role: "incoming",
    },
  ];

  const deltaSeconds = Math.abs(deltaTicks) / 254_016_000_000;
  const label = plan.ok ? plan.plan.notice : trimMonitorLabel(session.tool, session.edge);
  const cutTicks =
    session.edge === "end" && !isSameClipPair
      ? preview.timeline_start + preview.duration
      : preview.timeline_start;

  return (
    <div className="trim-monitor">
      <header className="trim-monitor-header">
        <div>
          <span className="trim-monitor-eyebrow">{trimMonitorLabel(session.tool, session.edge)}</span>
          <strong className="trim-monitor-delta">
            {deltaTicks < 0 ? "−" : "+"}
            {deltaSeconds.toFixed(2)}s
          </strong>
        </div>
        <code className="trim-monitor-cut">
          Cut {formatTimecodeFrames(cutTicks, project.settings.frame_rate)}
        </code>
      </header>
      <div className="trim-monitor-frames">
        {frames.map((frame) => (
          <RawFrame
            key={frame.role}
            spec={frame}
            getMediaUrl={getMediaUrl}
            frameRate={project.settings.frame_rate}
          />
        ))}
      </div>
      <p className="trim-monitor-note">
        {plan.ok ? label : plan.reason}
      </p>
      {session.dynamic && (
        <div className="trim-monitor-actions">
          <span>J/L shuttle · K stop · Enter commit · Esc cancel</span>
          <button type="button" onClick={onCancel}>Cancel</button>
          <button type="button" className="primary" onClick={onCommit}>Commit trim</button>
        </div>
      )}
    </div>
  );
}
