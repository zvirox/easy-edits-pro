import { motion, useReducedMotion } from "motion/react";
import type { ComponentProps, DragEventHandler, ReactNode } from "react";
import type { HTMLMotionProps } from "motion/react";

export type InteriorButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type InteriorButtonSize = "xs" | "sm" | "md" | "lg" | "icon";

type InteriorButtonProps = ComponentProps<"button"> & {
  variant?: InteriorButtonVariant;
  size?: InteriorButtonSize;
  loading?: boolean;
  loadingLabel?: string;
};

/** Animated, accessible button primitive used throughout the editor. */
export function InteriorButton({
  children,
  variant,
  size,
  loading = false,
  loadingLabel = "Working…",
  className = "",
  disabled,
  onDragStart,
  onDragEnd,
  "aria-busy": ariaBusy,
  ...props
}: InteriorButtonProps) {
  const reducedMotion = useReducedMotion();
  const classes = ["interior-button", variant && `interior-button--${variant}`, size && `interior-button--${size}`, className]
    .filter(Boolean)
    .join(" ");
  const motionProps = props as HTMLMotionProps<"button">;

  return (
    <motion.button
      {...motionProps}
      className={classes}
      data-interior-button="true"
      disabled={disabled || loading}
      aria-busy={loading || ariaBusy || undefined}
      onDragStartCapture={onDragStart as DragEventHandler<HTMLButtonElement> | undefined}
      onDragEndCapture={onDragEnd as DragEventHandler<HTMLButtonElement> | undefined}
      whileHover={!reducedMotion && !disabled && !loading ? { y: -1, scale: 1.01 } : undefined}
      whileTap={!reducedMotion && !disabled && !loading ? { scale: 0.97 } : undefined}
      transition={reducedMotion ? { duration: 0 } : { type: "spring", stiffness: 460, damping: 28, mass: 0.55 }}
    >
      {loading && (
        <>
          <span className="interior-button__spinner" aria-hidden="true" />
          <span className="sr-only" role="status" aria-live="polite">{loadingLabel}</span>
        </>
      )}
      {children as ReactNode}
    </motion.button>
  );
}
