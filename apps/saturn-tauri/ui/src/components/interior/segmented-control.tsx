// Adapted from Interior's MIT-licensed Segmented Control: https://www.interior.dev/docs/segmented-control
import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import { animate, motion, useMotionValue, useReducedMotion, useTransform } from "motion/react";

const spring = { type: "spring", stiffness: 520, damping: 34, mass: 0.45 } as const;

export type SegmentedOption = { value: string; label: string; disabled?: boolean };
type SegmentedControlProps = {
  options: SegmentedOption[];
  label: string;
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  className?: string;
};

export function SegmentedControl({ options, label, value, defaultValue, onValueChange, className = "" }: SegmentedControlProps) {
  const count = Math.max(1, options.length);
  const columns = `repeat(${count}, minmax(max-content, 1fr))`;
  const [internalValue, setInternalValue] = useState(() => defaultValue ?? options[0]?.value ?? "");
  const controlled = value !== undefined;
  const currentValue = controlled ? value : internalValue;
  const foundIndex = options.findIndex((option) => option.value === currentValue);
  const selectedIndex = foundIndex < 0 ? 0 : foundIndex;
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const callback = useRef(onValueChange);
  callback.current = onValueChange;
  const reducedMotion = useReducedMotion();
  const position = useMotionValue(selectedIndex);
  const thumbX = useTransform(position, (index) => `${index * 100}%`);
  const labelX = useTransform(position, (index) => `${index * -100}%`);

  useEffect(() => {
    if (reducedMotion) {
      position.set(selectedIndex);
      return;
    }
    const controls = animate(position, selectedIndex, spring);
    return () => controls.stop();
  }, [position, reducedMotion, selectedIndex]);

  const select = useCallback((nextValue: string) => {
    if (!controlled) setInternalValue(nextValue);
    if (nextValue !== currentValue) callback.current?.(nextValue);
  }, [controlled, currentValue]);

  const seek = useCallback((from: number, direction: number) => {
    let index = from;
    for (let step = 0; step < count; step += 1) {
      index = (index + direction + count) % count;
      if (!options[index]?.disabled) return index;
    }
    return from;
  }, [count, options]);

  const go = useCallback((index: number) => {
    const option = options[index];
    if (!option || option.disabled) return;
    buttons.current[index]?.focus();
    select(option.value);
  }, [options, select]);

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      event.preventDefault();
      go(seek(index, 1));
    } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      event.preventDefault();
      go(seek(index, -1));
    } else if (event.key === "Home") {
      event.preventDefault();
      go(seek(count - 1, 1));
    } else if (event.key === "End") {
      event.preventDefault();
      go(seek(0, -1));
    }
  };

  return (
    <div role="radiogroup" aria-label={label} className={`interior-segmented ${className}`}>
      <div className="interior-segmented-grid" style={{ gridTemplateColumns: columns }}>
        {options.map((option, index) => (
          <span key={option.value} aria-hidden="true" className={`interior-segment-label ${option.disabled ? "is-disabled" : index === selectedIndex ? "is-selected" : ""}`}>
            {option.label}
          </span>
        ))}
        <motion.div aria-hidden="true" className="interior-segment-thumb" style={{ width: `${100 / count}%`, x: thumbX }} initial={false}>
          <motion.div className="interior-segment-mask" style={{ x: labelX }} initial={false}>
            <div className="interior-segment-selected-labels" style={{ width: `${count * 100}%`, gridTemplateColumns: columns }}>
              {options.map((option) => <span key={option.value} className="interior-segment-label">{option.label}</span>)}
            </div>
          </motion.div>
        </motion.div>
        <div className="interior-segment-buttons" style={{ gridTemplateColumns: columns }}>
          {options.map((option, index) => (
            <button
              key={option.value}
              ref={(node) => { buttons.current[index] = node; }}
              type="button"
              role="radio"
              aria-checked={index === selectedIndex}
              aria-disabled={option.disabled || undefined}
              tabIndex={index === selectedIndex ? 0 : -1}
              onClick={() => !option.disabled && select(option.value)}
              onKeyDown={(event) => handleKeyDown(event, index)}
              className="interior-segment-hit-target"
            >
              <span className="sr-only">{option.label}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
