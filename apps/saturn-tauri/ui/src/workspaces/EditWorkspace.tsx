import { MediaPanel, type MediaPanelProps } from "../features/media-pool";
import { PreviewSection, type PreviewSectionProps } from "../features/preview";
import {
  TimelineSection,
  TimelineToolRail,
  TrimMonitorPanel,
  type TimelineSectionProps,
} from "../features/timeline";
import { InspectorPanel, type InspectorPanelProps } from "../features/inspector";
import { DockCard, type DockTab } from "../features/dock/DockCard";
import { MarkersPanel, type MarkersPanelProps } from "../features/markers";
import { CaptionsPanel, type CaptionsPanelProps } from "../features/captions";
import { GraphicsPanel, type GraphicOperations } from "../features/graphics";
import type { TrimSession } from "../app/timelineTrim";
import type { MediaAsset, ProjectDocument } from "../app/projectTypes";

export interface TrimMonitorEntryProps {
  session: TrimSession | null;
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  getMediaUrl: (path: string) => string;
  onCommit?: () => void;
  onCancel?: () => void;
}

export interface EditWorkspaceProps {
  mediaPanelProps: MediaPanelProps;
  previewSectionProps: PreviewSectionProps;
  timelineSectionProps: TimelineSectionProps;
  inspectorPanelProps: InspectorPanelProps;
  trimMonitorProps: TrimMonitorEntryProps;
  markersPanelProps: MarkersPanelProps;
  captionsPanelProps: CaptionsPanelProps;
  graphicsPanelProps: {
    ops: GraphicOperations;
    playhead: number;
    frame: { width: number; height: number };
  };
}

/**
 * Phase 4: the media pool and inspector are engine-managed floating panels;
 * preview and timeline stay docked in the work area and never float.
 */
export function EditWorkspace({
  mediaPanelProps,
  previewSectionProps,
  timelineSectionProps,
  inspectorPanelProps,
  trimMonitorProps,
  markersPanelProps,
  captionsPanelProps,
  graphicsPanelProps,
}: EditWorkspaceProps) {
  // Plan v4 §B: the Edit page keeps two stationary dock cards — library
  // (media pool · trim monitor) on the left of the program monitor and the
  // inspector stack (Inspector · Markers · Captions · Graphics) on the right.
  const libraryTabs: DockTab[] = [
    { id: "media", title: "Media pool", render: () => <MediaPanel {...mediaPanelProps} /> },
  ];
  if (trimMonitorProps.session) {
    libraryTabs.push({
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
  const inspectorTabs: DockTab[] = [
    { id: "inspector", title: "Inspector", render: () => <InspectorPanel {...inspectorPanelProps} /> },
    { id: "markers", title: "Markers", render: () => <MarkersPanel {...markersPanelProps} /> },
    { id: "captions", title: "Captions", render: () => <CaptionsPanel {...captionsPanelProps} /> },
    {
      id: "graphics",
      title: "Essential Graphics",
      render: () => <GraphicsPanel ops={graphicsPanelProps.ops} playhead={graphicsPanelProps.playhead} frame={graphicsPanelProps.frame} />,
    },
  ];

  return (
    <main className="editor">
      <DockCard
        className="editor-dock editor-dock-library"
        tabs={libraryTabs}
        ariaLabel="Library dock"
      />
      <section className="work-area">
        <PreviewSection {...previewSectionProps} />
      </section>
      <DockCard
        className="editor-dock editor-dock-inspector"
        tabs={inspectorTabs}
        ariaLabel="Inspector dock"
      />
      {/* Mock's NLE row: the toolbox strip travels with the timeline in the
          full-width bottom row and never moves (persistent skeleton). */}
      <div className="editor-nle-row">
        <aside className="editor-toolbox" aria-label="Editing toolbox">
          <TimelineToolRail
            timelineTool={timelineSectionProps.timelineTool}
            setTimelineTool={timelineSectionProps.setTimelineTool}
          />
        </aside>
        <div className="editor-timeline-dock">
          <TimelineSection {...timelineSectionProps} />
        </div>
      </div>
    </main>
  );
}

export default EditWorkspace;
