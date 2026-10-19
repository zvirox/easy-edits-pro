import { InteriorButton } from "./interior-button";
// Adapted from Interior's MIT-licensed Loading Button: https://www.interior.dev/docs/loading-button
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";

type Status = "idle" | "pending" | "success" | "error";
type LoadingButtonProps = {
  children: ReactNode;
  onAction: () => unknown;
  pendingLabel: string;
  successLabel: string;
  errorLabel: string;
  className?: string;
  disabled?: boolean;
  onError?: (error: unknown) => void;
};

export function LoadingButton({ children, onAction, pendingLabel, successLabel, errorLabel, className = "", disabled = false, onError }: LoadingButtonProps) {
  const [status, setStatus] = useState<Status>("idle");
  const phase = useRef<Status>("idle");
  const action = useRef(onAction);
  const reportError = useRef(onError);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reducedMotion = useReducedMotion();
  action.current = onAction;
  reportError.current = onError;

  useEffect(() => () => {
    if (timer.current) clearTimeout(timer.current);
  }, []);

  const run = useCallback(async () => {
    if (phase.current === "pending" || disabled) return;
    if (timer.current) clearTimeout(timer.current);
    phase.current = "pending";
    setStatus("pending");
    try {
      await action.current();
      phase.current = "success";
      setStatus("success");
    } catch (error) {
      reportError.current?.(error);
      phase.current = "error";
      setStatus("error");
    }
    timer.current = setTimeout(() => {
      phase.current = "idle";
      setStatus("idle");
      timer.current = null;
    }, 1400);
  }, [disabled]);

  const label = status === "pending" ? pendingLabel : status === "success" ? successLabel : status === "error" ? errorLabel : children;
  return (
    <InteriorButton type="button" className={`interior-loading-button ${className}`} disabled={disabled || status === "pending"} aria-busy={status === "pending"} onClick={() => void run()}>
      <span className="interior-loading-reserve" aria-hidden="true"><span>{children}</span><span>{pendingLabel}</span><span>{successLabel}</span><span>{errorLabel}</span></span>
      <span className="interior-loading-label" aria-live="polite">
        {status === "pending" && <motion.svg className="interior-loading-spinner" width="13" height="13" viewBox="0 0 12 12" fill="none" aria-hidden="true" animate={reducedMotion ? undefined : { rotate: 360 }} transition={reducedMotion ? undefined : { duration: 0.85, repeat: Infinity, ease: "linear" }}><circle cx="6" cy="6" r="4.5" stroke="currentColor" strokeWidth="1.5" strokeOpacity=".25" /><path d="M6 1.5a4.5 4.5 0 0 1 4.5 4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" /></motion.svg>}
        {label}
      </span>
    </InteriorButton>
  );
}
