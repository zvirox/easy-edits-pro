import { useEffect, useState } from "react";
import type { GraphicItem } from "../../engine/playback/types";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import type { GraphicOperations } from "../../app/useGraphics";
import {
  activeGraphics,
  overlayBox,
  shapeBackgroundCss,
  textOverlayStyle,
} from "./graphicsOverlay";

export interface GraphicsPanelProps {
  ops: GraphicOperations;
  playhead: number;
  /** Project frame size for the mini preview, in project pixels. */
  frame: { width: number; height: number };
}

type NumericField = {
  key: "x" | "y" | "width" | "height" | "opacity";
  label: string;
  step?: number;
};

const TRANSFORM_FIELDS: NumericField[] = [
  { key: "x", label: "X" },
  { key: "y", label: "Y" },
  { key: "width", label: "W" },
  { key: "height", label: "H" },
  { key: "opacity", label: "Opacity", step: 0.05 },
];

/** Essential Graphics: item list + property inspector (Phase 5 slice 3). */
export function GraphicsPanel({ ops, playhead, frame }: GraphicsPanelProps) {
  const [draft, setDraft] = useState<string>("");
  const selected = ops.selected;

  // Keep the text draft in sync when the selection changes under us.
  useEffect(() => {
    setDraft(selected?.text?.content ?? "");
  }, [selected?.id, selected?.text?.content]);

  const commitText = () => {
    if (!selected?.text) return;
    const next = draft.trim();
    if (next === "" || next === selected.text.content) {
      setDraft(selected.text.content);
      return;
    }
    void ops.updateText(selected.id, { text: next });
  };

  const active = activeGraphics(ops.graphics, playhead);
  const activeIds = new Set(active.map((graphic) => graphic.id));

  return (
    <div className="graphics-panel">
      <div className="graphics-add-row" role="group" aria-label="Add graphic">
        <InteriorButton variant="secondary" onClick={() => void ops.addGraphic("text")}>
          <Icon name="type" aria-hidden />
          Text
        </InteriorButton>
        <InteriorButton variant="secondary" onClick={() => void ops.addGraphic("rectangle")}>
          <Icon name="rectangle" aria-hidden />
          Rectangle
        </InteriorButton>
        <InteriorButton variant="secondary" onClick={() => void ops.addGraphic("ellipse")}>
          <Icon name="ellipse" aria-hidden />
          Ellipse
        </InteriorButton>
      </div>

      {ops.graphics.length === 0 ? (
        <p className="graphics-empty">
          No graphics yet. Add text, a rectangle, or an ellipse — items render over
          the video at their time range.
        </p>
      ) : (
        <ul className="graphics-list" aria-label="Graphics">
          {ops.graphics.map((graphic) => (
            <li key={graphic.id} className="graphics-list-row">
              <button
                type="button"
                className={`graphics-item${ops.selectedId === graphic.id ? " is-selected" : ""}${
                  activeIds.has(graphic.id) ? " is-live" : ""
                }`}
                onClick={() => ops.setSelectedId(graphic.id)}
                title={graphic.name}
              >
                <span className={`graphics-item-swatch kind-${graphic.kind}`} aria-hidden />
                <span className="graphics-item-name">{graphic.name}</span>
                <span className="graphics-item-kind">{graphic.kind}</span>
              </button>
              <div className="graphics-item-flags">
                <button
                  type="button"
                  className={`graphics-flag${graphic.visible ? " is-on" : ""}`}
                  onClick={() => void ops.setFlags(graphic.id, { visible: !graphic.visible })}
                  title={graphic.visible ? "Hide graphic" : "Show graphic"}
                >
                  {graphic.visible ? "eye" : "eye-off"}
                </button>
                <button
                  type="button"
                  className={`graphics-flag${graphic.locked ? " is-on" : ""}`}
                  onClick={() => void ops.setFlags(graphic.id, { locked: !graphic.locked })}
                  title={graphic.locked ? "Unlock graphic" : "Lock graphic"}
                >
                  {graphic.locked ? "locked" : "unlocked"}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}

      {selected && (
        <div className="graphics-inspector" aria-label="Graphic inspector">
          <header className="graphics-inspector-header">
            <span className="graphics-name" title="Graphic name">
              {selected.name}
            </span>
            <div className="graphics-inspector-actions">
              <InteriorButton
                variant="secondary"
                onClick={() => void ops.duplicateGraphic(selected.id)}
                title="Duplicate graphic"
              >
                <Icon name="remix" aria-hidden />
              </InteriorButton>
              <InteriorButton
                variant="secondary"
                onClick={() => void ops.saveTemplate(selected.id)}
                title="Save as template"
              >
                <Icon name="export" aria-hidden />
              </InteriorButton>
              <InteriorButton
                variant="danger"
                onClick={() => void ops.removeGraphic(selected.id)}
                title="Remove graphic"
              >
                <Icon name="remove" aria-hidden />
              </InteriorButton>
            </div>
          </header>

          <div className="graphics-mini-preview" aria-hidden>
            <div
              className="graphics-mini-frame"
              style={{
                aspectRatio: `${frame.width} / ${frame.height}`,
              }}
            >
              <MiniGraphic graphic={selected} frame={frame} />
            </div>
          </div>

          <div className="graphics-transform-grid">
            {TRANSFORM_FIELDS.map((field) => (
              <label key={field.key} className="graphics-field">
                <span>{field.label}</span>
                <input
                  type="number"
                  step={field.step ?? 1}
                  value={selected.transform[field.key]}
                  onChange={(event) => {
                    const value = Number(event.target.value);
                    if (Number.isFinite(value)) {
                      void ops.updateTransform(selected.id, { [field.key]: value });
                    }
                  }}
                />
              </label>
            ))}
          </div>

          <div className="graphics-arrange-row" role="group" aria-label="Arrange">
            {(["front", "forward", "backward", "back"] as const).map((order) => (
              <InteriorButton
                key={order}
                variant="secondary"
                onClick={() => void ops.arrange(selected.id, order)}
              >
                {order}
              </InteriorButton>
            ))}
          </div>

          {selected.text && (
            <section className="graphics-section">
              <h4>Text</h4>
              <textarea
                className="graphics-text-input"
                value={draft}
                rows={2}
                onChange={(event) => setDraft(event.target.value)}
                onBlur={commitText}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !event.shiftKey) {
                    event.preventDefault();
                    commitText();
                  }
                }}
              />
              <label className="graphics-field graphics-font-row">
                <span>Font</span>
                <select
                  value={selected.text.font_family}
                  onChange={(event) =>
                    void ops.updateText(selected.id!, { font_family: event.target.value })
                  }
                  onFocus={() => void ops.ensureFonts()}
                >
                  {(ops.fonts.length > 0
                    ? ops.fonts
                    : [selected.text.font_family]
                  ).map((family) => (
                    <option key={family} value={family}>
                      {family}
                    </option>
                  ))}
                </select>
              </label>
              <div className="graphics-transform-grid">
                <label className="graphics-field">
                  <span>Size</span>
                  <input
                    type="number"
                    min={1}
                    value={selected.text.font_size}
                    onChange={(event) => {
                      const value = Number(event.target.value);
                      if (Number.isFinite(value) && value > 0) {
                        void ops.updateText(selected.id!, { font_size: value });
                      }
                    }}
                  />
                </label>
                <label className="graphics-field">
                  <span>Tracking</span>
                  <input
                    type="number"
                    step={0.5}
                    value={selected.text.tracking}
                    onChange={(event) => {
                      const value = Number(event.target.value);
                      if (Number.isFinite(value)) {
                        void ops.updateText(selected.id!, { tracking: value });
                      }
                    }}
                  />
                </label>
                <label className="graphics-field">
                  <span>Leading</span>
                  <input
                    type="number"
                    step={0.1}
                    min={0.5}
                    value={selected.text.leading}
                    onChange={(event) => {
                      const value = Number(event.target.value);
                      if (Number.isFinite(value) && value >= 0.5) {
                        void ops.updateText(selected.id!, { leading: value });
                      }
                    }}
                  />
                </label>
              </div>
              <div className="graphics-align-row" role="group" aria-label="Paragraph alignment">
                {(["left", "center", "right"] as const).map((align) => (
                  <InteriorButton
                    key={align}
                    variant={selected.text?.align === align ? "primary" : "secondary"}
                    onClick={() => void ops.updateText(selected.id!, { align })}
                  >
                    {align}
                  </InteriorButton>
                ))}
              </div>
            </section>
          )}

          <section className="graphics-section">
            <h4>Fill</h4>
            <div className="graphics-color-row">
              <input
                type="color"
                value={toHex(selected.fill?.color)}
                onChange={(event) => void ops.setFill(selected.id, { color: event.target.value })}
                aria-label="Fill colour"
              />
              <label className="graphics-field">
                <span>Opacity</span>
                <input
                  type="number"
                  step={0.05}
                  min={0}
                  max={1}
                  value={selected.fill?.opacity ?? 1}
                  onChange={(event) => {
                    const value = Number(event.target.value);
                    if (Number.isFinite(value) && value >= 0 && value <= 1) {
                      void ops.setFill(selected.id, { opacity: value });
                    }
                  }}
                />
              </label>
            </div>
          </section>

          <section className="graphics-section">
            <h4>Stroke</h4>
            <div className="graphics-color-row">
              <input
                type="color"
                value={toHex(selected.stroke?.color ?? "#ffffff")}
                onChange={(event) => void ops.setStroke(selected.id, { color: event.target.value })}
                aria-label="Stroke colour"
              />
              <label className="graphics-field">
                <span>Width</span>
                <input
                  type="number"
                  step={0.5}
                  min={0}
                  value={selected.stroke?.width ?? 0}
                  onChange={(event) => {
                    const value = Number(event.target.value);
                    if (Number.isFinite(value) && value >= 0) {
                      void ops.setStroke(selected.id, { width: value });
                    }
                  }}
                />
              </label>
            </div>
          </section>
        </div>
      )}

      {ops.templates.length > 0 && (
        <section className="graphics-section graphics-templates" aria-label="Graphic templates">
          <h4>Templates</h4>
          <ul>
            {ops.templates.map((template) => (
              <li key={template.id}>
                <button
                  type="button"
                  className="graphics-template-row"
                  onClick={() => void ops.applyTemplate(template.id)}
                  title="Apply at playhead"
                >
                  <span>{template.name}</span>
                  <span className="graphics-item-kind">{template.body.kind}</span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

/** One graphic rendered as a DOM overlay box (percent of the frame). */
export function MiniGraphic({
  graphic,
  frame,
}: {
  graphic: GraphicItem;
  frame: { width: number; height: number };
}) {
  const box = overlayBox(graphic, frame);
  const style: Record<string, string> = {
    left: `${box.left}%`,
    top: `${box.top}%`,
    width: `${box.width}%`,
    height: `${box.height}%`,
    opacity: String(graphic.transform.opacity),
  };
  if (graphic.text) {
    return (
      <div className="graphics-overlay-text" style={{ ...style, ...textOverlayStyle(graphic, frame) }}>
        {graphic.text.content}
      </div>
    );
  }
  const shape = shapeBackgroundCss(graphic);
  return (
    <div
      className={`graphics-overlay-shape kind-${graphic.kind}`}
      style={{ ...style, ...shape }}
    />
  );
}

function toHex(color: string | undefined): string {
  if (color && /^#[0-9a-f]{6}$/i.test(color)) return color;
  return "#ffffff";
}
