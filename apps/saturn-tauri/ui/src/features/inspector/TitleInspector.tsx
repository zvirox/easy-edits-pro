import { InteriorButton } from "../../components/interior/interior-button";
import type { Dispatch, SetStateAction } from "react";
import { formatTime } from "../../utils/formatters";
import type { Clip, MediaAsset, TitleProperties } from "../../app/projectTypes";

export interface TitleInspectorProps {
  selectedTimelineClip: Clip | null;
  mediaForClip: MediaAsset | undefined;
  titleDraft: TitleProperties | null;
  setTitleDraft: Dispatch<SetStateAction<TitleProperties | null>>;
  saveTitleProperty: (props: Partial<TitleProperties>) => Promise<void>;
  removeSelectedClip: () => void;
}

export function TitleInspector({
  selectedTimelineClip,
  mediaForClip,
  titleDraft,
  setTitleDraft,
  saveTitleProperty,
  removeSelectedClip,
}: TitleInspectorProps) {
  if (!selectedTimelineClip) return null;

  const currentTitle = titleDraft ?? mediaForClip?.title ?? {
    text: "Title Text",
    subtitle: "",
    style: "lower_third",
    font_family: "Inter",
    font_size: 42,
    color: "#ffffff",
    bg_color: "rgba(15, 23, 42, 0.75)",
  };

  return (
    <>
      <h2>Title &amp; Graphic inspector</h2>
      <p>Customize lower-thirds, headline cards, and overlays.</p>
      <label>
        Headline text
        <input
          value={currentTitle.text}
          onChange={(e) => setTitleDraft({ ...currentTitle, text: e.target.value })}
          onBlur={(e) => void saveTitleProperty({ text: e.target.value })}
          placeholder="Headline text"
        />
      </label>
      <label>
        Subtitle / Description
        <input
          value={currentTitle.subtitle ?? ""}
          onChange={(e) => setTitleDraft({ ...currentTitle, subtitle: e.target.value })}
          onBlur={(e) => void saveTitleProperty({ subtitle: e.target.value })}
          placeholder="Presenter name, role, or description"
        />
      </label>
      <label>
        Graphic style
        <select
          value={currentTitle.style}
          onChange={(e) => void saveTitleProperty({ style: e.target.value })}
        >
          <option value="lower_third">Lower-Third (Broadcasting)</option>
          <option value="center_title">Centered Title (Cinematic)</option>
          <option value="minimal_pill">Minimal Pill (Modern Streamer)</option>
        </select>
      </label>
      <label>
        Font size ({Math.round(currentTitle.font_size)}px)
        <input
          type="range"
          min="18"
          max="96"
          step="2"
          value={currentTitle.font_size}
          onChange={(e) => setTitleDraft({ ...currentTitle, font_size: Number(e.target.value) })}
          onPointerUp={(e) => void saveTitleProperty({ font_size: Number(e.currentTarget.value) })}
        />
      </label>
      <label>
        Text color
        <input
          type="color"
          value={currentTitle.color || "#ffffff"}
          onChange={(e) => {
            setTitleDraft({ ...currentTitle, color: e.target.value });
            void saveTitleProperty({ color: e.target.value });
          }}
        />
      </label>
      <div className="control">Start <span>{formatTime(selectedTimelineClip.timeline_start)}</span></div>
      <div className="control">Duration <span>{formatTime(selectedTimelineClip.duration)}</span></div>
      <InteriorButton className="danger-button" onClick={removeSelectedClip}>Remove title clip</InteriorButton>
    </>
  );
}
