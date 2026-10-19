import { InteriorButton } from "./interior/interior-button";
import React, { useRef, useCallback } from "react";

export interface ColorWheelValue {
  x: number; // -1.0 to 1.0 (tint bias)
  y: number; // -1.0 to 1.0
  yLum: number; // -1.0 to 1.0 (luminance level)
}

export interface ColorWheelsData {
  lift: ColorWheelValue;
  gamma: ColorWheelValue;
  gain: ColorWheelValue;
  offset: ColorWheelValue;
}

export const DEFAULT_COLOR_WHEELS: ColorWheelsData = {
  lift: { x: 0, y: 0, yLum: 0 },
  gamma: { x: 0, y: 0, yLum: 0 },
  gain: { x: 0, y: 0, yLum: 0 },
  offset: { x: 0, y: 0, yLum: 0 },
};

export interface ColorWheelsProps {
  value: ColorWheelsData;
  onChange: (next: ColorWheelsData) => void;
  disabled?: boolean;
  className?: string;
}

interface SingleWheelProps {
  title: string;
  subtitle: string;
  val: ColorWheelValue;
  onChange: (next: ColorWheelValue) => void;
  disabled?: boolean;
}

const SingleWheel: React.FC<SingleWheelProps> = ({
  title,
  subtitle,
  val,
  onChange,
  disabled = false,
}) => {
  const discRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

  const updateFromPointer = useCallback(
    (clientX: number, clientY: number) => {
      const disc = discRef.current;
      if (!disc) return;
      const rect = disc.getBoundingClientRect();
      const centerX = rect.left + rect.width / 2;
      const centerY = rect.top + rect.height / 2;
      const radius = rect.width / 2;

      let dx = (clientX - centerX) / radius;
      let dy = (clientY - centerY) / radius;

      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist > 1) {
        dx /= dist;
        dy /= dist;
      }

      onChange({
        ...val,
        x: Math.round(dx * 100) / 100,
        y: Math.round(dy * 100) / 100,
      });
    },
    [val, onChange]
  );

  const handlePointerDown = (e: React.PointerEvent) => {
    if (disabled || e.button !== 0) return;
    e.preventDefault();
    isDragging.current = true;
    e.currentTarget.setPointerCapture(e.pointerId);
    updateFromPointer(e.clientX, e.clientY);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!isDragging.current || disabled) return;
    updateFromPointer(e.clientX, e.clientY);
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    if (isDragging.current) {
      isDragging.current = false;
      try {
        e.currentTarget.releasePointerCapture(e.pointerId);
      } catch {}
    }
  };

  const resetPuck = () => {
    if (disabled) return;
    onChange({ ...val, x: 0, y: 0 });
  };

  const resetLum = () => {
    if (disabled) return;
    onChange({ ...val, yLum: 0 });
  };

  const resetAll = () => {
    if (disabled) return;
    onChange({ x: 0, y: 0, yLum: 0 });
  };

  // Convert (x, y) to puck pixel position within 100px disc
  const puckLeft = 50 + val.x * 44;
  const puckTop = 50 + val.y * 44;

  return (
    <article className={`color-wheel-item ${disabled ? "is-disabled" : ""}`}>
      <header className="wheel-header">
        <div className="wheel-titles">
          <strong>{title}</strong>
          <small>{subtitle}</small>
        </div>
        <InteriorButton
          type="button"
          className="wheel-reset-btn"
          title={`Reset ${title} wheel to neutral`}
          onClick={resetAll}
          disabled={disabled}
        >
          ↺
        </InteriorButton>
      </header>

      {/* 2D Circular Color Wheel Disc */}
      <div
        ref={discRef}
        className="wheel-disc"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
        onDoubleClick={resetPuck}
        title="Drag to balance color hue & saturation (Double-click to reset)"
      >
        <div className="wheel-disc-conic-bg" />
        <div className="wheel-crosshair horizontal" />
        <div className="wheel-crosshair vertical" />
        <div className="wheel-center-guide" />
        <div
          className="wheel-puck"
          style={{
            left: `${puckLeft}%`,
            top: `${puckTop}%`,
          }}
        />
      </div>

      {/* Master Luminance Slider */}
      <div className="wheel-lum-section">
        <div className="wheel-lum-label-row">
          <span>Y LUM</span>
          <InteriorButton
            type="button"
            className="lum-reset-btn"
            title="Reset luminance to 0.00"
            onClick={resetLum}
            disabled={disabled}
          >
            0
          </InteriorButton>
          <code>{val.yLum > 0 ? `+${val.yLum.toFixed(2)}` : val.yLum.toFixed(2)}</code>
        </div>
        <input
          type="range"
          className="wheel-lum-slider"
          min="-1"
          max="1"
          step="0.02"
          disabled={disabled}
          value={val.yLum}
          onChange={(e) => onChange({ ...val, yLum: Number(e.target.value) })}
        />
      </div>

      {/* Numeric Vector Readout */}
      <footer className="wheel-footer-readout">
        <span>X <b>{val.x > 0 ? `+${val.x.toFixed(2)}` : val.x.toFixed(2)}</b></span>
        <span>Y <b>{val.y > 0 ? `+${val.y.toFixed(2)}` : val.y.toFixed(2)}</b></span>
      </footer>
    </article>
  );
};

export const ColorWheels: React.FC<ColorWheelsProps> = ({
  value,
  onChange,
  disabled = false,
  className = "",
}) => {
  const updateWheel = (name: keyof ColorWheelsData, wheelVal: ColorWheelValue) => {
    onChange({
      ...value,
      [name]: wheelVal,
    });
  };

  const resetAllWheels = () => {
    onChange(DEFAULT_COLOR_WHEELS);
  };

  return (
    <section className={`color-wheels-dock ${className}`}>
      <header className="color-wheels-header">
        <div className="wheels-header-title">
          <strong>PRIMARY COLOR BALANCE WHEELS</strong>
          <small>3-Way Lift / Gamma / Gain &amp; Master Offset</small>
        </div>
        <InteriorButton
          type="button"
          className="btn-secondary btn-reset-wheels"
          title="Reset all color wheels to neutral"
          onClick={resetAllWheels}
          disabled={disabled}
        >
          Reset All Wheels
        </InteriorButton>
      </header>

      <div className="color-wheels-grid">
        <SingleWheel
          title="LIFT"
          subtitle="Shadows"
          val={value.lift}
          onChange={(next) => updateWheel("lift", next)}
          disabled={disabled}
        />
        <SingleWheel
          title="GAMMA"
          subtitle="Midtones"
          val={value.gamma}
          onChange={(next) => updateWheel("gamma", next)}
          disabled={disabled}
        />
        <SingleWheel
          title="GAIN"
          subtitle="Highlights"
          val={value.gain}
          onChange={(next) => updateWheel("gain", next)}
          disabled={disabled}
        />
        <SingleWheel
          title="OFFSET"
          subtitle="Master"
          val={value.offset}
          onChange={(next) => updateWheel("offset", next)}
          disabled={disabled}
        />
      </div>
    </section>
  );
};

export default ColorWheels;
