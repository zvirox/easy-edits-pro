// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md

export interface AgentSessionStatusResponse {
  has_active_session: boolean;
  session_id: string | null;
  is_stopped: boolean;
  generation: number;
  actor: unknown;
  active_branch: string;
  playhead: number;
  locks: Array<unknown>;
  sessions?: Array<{
    session_id: string;
    actor: unknown;
    active_branch: string;
    playhead: number;
    generation: number;
    is_stopped: boolean;
  }>;
}

export interface AgentStopResponse {
  session_id: string;
  stopped: boolean;
  generation: number;
}

export interface AgentResumeResponse {
  session_id: string;
  resumed: boolean;
  generation: number;
}

export interface AgentUndoChangeResponse {
  undone_change_id: number;
  undo_change_id: number;
  command_id: string;
  affected_entities: number[];
}

export interface AgentBridgeClient {
  stopSession(sessionId: string): Promise<AgentStopResponse>;
  resumeSession(sessionId: string): Promise<AgentResumeResponse>;
  getSessionStatus(sessionId?: string | null): Promise<AgentSessionStatusResponse>;
  undoChange(changeId: number): Promise<AgentUndoChangeResponse>;
  runCommand<T = unknown>(id: string, params?: unknown): Promise<T>;
}

export function isDemoAgentMode(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  try {
    return new URLSearchParams(window.location.search).get("demo") === "1";
  } catch {
    return false;
  }
}

export function normalizeBridgeError(action: string, error: unknown): Error {
  if (error instanceof Error) {
    return error;
  }
  const msg = typeof error === "string" ? error : JSON.stringify(error);
  return new Error(`${action} failed: ${msg}`);
}

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

const defaultClient: AgentBridgeClient = {
  async stopSession(sessionId: string): Promise<AgentStopResponse> {
    if (!sessionId.trim()) {
      throw new Error("No active agent session to stop");
    }
    try {
      return await tauriInvoke<AgentStopResponse>("agent_session_stop", { sessionId });
    } catch (error) {
      throw normalizeBridgeError("agent_session_stop", error);
    }
  },

  async resumeSession(sessionId: string): Promise<AgentResumeResponse> {
    if (!sessionId.trim()) {
      throw new Error("No active agent session to resume");
    }
    try {
      return await tauriInvoke<AgentResumeResponse>("agent_session_resume", { sessionId });
    } catch (error) {
      throw normalizeBridgeError("agent_session_resume", error);
    }
  },

  async getSessionStatus(
    sessionId?: string | null,
  ): Promise<AgentSessionStatusResponse> {
    try {
      return await tauriInvoke<AgentSessionStatusResponse>("agent_session_status", {
        sessionId: sessionId ?? null,
      });
    } catch (error) {
      throw normalizeBridgeError("agent_session_status", error);
    }
  },

  async undoChange(changeId: number): Promise<AgentUndoChangeResponse> {
    try {
      return await tauriInvoke<AgentUndoChangeResponse>("agent_undo_change", { changeId });
    } catch (error) {
      throw normalizeBridgeError("agent_undo_change", error);
    }
  },

  async runCommand<T = unknown>(id: string, params: unknown = null): Promise<T> {
    try {
      return await tauriInvoke<T>("execute_command", { request: { id, params } });
    } catch (error) {
      throw normalizeBridgeError(id, error);
    }
  },
};

let overrideClient: Partial<AgentBridgeClient> | null = null;

export function setAgentBridgeOverride(override: Partial<AgentBridgeClient> | null): void {
  overrideClient = override;
}

export const agentBridge: AgentBridgeClient = {
  stopSession: (sessionId) =>
    (overrideClient?.stopSession ?? defaultClient.stopSession)(sessionId),
  resumeSession: (sessionId) =>
    (overrideClient?.resumeSession ?? defaultClient.resumeSession)(sessionId),
  getSessionStatus: (sessionId) =>
    (overrideClient?.getSessionStatus ?? defaultClient.getSessionStatus)(sessionId),
  undoChange: (changeId) =>
    (overrideClient?.undoChange ?? defaultClient.undoChange)(changeId),
  runCommand: <T = unknown>(id: string, params?: unknown) =>
    (overrideClient?.runCommand ?? defaultClient.runCommand)<T>(id, params),
};
