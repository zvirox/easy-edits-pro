import { MediaPanel, type MediaPanelProps } from "../features/media-pool";
import { PreviewSection, type PreviewSectionProps } from "../features/preview";
import { TimelineSection, type TimelineSectionProps } from "../features/timeline";
import { InspectorPanel, type InspectorPanelProps } from "../features/inspector";

export interface EditWorkspaceProps {
  mediaPanelProps: MediaPanelProps;
  previewSectionProps: PreviewSectionProps;
  timelineSectionProps: TimelineSectionProps;
  inspectorPanelProps: InspectorPanelProps;
}

export function EditWorkspace({
  mediaPanelProps,
  previewSectionProps,
  timelineSectionProps,
  inspectorPanelProps,
}: EditWorkspaceProps) {
  return (
    <main className="editor">
      <MediaPanel {...mediaPanelProps} />
      <section className="work-area">
        <PreviewSection {...previewSectionProps} />
        <TimelineSection {...timelineSectionProps} />
      </section>
      <InspectorPanel {...inspectorPanelProps} />
    </main>
  );
}

export default EditWorkspace;
