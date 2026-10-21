import { MediaPanel, type MediaPanelProps } from "../features/media-pool";
import { PreviewSection, type PreviewSectionProps } from "../features/preview";
import { TimelineSection, type TimelineSectionProps } from "../features/timeline";
import { InspectorPanel, type InspectorPanelProps } from "../features/inspector";
import { PanelHost, type PanelEntry } from "../features/panels";

export interface EditWorkspaceProps {
  mediaPanelProps: MediaPanelProps;
  previewSectionProps: PreviewSectionProps;
  timelineSectionProps: TimelineSectionProps;
  inspectorPanelProps: InspectorPanelProps;
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
}: EditWorkspaceProps) {
  const entries: PanelEntry[] = [
    { id: "media", title: "Media pool", render: () => <MediaPanel {...mediaPanelProps} /> },
    { id: "inspector", title: "Inspector", render: () => <InspectorPanel {...inspectorPanelProps} /> },
  ];

  return (
    <main className="editor is-floating-panels">
      <section className="work-area">
        <PreviewSection {...previewSectionProps} />
        <TimelineSection {...timelineSectionProps} />
      </section>
      <PanelHost workspace="edit" entries={entries} />
    </main>
  );
}

export default EditWorkspace;
