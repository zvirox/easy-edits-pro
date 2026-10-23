import { DockCard, type DockTab } from "../../features/dock/DockCard";
import { MediaPanel, type MediaPanelProps } from "../../features/media-pool";

/**
 * The stub left dock shared by the docked special modes (effects, deliver,
 * fusion, keyframes): one Media pool tab beside the persistent program
 * monitor (plan v4 G5).
 */
export function MediaPoolDockLeft({
  mediaPanelProps,
  ariaLabel = "Media dock",
}: {
  mediaPanelProps: MediaPanelProps;
  ariaLabel?: string;
}) {
  const tabs: DockTab[] = [
    { id: "media", title: "Media pool", render: () => <MediaPanel {...mediaPanelProps} /> },
  ];
  return <DockCard className="shell-dock shell-dock-left" tabs={tabs} ariaLabel={ariaLabel} />;
}
