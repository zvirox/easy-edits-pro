import { InteriorButton } from "./interior/interior-button";
import React, { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  timestamp?: string;
  actionKey?: string;
}

interface OllamaChatDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  onExecuteAction?: (actionKey: string, payload?: any) => void;
}

const DEFAULT_SYSTEM_PROMPT = `You are Saturn AI, an expert professional video editing assistant integrated directly into Saturn Video Editor. 
You provide concrete timeline editing advice, color grading LUT/curves recommendations, audio mastering guidance (EBU R128), shader effect tips, and project structuring. 
Keep your answers concise, practical, and directly applicable to professional NLE workflows.`;

const AVAILABLE_MODELS = [
  { id: "llama3.2", name: "Llama 3.2 (Default · Fast)", description: "3B lightweight model for fast timeline commands" },
  { id: "mistral", name: "Mistral 7B", description: "High-reasoning general creative assistant" },
  { id: "qwen2.5", name: "Qwen 2.5", description: "Excellent multilingual & coding capabilities" },
  { id: "deepseek-r1:8b", name: "DeepSeek R1 (8B)", description: "Deep reasoning & editing logic" },
  { id: "codellama", name: "CodeLlama", description: "Specialized in GLSL shader plugins" },
];

const QUICK_ACTIONS = [
  {
    label: "🎬 Ripple Delete Pauses",
    prompt: "Scan the timeline for silence intervals below -38 dBFS and explain how ripple deletion will compact dialogue pauses.",
    actionKey: "ripple_delete_pauses",
  },
  {
    label: "🎨 Warm Sunset Curve",
    prompt: "Recommend an S-curve contrast with lifted warm red highlights and cool teal shadow offsets for cinematic sunset grading.",
    actionKey: "warm_sunset_curves",
  },
  {
    label: "🎚️ EBU R128 Normalization",
    prompt: "Analyze loudness compliance for broadcast delivery (-23 LUFS integrated, -1.0 dBTP true peak) and suggest track gain adjustments.",
    actionKey: "ebu_r128_normalize",
  },
  {
    label: "📁 Group Dialogue Tracks",
    prompt: "How should dialogue tracks (DX1, DX2, Lav, Boom) be grouped, color-coded, and routed to the dialogue submix bus?",
    actionKey: "group_dialogue_tracks",
  },
  {
    label: "📺 Add Retro CRT Shader",
    prompt: "How can I configure the custom GLSL fragment shader for authentic CRT scanlines, aperture grille, and chromatic aberration?",
    actionKey: "add_crt_shader",
  },
  {
    label: "🤖 AI Background Matting",
    prompt: "Explain how RobustVideoMatting (RVM) extracts foreground alpha mattes without a green screen using temporal recurrent memory.",
    actionKey: "ai_matting_rvm",
  },
  {
    label: "📐 Export to OpenTimelineIO",
    prompt: "Guide me through exporting the current sequence to OpenTimelineIO (.otio) format for interchange with DaVinci Resolve.",
    actionKey: "export_otio",
  },
];

export const OllamaChatDrawer: React.FC<OllamaChatDrawerProps> = ({
  isOpen,
  onClose,
  onExecuteAction,
}) => {
  const [model, setModel] = useState("llama3.2");
  const [customModel, setCustomModel] = useState("");
  const [showCustomModel, setShowCustomModel] = useState(false);
  const [systemPrompt, setSystemPrompt] = useState(DEFAULT_SYSTEM_PROMPT);
  const [showSystemPrompt, setShowSystemPrompt] = useState(false);
  const [inputPrompt, setInputPrompt] = useState("");
  const [isSending, setIsSending] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      role: "assistant",
      content:
        "👋 Hello! I am your Saturn AI Timeline Assistant powered by local Ollama. Ask me about cuts, color curves, secondary qualifiers, batch rendering, or click any quick action below to get started.",
      timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
    },
  ]);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (isOpen) {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [isOpen, messages]);

  const activeModelName = showCustomModel && customModel.trim() ? customModel.trim() : model;

  const handleSendMessage = async (textToSend?: string, actionKey?: string) => {
    const text = textToSend ?? inputPrompt;
    if (!text.trim() || isSending) return;

    const userMsg: ChatMessage = {
      role: "user",
      content: text.trim(),
      timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
      actionKey,
    };

    const updatedMessages = [...messages, userMsg];
    setMessages(updatedMessages);
    if (!textToSend) setInputPrompt("");
    setIsSending(true);
    setStatusMessage("Generating response via Ollama...");

    try {
      const response = await invoke<string>("ask_local_ai", {
        prompt: text.trim(),
        model: activeModelName,
        messages: updatedMessages.map((m) => ({ role: m.role, content: m.content })),
        systemPrompt: systemPrompt.trim(),
      });

      const assistantMsg: ChatMessage = {
        role: "assistant",
        content: response,
        timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        actionKey,
      };

      setMessages((prev) => [...prev, assistantMsg]);
      setStatusMessage(null);
    } catch (err: any) {
      const fallbackResponse = `Notice: Local Ollama service is unavailable (${err?.toString() || "Connection error"}).\n\nFallback Editor Advice:\n${getFallbackResponse(text)}`;
      const assistantMsg: ChatMessage = {
        role: "assistant",
        content: fallbackResponse,
        timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        actionKey,
      };
      setMessages((prev) => [...prev, assistantMsg]);
      setStatusMessage(null);
    } finally {
      setIsSending(false);
    }
  };

  const getFallbackResponse = (query: string): string => {
    const q = query.toLowerCase();
    if (q.includes("pause") || q.includes("silence") || q.includes("ripple")) {
      return "To ripple delete pauses in Saturn, open the Timeline menu or use the Silence Detector. Clips will automatically ripple left to remove silent intervals below the threshold.";
    }
    if (q.includes("curve") || q.includes("sunset") || q.includes("color")) {
      return "For warm sunset grading: in the Color workspace, switch to RGB Curves. Lift Red highlights by pulling Red (0.75 -> 0.85), slightly dip Green midtones, and lower Blue in highlights while elevating it in shadows for a teal-and-orange balance.";
    }
    if (q.includes("loudness") || q.includes("ebu") || q.includes("r128")) {
      return "EBU R128 target is -23.0 LUFS with a -1.0 dBTP ceiling. Use the Master Audio Bus compressor/limiter and check the integrated loudness meter.";
    }
    if (q.includes("group") || q.includes("track")) {
      return "Track Groups allow collapsing multiple tracks (e.g. dialogue DX1-DX4) under a single visual header with synchronized Mute, Solo, and Lock controls.";
    }
    if (q.includes("shader") || q.includes("crt")) {
      return "Select any clip, open the Custom Shader inspector, choose the 'CRT Scanlines' preset or paste your custom GLSL fragment code, and adjust the scanline_intensity and distortion uniforms.";
    }
    if (q.includes("matting") || q.includes("rvm") || q.includes("modnet")) {
      return "RobustVideoMatting (RVM) extracts clean human alpha mattes temporally without requiring a green screen. Adjust feathering and threshold in the AI Matting panel.";
    }
    if (q.includes("otio") || q.includes("opentimelineio")) {
      return "You can export the entire sequence to OpenTimelineIO via File > Export > OpenTimelineIO (.otio) to seamlessly interchange with DaVinci Resolve or Final Cut Pro.";
    }
    return "Saturn AI is ready to assist with timeline edits, color grading, audio routing, and visual effects.";
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  const handleClearChat = () => {
    setMessages([
      {
        role: "assistant",
        content: "Chat history cleared. How can I assist you with your project?",
        timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
      },
    ]);
  };

  return (
    <aside
      className="ai-chat-drawer glass"
      role="complementary"
      aria-label="AI Assistant"
      data-open={isOpen ? "true" : "false"}
    >
        {/* Header */}
        <header
          className="ai-chat-drawer-header"
          style={{
            padding: "14px 18px",
            borderBottom: "1px solid var(--border-default)",
            background: "transparent",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span style={{ fontSize: 22 }}>🧠</span>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <strong style={{ fontSize: 15, color: "var(--text-primary)" }}>Saturn AI Assistant</strong>
                <span
                  style={{
                    fontSize: 9,
                    fontWeight: 700,
                    padding: "2px 6px",
                    borderRadius: 4,
                    background: "var(--border-default)",
                    color: "var(--text-secondary)",
                    letterSpacing: "0.5px",
                  }}
                >
                  OLLAMA CHAT
                </span>
              </div>
              <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
                Multi-turn conversational assistant & timeline automation
              </span>
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <InteriorButton
              type="button"
              onClick={handleClearChat}
              title="Clear Chat History"
              style={{
                background: "transparent",
                border: "1px solid var(--border-default)",
                color: "var(--text-secondary)",
                fontSize: 12,
                borderRadius: 4,
                padding: "4px 8px",
                cursor: "pointer",
              }}
            >
              🧹 Clear
            </InteriorButton>
            <InteriorButton
              type="button"
              onClick={onClose}
              style={{
                background: "transparent",
                border: "none",
                color: "var(--text-secondary)",
                fontSize: 18,
                cursor: "pointer",
                padding: "2px 6px",
              }}
            >
              ✕
            </InteriorButton>
          </div>
        </header>

        {/* Model & Config Bar */}
        <div
          style={{
            padding: "10px 16px",
            background: "var(--bg-surface)",
            borderBottom: "1px solid var(--border-subtle)",
            display: "flex",
            flexDirection: "column",
            gap: 8,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, flex: 1 }}>
              <span style={{ fontSize: 11, color: "var(--text-secondary)", fontWeight: 600 }}>Model:</span>
              {!showCustomModel ? (
                <select
                  value={model}
                  onChange={(e) => {
                    if (e.target.value === "custom") {
                      setShowCustomModel(true);
                    } else {
                      setModel(e.target.value);
                    }
                  }}
                  style={{
                    flex: 1,
                    background: "var(--bg-elevated)",
                    border: "1px solid var(--border-default)",
                    color: "var(--text-primary)",
                    fontSize: 12,
                    borderRadius: 4,
                    padding: "4px 8px",
                  }}
                >
                  {AVAILABLE_MODELS.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.name}
                    </option>
                  ))}
                  <option value="custom">Custom model...</option>
                </select>
              ) : (
                <div style={{ display: "flex", gap: 4, flex: 1 }}>
                  <input
                    type="text"
                    value={customModel}
                    onChange={(e) => setCustomModel(e.target.value)}
                    placeholder="e.g. llama3:70b"
                    style={{
                      flex: 1,
                      background: "var(--bg-elevated)",
                      border: "1px solid var(--accent-primary)",
                      color: "var(--text-primary)",
                      fontSize: 12,
                      borderRadius: 4,
                      padding: "4px 8px",
                    }}
                  />
                  <InteriorButton
                    type="button"
                    onClick={() => setShowCustomModel(false)}
                    style={{
                      background: "var(--border-default)",
                      border: "none",
                      color: "var(--text-primary)",
                      fontSize: 11,
                      padding: "4px 8px",
                      borderRadius: 4,
                      cursor: "pointer",
                    }}
                  >
                    Preset
                  </InteriorButton>
                </div>
              )}
            </div>

            <InteriorButton
              type="button"
              onClick={() => setShowSystemPrompt(!showSystemPrompt)}
              style={{
                background: showSystemPrompt ? "var(--border-default)" : "transparent",
                border: "1px solid var(--border-default)",
                color: showSystemPrompt ? "var(--accent-text)" : "var(--text-secondary)",
                fontSize: 11,
                borderRadius: 4,
                padding: "4px 8px",
                cursor: "pointer",
              }}
            >
              ⚙️ System Prompt
            </InteriorButton>
          </div>

          {showSystemPrompt && (
            <div style={{ marginTop: 4 }}>
              <textarea
                value={systemPrompt}
                onChange={(e) => setSystemPrompt(e.target.value)}
                rows={3}
                style={{
                  width: "100%",
                  background: "var(--bg-input)",
                  border: "1px solid var(--border-default)",
                  borderRadius: 4,
                  color: "var(--text-secondary)",
                  fontSize: 11,
                  padding: 8,
                  fontFamily: "monospace",
                  resize: "vertical",
                  boxSizing: "border-box",
                }}
              />
              <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 4 }}>
                <InteriorButton
                  type="button"
                  onClick={() => setSystemPrompt(DEFAULT_SYSTEM_PROMPT)}
                  style={{
                    background: "transparent",
                    border: "none",
                    color: "var(--text-muted)",
                    fontSize: 10,
                    cursor: "pointer",
                    textDecoration: "underline",
                  }}
                >
                  Reset to Default
                </InteriorButton>
              </div>
            </div>
          )}
        </div>

        {/* Quick Action Chips */}
        <div
          style={{
            padding: "8px 14px",
            background: "var(--bg-surface)",
            borderBottom: "1px solid var(--border-subtle)",
            overflowX: "auto",
            display: "flex",
            gap: 6,
            whiteSpace: "nowrap",
          }}
        >
          {QUICK_ACTIONS.map((action, idx) => (
            <InteriorButton
              key={idx}
              type="button"
              onClick={() => handleSendMessage(action.prompt, action.actionKey)}
              disabled={isSending}
              style={{
                background: "var(--bg-elevated)",
                border: "1px solid var(--border-default)",
                color: "var(--text-secondary)",
                fontSize: 11,
                padding: "4px 10px",
                borderRadius: 14,
                cursor: isSending ? "not-allowed" : "pointer",
                transition: "border-color 0.15s ease, color 0.15s ease",
                flexShrink: 0,
              }}
              onMouseEnter={(e) => (e.currentTarget.style.borderColor = "var(--accent-primary-hover)")}
              onMouseLeave={(e) => (e.currentTarget.style.borderColor = "var(--border-default)")}
            >
              {action.label}
            </InteriorButton>
          ))}
        </div>

        {/* Chat Message History */}
        <div
          style={{
            flex: 1,
            overflowY: "auto",
            padding: "16px 16px",
            display: "flex",
            flexDirection: "column",
            gap: 14,
          }}
        >
          {messages.map((msg, index) => {
            const isUser = msg.role === "user";
            return (
              <div
                key={index}
                style={{
                  display: "flex",
                  flexDirection: "column",
                  alignItems: isUser ? "flex-end" : "flex-start",
                  maxWidth: "92%",
                  alignSelf: isUser ? "flex-end" : "flex-start",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    marginBottom: 4,
                    fontSize: 10,
                    color: "var(--text-muted)",
                  }}
                >
                  <span>{isUser ? "You" : `Saturn AI (${activeModelName})`}</span>
                  {msg.timestamp && <span>· {msg.timestamp}</span>}
                </div>
                <div
                  style={{
                    background: isUser ? "color-mix(in srgb, var(--accent-primary) 18%, transparent)" : "var(--bg-elevated)",
                    color: "var(--text-primary)",
                    border: isUser ? "1px solid var(--accent-primary)" : "1px solid var(--border-default)",
                    borderRadius: isUser ? "12px 12px 2px 12px" : "12px 12px 12px 2px",
                    padding: "10px 14px",
                    fontSize: 13,
                    lineHeight: 1.5,
                    whiteSpace: "pre-wrap",
                    wordBreak: "break-word",
                  }}
                >
                  {msg.content}
                </div>
                {msg.actionKey && !isUser && onExecuteAction && (
                  <InteriorButton
                    type="button"
                    onClick={() => onExecuteAction(msg.actionKey!)}
                    style={{
                      marginTop: 6,
                      background: "color-mix(in srgb, var(--status-success) 18%, transparent)",
                      border: "1px solid var(--status-success)",
                      color: "var(--status-success)",
                      fontSize: 11,
                      fontWeight: 600,
                      padding: "4px 10px",
                      borderRadius: 4,
                      cursor: "pointer",
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                    }}
                  >
                    ⚡ Apply Suggested Action
                  </InteriorButton>
                )}
              </div>
            );
          })}
          {isSending && (
            <div
              style={{
                alignSelf: "flex-start",
                background: "var(--bg-elevated)",
                border: "1px solid var(--border-default)",
                borderRadius: "12px 12px 12px 2px",
                padding: "10px 14px",
                display: "flex",
                alignItems: "center",
                gap: 8,
                color: "var(--text-secondary)",
                fontSize: 12,
              }}
            >
              <div
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: "var(--accent-primary-hover)",
                  animation: "pulse 1s infinite alternate",
                }}
              />
              <span>{statusMessage || "Thinking..."}</span>
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>

        {/* Input Bar */}
        <div
          style={{
            padding: "12px 16px",
            background: "var(--bg-surface)",
            borderTop: "1px solid var(--border-subtle)",
            display: "flex",
            flexDirection: "column",
            gap: 8,
          }}
        >
          <div style={{ display: "flex", gap: 8, alignItems: "flex-end" }}>
            <textarea
              ref={inputRef}
              value={inputPrompt}
              onChange={(e) => setInputPrompt(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={isSending}
              placeholder="Ask Saturn AI about edits, color, audio, or shaders... (Enter to send, Shift+Enter for newline)"
              rows={2}
              style={{
                flex: 1,
                background: "var(--bg-input)",
                border: "1px solid var(--border-default)",
                borderRadius: 6,
                color: "var(--text-primary)",
                fontSize: 13,
                padding: "8px 12px",
                resize: "none",
                outline: "none",
                fontFamily: "inherit",
              }}
            />
            <InteriorButton
              type="button"
              onClick={() => handleSendMessage()}
              disabled={!inputPrompt.trim() || isSending}
              style={{
                height: 48,
                padding: "0 18px",
                background: !inputPrompt.trim() || isSending ? "var(--bg-input)" : "var(--accent-primary)",
                color: !inputPrompt.trim() || isSending ? "var(--text-muted)" : "var(--text-on-accent)",
                border: "none",
                borderRadius: 6,
                fontSize: 13,
                fontWeight: 600,
                cursor: !inputPrompt.trim() || isSending ? "not-allowed" : "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: 6,
              }}
            >
              Send ✈️
            </InteriorButton>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontSize: 10, color: "var(--text-muted)" }}>
              💡 Runs completely offline via local Ollama endpoint (http://127.0.0.1:11434).
            </span>
          </div>
        </div>
    </aside>
  );
};
