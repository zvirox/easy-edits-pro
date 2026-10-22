import { MediaPanel, type MediaPanelProps } from "../features/media-pool";
import { PreviewSection, type PreviewSectionProps } from "../features/preview";
import {
  TimelineSection,
  TimelineToolRail,
  TrimMonitorPanel,
  type TimelineSectionProps,
} from "../features/timeline";
import { InspectorPanel, type InspectorPanelProps } from "../features/inspector";
import { PanelHost, type PanelEntry } from "../features/panels";
import { EDIT_CONSTRAINTS } from "../features/panels/layouts";
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
  const entries: PanelEntry[] = [
    { id: "media", title: "Media pool", render: () => <MediaPanel {...mediaPanelProps} /> },
    { id: "inspector", title: "Inspector", render: () => <InspectorPanel {...inspectorPanelProps} /> },
    {
      id: "markers",
      title: "Markers",
      constraints: EDIT_CONSTRAINTS.markers,
      render: () => <MarkersPanel {...markersPanelProps} />,
    },
    {
      id: "captions",
      title: "Captions",
      constraints: EDIT_CONSTRAINTS.captions,
      render: () => <CaptionsPanel {...captionsPanelProps} />,
    },
    {
      id: "graphics",
      title: "Essential Graphics",
      constraints: EDIT_CONSTRAINTS.graphics,
      render: () => <GraphicsPanel ops={graphicsPanelProps.ops} playhead={graphicsPanelProps.playhead} frame={graphicsPanelProps.frame} />,
    },
  ];
  if (trimMonitorProps.session) {
    entries.push({
      id: "trim-monitor",
      title: "Trim Monitor",
      constraints: EDIT_CONSTRAINTS["trim-monitor"],
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

  return (
    <main className="editor is-floating-panels">
      <div className="editor-panel-layer">
        <PanelHost
          workspace="edit"
          entries={entries}
          defaultHidden={["markers", "captions", "graphics"]}
        />
      </div>
      <aside className="editor-toolbox" aria-label="Editing toolbox">
        <TimelineToolRail
          timelineTool={timelineSectionProps.timelineTool}
          setTimelineTool={timelineSectionProps.setTimelineTool}
        />
      </aside>
      <section className="work-area">
        <PreviewSection {...previewSectionProps} />
      </section>
      <div className="editor-timeline-dock">
        <TimelineSection {...timelineSectionProps} />
      </div>
    </main>
  );
}

export default EditWorkspace;
