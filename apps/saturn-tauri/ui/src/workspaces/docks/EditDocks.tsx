// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { MediaPanel, type MediaPanelProps } from "../../features/media-pool";
import { TrimMonitorPanel } from "../../features/timeline";
import { InspectorPanel, type InspectorPanelProps } from "../../features/inspector";
import { DockCard, type DockTab } from "../../features/dock/DockCard";
import { MarkersPanel, type MarkersPanelProps } from "../../features/markers";
import { CaptionsPanel, type CaptionsPanelProps } from "../../features/captions";
import { NodeGraphPanel, type NodeGraphPanelProps } from "../../features/nodegraph";
import { GraphicsPanel, type GraphicOperations } from "../../features/graphics";
import { AgentCard, ReviewCard, useAgentSession } from "../../features/agent";
import { TaskSheetPanel } from "../../features/tasksheet";
import { ScriptSyncPanel } from "../../features/captions";
import { AgentTelemetryDrawer } from "../../features/inspector";
import type { TrimSession } from "../../app/timelineTrim";
import type { MediaAsset, ProjectDocument } from "../../app/projectTypes";

export interface TrimMonitorEntryProps {
  session: TrimSession | null;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  getMediaUrl: (path: string) => string;
  onCommit?: () => void;
  onCancel?: () => void;
}

export interface EditDockProps {
  mediaPanelProps: MediaPanelProps;
  trimMonitorProps: TrimMonitorEntryProps;
  inspectorPanelProps: InspectorPanelProps;
  markersPanelProps: MarkersPanelProps;
  captionsPanelProps: CaptionsPanelProps;
  nodeGraphPanelProps: NodeGraphPanelProps;
  graphicsPanelProps: {
    ops: GraphicOperations;
    playhead: number;
    frame: { width: number; height: number };
  };
}

/**
 * Edit-mode dock tabs for the WorkspaceShell (plan v4 G3): the library dock
 * (Media pool · Trim Monitor) sits left of the program monitor; the inspector
 * stack (Inspector · Markers · Captions · Graphics) sits right of it. The
 * monitor, toolbox, and timeline outside these cards are shell-owned and
 * never remount.
 */
export function EditDockLeft({
  mediaPanelProps,
  trimMonitorProps,
}: Pick<EditDockProps, "mediaPanelProps" | "trimMonitorProps">) {
  const tabs: DockTab[] = [
    { id: "media", title: "Media pool", render: () => <MediaPanel {...mediaPanelProps} /> },
  ];
  if (trimMonitorProps.session) {
    tabs.push({
      id: "trim-monitor",
      title: "Trim Monitor",
      render: () => (
        <TrimMonitorPanel
          session={trimMonitorProps.session!}
          project={trimMonitorProps.project}
          media={trimMonitorProps.media}
          getMediaUrl={trimMonitorProps.getMediaUrl}
          onCommit={trimMonitorProps.onCommit}
          onCancel={trimMonitorProps.onCancel}
        />
      ),
    });
  }
  return <DockCard className="shell-dock shell-dock-left" tabs={tabs} ariaLabel="Library dock" />;
}

export function EditDockRight({
  inspectorPanelProps,
  markersPanelProps,
  captionsPanelProps,
  graphicsPanelProps,
  nodeGraphPanelProps,
}: Pick<
  EditDockProps,
  | "inspectorPanelProps"
  | "markersPanelProps"
  | "captionsPanelProps"
  | "graphicsPanelProps"
  | "nodeGraphPanelProps"
>) {
  const agent = useAgentSession({
    seek: markersPanelProps.onSeek,
    playhead: graphicsPanelProps.playhead,
  });

  const tabs: DockTab[] = [
    { id: "inspector", title: "Inspector", render: () => <InspectorPanel {...inspectorPanelProps} /> },
    {
      id: "sheet",
      title: "Sheet",
      render: () => (
        <TaskSheetPanel
          sheet={agent.session.taskSheet}
          onUpdateSheet={agent.updateTaskSheet}
          onSubmitSheet={agent.submitTaskSheet}
          onLoadTemplate={agent.loadTemplate}
          activeStatus={agent.session.taskSheet?.status}
        />
      ),
    },
    {
      id: "agent",
      title: "Agent",
      render: () => (
        <AgentCard
          session={agent.session}
          onStop={agent.stopSession}
          onResume={agent.resumeSession}
          onUpdateBrief={agent.updateBrief}
          onAllowPermission={agent.allowPermission}
          onDenyPermission={agent.denyPermission}
          onJumpToTick={agent.jumpToTick}
          onAnswerQuestion={agent.answerQuestion}
        />
      ),
    },
    {
      id: "review",
      title: "Review",
      render: () => (
        <ReviewCard
          session={agent.session}
          playhead={agent.playhead}
          selectedRoundNumber={agent.selectedRoundNumber}
          onSelectRound={agent.setSelectedRoundNumber}
          selectedNoteId={agent.selectedNoteId}
          onSelectNote={agent.setSelectedNoteId}
          onAddReviewNote={agent.addReviewNote}
          onResolveReviewNote={agent.resolveReviewNote}
          onOpenReviewRound={agent.openReviewRound}
          onCloseReviewRound={agent.closeReviewRound}
          onAcceptChange={agent.acceptChange}
          onRejectChange={agent.rejectChange}
          onFixChange={agent.fixChange}
          onAcceptAllBranch={agent.acceptAllBranch}
          onRejectAllBranch={agent.rejectAllBranch}
          onJumpToTick={agent.jumpToTick}
        />
      ),
    },
    {
      id: "script",
      title: "Script",
      render: () => (
        <ScriptSyncPanel
          playhead={graphicsPanelProps.playhead}
          onSeek={(tick) => agent.jumpToTick(tick)}
        />
      ),
    },
    {
      id: "telemetry",
      title: "Telemetry",
      render: () => <AgentTelemetryDrawer />,
    },
    { id: "markers", title: "Markers", render: () => <MarkersPanel {...markersPanelProps} /> },
    { id: "captions", title: "Captions", render: () => <CaptionsPanel {...captionsPanelProps} /> },
    {
      id: "graphics",
      title: "Essential Graphics",
      render: () => (
        <GraphicsPanel ops={graphicsPanelProps.ops} playhead={graphicsPanelProps.playhead} frame={graphicsPanelProps.frame} />
      ),
    },
    {
      id: "nodegraph",
      title: "Nodes",
      render: () => <NodeGraphPanel {...nodeGraphPanelProps} />,
    },
  ];
  return <DockCard className="shell-dock shell-dock-right" tabs={tabs} ariaLabel="Inspector dock" />;
}
