import { AnimatePresence, motion, type HTMLMotionProps } from "motion/react";
import type { CSSProperties, MouseEventHandler, ReactNode } from "react";
import { backdropVariants, dialogVariants, useMotionPreset } from "./variants";

type CardProps<T extends "div" | "form"> = Omit<
  HTMLMotionProps<T>,
  "initial" | "animate" | "exit" | "variants" | "transition" | "children"
>;

interface GlassDialogBaseProps {
  open: boolean;
  cardClassName: string;
  children: ReactNode;
  backdropClassName?: string;
  backdropStyle?: CSSProperties;
  onBackdropClick?: MouseEventHandler<HTMLDivElement>;
}

type GlassDialogDivProps = GlassDialogBaseProps & {
  as?: "div";
  cardProps?: CardProps<"div">;
};

type GlassDialogFormProps = GlassDialogBaseProps & {
  as: "form";
  cardProps?: CardProps<"form">;
};

export type GlassDialogProps = GlassDialogDivProps | GlassDialogFormProps;

/**
 * Shared Phase 3 mount/unmount shell for modal dialogs.
 *
 * AnimatePresence keeps the dialog mounted long enough to play the exit
 * animation. The card eases in with opacity/scale only; layout is untouched.
 */
export function GlassDialog(props: GlassDialogProps) {
  const preset = useMotionPreset();
  const {
    open,
    cardClassName,
    children,
    backdropClassName = "dialog-backdrop",
    backdropStyle,
    onBackdropClick,
  } = props;
  const presenceProps = {
    initial: preset.initial,
    animate: "animate" as const,
    exit: "exit" as const,
    transition: preset.transition,
  };

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className={backdropClassName}
          role="presentation"
          style={backdropStyle}
          onClick={onBackdropClick}
          initial={preset.initial}
          animate="animate"
          exit="exit"
          variants={backdropVariants}
          transition={preset.transition}
        >
          {props.as === "form" ? (
            <motion.form
              className={cardClassName}
              data-elev="4"
              variants={dialogVariants}
              {...presenceProps}
              {...props.cardProps}
            >
              {children}
            </motion.form>
          ) : (
            <motion.div
              className={cardClassName}
              data-elev="4"
              variants={dialogVariants}
              {...presenceProps}
              {...props.cardProps}
            >
              {children}
            </motion.div>
          )}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
