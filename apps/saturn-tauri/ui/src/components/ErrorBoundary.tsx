// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { InteriorButton } from "./interior/interior-button";
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
          <div
            className="surface-1"
            style={{
              position: "relative",
              zIndex: 1,
              maxWidth: "520px",
              width: "100%",
              padding: "28px 24px",
              borderRadius: "8px",
            }}
          >
            <div
              style={{
                width: "44px",
                height: "44px",
                margin: "0 auto 16px",
                borderRadius: "8px",
                background: "var(--tint-red-bg)",
                border: "1px solid var(--status-danger-bright)",
                color: "var(--tint-red-text)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: "20px",
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
                color: "var(--text-primary)",
              }}
            >
              {this.props.fallbackTitle ?? "Workspace Rendering Issue"}
            </h2>
            <p
              style={{
                margin: "0 0 18px",
                fontSize: "13px",
                color: "var(--text-secondary)",
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
                  borderRadius: "6px",
                  background: "var(--bg-app)",
                  border: "1px solid var(--status-danger-bright)",
                  color: "var(--tint-red-text)",
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
              <InteriorButton
                type="button"
                onClick={this.handleReset}
                style={{
                  padding: "8px 16px",
                  borderRadius: "6px",
                  background: "var(--accent-primary)",
                  color: "var(--on-accent)",
                  border: "1px solid var(--accent-primary)",
                  fontWeight: 600,
                  fontSize: "13px",
                  cursor: "pointer",
                  transition: "background-color 150ms ease-out, border-color 150ms ease-out",
                }}
              >
                Try Again
              </InteriorButton>
              <InteriorButton
                type="button"
                onClick={this.handleReload}
                style={{
                  padding: "8px 16px",
                  borderRadius: "6px",
                  background: "var(--bg-surface-2)",
                  color: "var(--text-primary)",
                  border: "1px solid var(--border-color)",
                  fontSize: "13px",
                  fontWeight: 500,
                  cursor: "pointer",
                  transition: "background-color 150ms ease-out, border-color 150ms ease-out",
                }}
              >
                Reload Studio
              </InteriorButton>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
