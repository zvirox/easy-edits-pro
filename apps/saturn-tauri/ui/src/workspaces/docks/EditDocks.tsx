import { MediaPanel, type MediaPanelProps } from "../../features/media-pool";
import { TrimMonitorPanel } from "../../features/timeline";
import { InspectorPanel, type InspectorPanelProps } from "../../features/inspector";
import { DockCard, type DockTab } from "../../features/dock/DockCard";
import { MarkersPanel, type MarkersPanelProps } from "../../features/markers";
import { CaptionsPanel, type CaptionsPanelProps } from "../../features/captions";
import { GraphicsPanel, type GraphicOperations } from "../../features/graphics";
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
}: Pick<
  EditDockProps,
  "inspectorPanelProps" | "markersPanelProps" | "captionsPanelProps" | "graphicsPanelProps"
>) {
  const tabs: DockTab[] = [
    { id: "inspector", title: "Inspector", render: () => <InspectorPanel {...inspectorPanelProps} /> },
    { id: "markers", title: "Markers", render: () => <MarkersPanel {...markersPanelProps} /> },
    { id: "captions", title: "Captions", render: () => <CaptionsPanel {...captionsPanelProps} /> },
    {
      id: "graphics",
      title: "Essential Graphics",
      render: () => (
        <GraphicsPanel ops={graphicsPanelProps.ops} playhead={graphicsPanelProps.playhead} frame={graphicsPanelProps.frame} />
      ),
    },
  ];
  return <DockCard className="shell-dock shell-dock-right" tabs={tabs} ariaLabel="Inspector dock" />;
}
