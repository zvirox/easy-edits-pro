import { Component, type ErrorInfo, type ReactNode } from "react";

export interface ErrorBoundaryProps {
  children: ReactNode;
  fallbackTitle?: string;
  onReset?: () => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  override state: ErrorBoundaryState = {
    hasError: false,
    error: null,
    errorInfo: null,
  };

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  override componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error("Easy Edits Pro Uncaught Error:", error, errorInfo);
    this.setState({ errorInfo });
  }

  handleReset = (): void => {
    this.setState({ hasError: false, error: null, errorInfo: null });
    this.props.onReset?.();
  };

  handleReload = (): void => {
    window.location.reload();
  };

  override render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div
          role="alert"
          style={{
            position: "relative",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            width: "100%",
            height: "100%",
            minHeight: "280px",
            padding: "24px",
            background: "var(--color-bg-base, #09090b)",
            color: "var(--color-text-primary, #f8fafc)",
            fontFamily: "Inter Variable, Inter, system-ui, -apple-system, sans-serif",
            textAlign: "center",
            boxSizing: "border-box",
            overflow: "hidden",
          }}
        >
          {/* Ambient Top Glow */}
          <div
            aria-hidden="true"
            style={{
              position: "absolute",
              top: "-80px",
              left: "50%",
              transform: "translateX(-50%)",
              width: "600px",
              height: "360px",
              background: "radial-gradient(circle, var(--color-accent-glow, rgba(99, 102, 241, 0.18)) 0%, transparent 70%)",
              pointerEvents: "none",
              filter: "blur(50px)",
              zIndex: 0,
            }}
          />

          <div
            style={{
              position: "relative",
              zIndex: 1,
              maxWidth: "520px",
              width: "100%",
              padding: "32px 28px",
              borderRadius: "18px",
              background: "var(--color-bg-surface, #121216)",
              border: "1px solid var(--color-border-subtle, rgba(255, 255, 255, 0.08))",
              boxShadow: "0 24px 60px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.06)",
              backdropFilter: "blur(20px)",
              WebkitBackdropFilter: "blur(20px)",
            }}
          >
            <div
              style={{
                width: "48px",
                height: "48px",
                margin: "0 auto 16px",
                borderRadius: "12px",
                background: "rgba(239, 68, 68, 0.12)",
                border: "1px solid rgba(239, 68, 68, 0.25)",
                color: "#f87171",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: "22px",
              }}
            >
              ⚠
            </div>
            <h2
              style={{
                margin: "0 0 8px",
                fontSize: "19px",
                fontWeight: 600,
                letterSpacing: "-0.02em",
                color: "var(--color-text-primary, #f8fafc)",
              }}
            >
              {this.props.fallbackTitle ?? "Workspace Rendering Issue"}
            </h2>
            <p
              style={{
                margin: "0 0 18px",
                fontSize: "13px",
                color: "var(--color-text-secondary, #94a3b8)",
                lineHeight: 1.6,
              }}
            >
              The studio encountered an unexpected error while rendering this workspace. Your project document remains safe in memory.
            </p>
            {this.state.error && (
              <pre
                style={{
                  margin: "0 0 22px",
                  padding: "12px 14px",
                  borderRadius: "8px",
                  background: "var(--color-bg-base, #09090b)",
                  border: "1px solid rgba(239, 68, 68, 0.2)",
                  color: "#fca5a5",
                  fontSize: "12px",
                  fontFamily: "ui-monospace, 'SF Mono', Menlo, Consolas, monospace",
                  textAlign: "left",
                  overflowX: "auto",
                  maxHeight: "120px",
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                }}
              >
                {this.state.error.message || String(this.state.error)}
              </pre>
            )}
            <div style={{ display: "flex", gap: "10px", justifyContent: "center" }}>
              <button
                type="button"
                onClick={this.handleReset}
                style={{
                  padding: "9px 20px",
                  borderRadius: "8px",
                  background: "var(--color-accent-primary, #6366f1)",
                  color: "#ffffff",
                  border: "1px solid rgba(255, 255, 255, 0.14)",
                  boxShadow: "0 0 20px var(--color-accent-glow, rgba(99, 102, 241, 0.35))",
                  fontWeight: 600,
                  fontSize: "13px",
                  cursor: "pointer",
                  transition: "all 0.15s ease",
                }}
              >
                Try Again
              </button>
              <button
                type="button"
                onClick={this.handleReload}
                style={{
                  padding: "9px 20px",
                  borderRadius: "8px",
                  background: "var(--color-bg-elevated, #18181c)",
                  color: "var(--color-text-primary, #f8fafc)",
                  border: "1px solid var(--color-border-subtle, rgba(255, 255, 255, 0.08))",
                  fontSize: "13px",
                  fontWeight: 500,
                  cursor: "pointer",
                  transition: "all 0.15s ease",
                }}
              >
                Reload Studio
              </button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
