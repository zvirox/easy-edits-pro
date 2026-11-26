// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { AnimatePresence, motion, type HTMLMotionProps } from 'motion/react';
import type { CSSProperties, MouseEventHandler, ReactNode } from 'react';
import { backdropVariants, dialogVariants, useMotionPreset } from '../../motion/variants';

type CardProps<T extends 'div' | 'form'> = Omit<
  HTMLMotionProps<T>,
  'initial' | 'animate' | 'exit' | 'variants' | 'transition' | 'children'
>;

interface LinearDialogBaseProps {
  open: boolean;
  cardClassName: string;
  children: ReactNode;
  backdropClassName?: string;
  backdropStyle?: CSSProperties;
  onBackdropClick?: MouseEventHandler<HTMLDivElement>;
}

type LinearDialogDivProps = LinearDialogBaseProps & {
  as?: 'div';
  cardProps?: CardProps<'div'>;
};

type LinearDialogFormProps = LinearDialogBaseProps & {
  as: 'form';
  cardProps?: CardProps<'form'>;
};

export type LinearDialogProps = LinearDialogDivProps | LinearDialogFormProps;

/**
 * Shared Linear mount/unmount shell for modal dialogs with a solid dimmed scrim.
 * Zero blur, zero dark-mode shadow, 8px card rounding.
 */
export function LinearDialog(props: LinearDialogProps) {
  const preset = useMotionPreset();
  const {
    open,
    cardClassName,
    children,
    backdropClassName = 'dialog-backdrop',
    backdropStyle,
    onBackdropClick,
  } = props;
  const presenceProps = {
    initial: preset.initial,
    animate: 'animate' as const,
    exit: 'exit' as const,
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
          <div
            aria-hidden="true"
            className="fixed inset-0 bg-scrim pointer-events-none"
          />
          {props.as === 'form' ? (
            <motion.form
              className={`relative z-10 rounded-[8px] border border-border-strong bg-surface text-fg shadow-card ${cardClassName}`}
              variants={dialogVariants}
              {...presenceProps}
              {...props.cardProps}
            >
              {children}
            </motion.form>
          ) : (
            <motion.div
              className={`relative z-10 rounded-[8px] border border-border-strong bg-surface text-fg shadow-card ${cardClassName}`}
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
