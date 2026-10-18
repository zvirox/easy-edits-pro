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
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            width: "100%",
            height: "100%",
            minHeight: "240px",
            padding: "24px",
            background: "var(--bg-app, #0b0d10)",
            color: "var(--text-primary, #f3f4f6)",
            fontFamily: "system-ui, -apple-system, sans-serif",
            textAlign: "center",
            boxSizing: "border-box",
          }}
        >
          <div
            style={{
              maxWidth: "520px",
              padding: "28px",
              borderRadius: "12px",
              background: "var(--bg-panel, #13161b)",
              border: "1px solid var(--border-panel, #262b34)",
              boxShadow: "0 16px 36px rgba(0, 0, 0, 0.4)",
            }}
          >
            <div
              style={{
                width: "48px",
                height: "48px",
                margin: "0 auto 16px",
                borderRadius: "50%",
                background: "rgba(239, 68, 68, 0.15)",
                color: "#ef4444",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: "24px",
              }}
            >
              ⚠
            </div>
            <h2 style={{ margin: "0 0 8px", fontSize: "18px", fontWeight: 600 }}>
              {this.props.fallbackTitle ?? "Workspace Rendering Issue"}
            </h2>
            <p style={{ margin: "0 0 16px", fontSize: "13px", color: "var(--text-secondary, #9ca3af)", lineHeight: 1.5 }}>
              The studio encountered an unexpected error while rendering this workspace. Your project document remains safe in memory.
            </p>
            {this.state.error && (
              <pre
                style={{
                  margin: "0 0 20px",
                  padding: "10px 14px",
                  borderRadius: "6px",
                  background: "var(--bg-elevated, #1c2027)",
                  border: "1px solid var(--border-panel, #262b34)",
                  color: "#f87171",
                  fontSize: "12px",
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
                  padding: "8px 18px",
                  borderRadius: "6px",
                  background: "var(--accent-primary, #0ea5e9)",
                  color: "#ffffff",
                  border: "none",
                  fontWeight: 600,
                  fontSize: "13px",
                  cursor: "pointer",
                }}
              >
                Try Again
              </button>
              <button
                type="button"
                onClick={this.handleReload}
                style={{
                  padding: "8px 18px",
                  borderRadius: "6px",
                  background: "var(--bg-elevated, #1c2027)",
                  color: "var(--text-primary, #f3f4f6)",
                  border: "1px solid var(--border-panel, #262b34)",
                  fontSize: "13px",
                  cursor: "pointer",
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
