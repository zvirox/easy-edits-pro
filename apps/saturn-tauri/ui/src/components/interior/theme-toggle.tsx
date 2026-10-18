// Adapted from Interior's MIT-licensed Component System: https://www.interior.dev/
import { useCallback, useEffect, useRef } from "react";
import { animate, motion, useMotionValue, useReducedMotion, useTransform } from "motion/react";

const spring = { type: "spring", stiffness: 480, damping: 32, mass: 0.5 } as const;

export interface ThemeToggleProps {
  theme: "dark" | "light";
  onToggle: () => void;
  className?: string;
}

export function ThemeToggle({ theme, onToggle, className = "" }: ThemeToggleProps) {
  const isDark = theme === "dark";
  const selectedIndex = isDark ? 1 : 0;
  const reducedMotion = useReducedMotion();
  const position = useMotionValue(selectedIndex);
  const thumbX = useTransform(position, (index) => `${index * 100}%`);
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (reducedMotion) {
      position.set(selectedIndex);
      return;
    }
    const controls = animate(position, selectedIndex, spring);
    return () => controls.stop();
  }, [position, reducedMotion, selectedIndex]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " " || e.key === "ArrowLeft" || e.key === "ArrowRight") {
        e.preventDefault();
        onToggle();
      }
    },
    [onToggle]
  );

  return (
    <div
      className={`interior-theme-toggle-wrap ${className}`}
      data-tooltip={`Switch to ${isDark ? "Light" : "Dark"} Mode (Ctrl+Alt+T)`}
      data-tooltip-pos="bottom"
    >
      <button
        ref={buttonRef}
        type="button"
        role="switch"
        aria-checked={isDark}
        aria-label={`Toggle theme, currently ${theme} mode`}
        className="interior-theme-toggle-switch"
        onClick={onToggle}
        onKeyDown={handleKeyDown}
      >
        {/* Animated sliding thumb */}
        <motion.span
          className="interior-theme-thumb"
          style={{ x: thumbX }}
          aria-hidden="true"
        />

        {/* Light Option Icon */}
        <span
          className={`interior-theme-icon ${!isDark ? "is-active" : ""}`}
          aria-hidden="true"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="5" />
            <line x1="12" y1="1" x2="12" y2="3" />
            <line x1="12" y1="21" x2="12" y2="23" />
            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
            <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
            <line x1="1" y1="12" x2="3" y2="12" />
            <line x1="21" y1="12" x2="23" y2="12" />
            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
            <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
          </svg>
        </span>

        {/* Dark Option Icon */}
        <span
          className={`interior-theme-icon ${isDark ? "is-active" : ""}`}
          aria-hidden="true"
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
          </svg>
        </span>

        {/* Invisible reserve layout to guarantee 0 layout shift */}
        <span className="interior-theme-reserve" aria-hidden="true">
          <span>☀️</span>
          <span>🌙</span>
        </span>
      </button>
    </div>
  );
}
