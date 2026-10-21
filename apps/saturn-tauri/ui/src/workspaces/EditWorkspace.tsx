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
}: EditWorkspaceProps) {
  const entries: PanelEntry[] = [
    { id: "media", title: "Media pool", render: () => <MediaPanel {...mediaPanelProps} /> },
    { id: "inspector", title: "Inspector", render: () => <InspectorPanel {...inspectorPanelProps} /> },
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
        <PanelHost workspace="edit" entries={entries} />
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
