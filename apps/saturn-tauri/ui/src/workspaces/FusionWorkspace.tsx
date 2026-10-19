import { InteriorButton } from "../components/interior/interior-button";
import { type MouseEvent } from "react";
import {
  type ProjectDocument,
  type MediaAsset,
  type CompositingNode,
  type CompositingNodeKind,
  type NodeConnection,
  type RotoSplinePoint,
} from "../types";

export interface ConnectingSocket {
  nodeId: number;
  socket: string;
  isOutput: boolean;
}

export interface FusionWorkspaceProps {
  project: ProjectDocument["project"] | null;
  media: MediaAsset[];
  selectedNodeId: number | null;
  setSelectedNodeId: (id: number | null) => void;
  connectingSocket: ConnectingSocket | null;
  setConnectingSocket: (val: ConnectingSocket | null) => void;
  draggingNodeId: number | null;
  setDraggingNodeId: (id: number | null) => void;
  nodeDragOffset: { x: number; y: number };
  setNodeDragOffset: (val: { x: number; y: number }) => void;
  updateFusionNodePos: (id: number, x: number, y: number) => Promise<void>;
  connectFusionSockets: (fromNode: number, fromSock: string, toNode: number, toSock: string) => Promise<void>;
  disconnectFusionSockets: (fromNode: number, fromSock: string, toNode: number, toSock: string) => Promise<void>;
  addFusionNode: (kind: CompositingNodeKind, title?: string, x?: number, y?: number, params?: Record<string, unknown>) => Promise<void>;
  removeFusionNode: (id: number) => Promise<void>;
  updateFusionNodeParams: (id: number, params: Record<string, unknown>) => Promise<void>;
  fusionMaskFeather: number;
  setFusionMaskFeather: (val: number) => void;
  fusionMaskInverted: boolean;
  setFusionMaskInverted: (val: boolean) => void;
  fusionMaskPoints: RotoSplinePoint[];
  addRotoSplineMask: (points: RotoSplinePoint[], feather: number, closed?: boolean, inverted?: boolean) => Promise<void>;
  setNotice: (msg: string) => void;
}

export function FusionWorkspace({
  project,
  media,
  selectedNodeId,
  setSelectedNodeId,
  connectingSocket,
  setConnectingSocket,
  draggingNodeId,
  setDraggingNodeId,
  nodeDragOffset,
  setNodeDragOffset,
  updateFusionNodePos,
  connectFusionSockets,
  disconnectFusionSockets,
  addFusionNode,
  removeFusionNode,
  updateFusionNodeParams,
  fusionMaskFeather,
  setFusionMaskFeather,
  fusionMaskInverted,
  setFusionMaskInverted,
  fusionMaskPoints,
  addRotoSplineMask,
  setNotice,
}: FusionWorkspaceProps) {
  const graph = project?.sequence?.node_graph ?? { nodes: [], connections: [], masks: [] };
  const nodes: CompositingNode[] = graph.nodes;
  const connections: NodeConnection[] = graph.connections;
  const selectedNode = nodes.find((n: CompositingNode) => n.id === selectedNodeId);

  const handleCanvasMouseDown = (e: MouseEvent<HTMLDivElement>) => {
    if ((e.target as HTMLElement).closest(".fusion-node-card") || (e.target as HTMLElement).closest(".socket-port")) {
      return;
    }
    setSelectedNodeId(null);
    setConnectingSocket(null);
  };

  const handleNodeDragStart = (e: MouseEvent, node: CompositingNode) => {
    e.stopPropagation();
    setSelectedNodeId(node.id);
    setDraggingNodeId(node.id);
    setNodeDragOffset({
      x: e.clientX - node.pos_x,
      y: e.clientY - node.pos_y,
    });
  };

  const handleCanvasMouseMove = (e: MouseEvent<HTMLDivElement>) => {
    if (draggingNodeId == null) return;
    const newX = Math.max(20, Math.round(e.clientX - nodeDragOffset.x));
    const newY = Math.max(20, Math.round(e.clientY - nodeDragOffset.y));
    void updateFusionNodePos(draggingNodeId, newX, newY);
  };

  const handleCanvasMouseUp = () => {
    setDraggingNodeId(null);
  };

  const handleSocketClick = (e: MouseEvent, nodeId: number, socket: string, isOutput: boolean) => {
    e.stopPropagation();
    if (!connectingSocket) {
      setConnectingSocket({ nodeId, socket, isOutput });
      setNotice(`Connecting from ${socket}... click a target socket`);
    } else {
      if (connectingSocket.nodeId === nodeId) {
        setConnectingSocket(null);
        return;
      }
      // One must be output and the other input
      if (connectingSocket.isOutput !== isOutput) {
        const fromNode = connectingSocket.isOutput ? connectingSocket.nodeId : nodeId;
        const fromSock = connectingSocket.isOutput ? connectingSocket.socket : socket;
        const toNode = connectingSocket.isOutput ? nodeId : connectingSocket.nodeId;
        const toSock = connectingSocket.isOutput ? socket : connectingSocket.socket;
        void connectFusionSockets(fromNode, fromSock, toNode, toSock);
        setNotice("Connected sockets");
      } else {
        setNotice("⚠️ Connect an output socket to an input socket");
      }
      setConnectingSocket(null);
    }
  };

  const initStarterTree = async () => {
    await addFusionNode("media_source", "MediaIn1", 60, 160, { media_id: media[0]?.id ?? 1 });
    await addFusionNode("keyer", "ChromaKey1", 280, 160, { color: "#00ff00", tolerance: 40.0 });
    await addFusionNode("transform", "Transform1", 500, 160, { scale: 1.0, x: 0, y: 0 });
    await addFusionNode("merge", "Merge1", 720, 160, { blend_mode: "normal", opacity: 1.0 });
    await addFusionNode("output_sink", "MediaOut1", 940, 160);
    setNotice("Initialized Fusion Starter Node Tree");
  };

  return (
    <main className="workspace-page fusion-workspace-page">
      <header className="workspace-sub-header fusion-workspace-header" style={{ height: "36px", minHeight: "36px", padding: "0 12px" }}>
        <div className="scopes-title-group" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          <strong style={{ fontSize: "11px", letterSpacing: "0.06em", color: "var(--text-secondary)" }}>FUSION NODE COMPOSITING</strong>
          <span style={{ fontSize: "10px", color: "var(--text-muted)" }}>
            ({nodes.length} nodes · {connections.length} links)
          </span>
        </div>

        <div className="fusion-header-actions" style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <div className="fusion-add-buttons">
            <span className="fusion-add-label">+ Add Node:</span>
            <InteriorButton className="fusion-tool-btn" onClick={() => void addFusionNode("merge")}>Merge</InteriorButton>
            <InteriorButton className="fusion-tool-btn" onClick={() => void addFusionNode("transform")}>Transform</InteriorButton>
            <InteriorButton className="fusion-tool-btn" onClick={() => void addFusionNode("keyer")}>ChromaKey</InteriorButton>
            <InteriorButton className="fusion-tool-btn" onClick={() => void addFusionNode("mask")}>RotoMask</InteriorButton>
            <InteriorButton className="fusion-tool-btn" onClick={() => void addFusionNode("particles")}>Particles</InteriorButton>
            <InteriorButton className="fusion-tool-btn" onClick={() => void addFusionNode("media_source")}>MediaIn</InteriorButton>
            <InteriorButton className="fusion-tool-btn" onClick={() => void addFusionNode("output_sink")}>MediaOut</InteriorButton>
          </div>
          {nodes.length === 0 && (
            <InteriorButton className="primary-action-btn" onClick={() => void initStarterTree()} style={{ padding: "3px 10px", fontSize: "11px" }}>
              ✨ Init Tree
            </InteriorButton>
          )}
        </div>
      </header>

      <div className="fusion-body">
        {/* Main Node Graph Canvas */}
        <div
          className="fusion-canvas-container"
          onMouseDown={handleCanvasMouseDown}
          onMouseMove={handleCanvasMouseMove}
          onMouseUp={handleCanvasMouseUp}
        >
          <svg className="fusion-wires-layer">
            <defs>
              <linearGradient id="wireGrad" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#be185d" />
                <stop offset="100%" stopColor="#a855f7" />
              </linearGradient>
              <linearGradient id="maskWireGrad" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#f59e0b" />
                <stop offset="100%" stopColor="#ec4899" />
              </linearGradient>
            </defs>

            {/* Render SVG Bezier Connections */}
            {connections.map((c: NodeConnection, i: number) => {
              const fromNode = nodes.find((n: CompositingNode) => n.id === c.from_node);
              const toNode = nodes.find((n: CompositingNode) => n.id === c.to_node);
              if (!fromNode || !toNode) return null;

              const x1 = fromNode.pos_x + 150;
              const y1 = fromNode.pos_y + 40;
              const x2 = toNode.pos_x;
              const y2 = toNode.pos_y + 40;
              const dx = Math.max(50, Math.abs(x2 - x1) * 0.5);
              const pathData = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
              const isMaskWire = c.to_socket === "mask" || c.from_socket === "mask";

              return (
                <g key={`${c.from_node}-${c.to_node}-${i}`} className="fusion-wire-group">
                  <path
                    d={pathData}
                    className="fusion-wire-hitbox"
                    onClick={() => void disconnectFusionSockets(c.from_node, c.from_socket, c.to_node, c.to_socket)}
                  />
                  <path
                    d={pathData}
                    className="fusion-wire"
                    stroke={isMaskWire ? "url(#maskWireGrad)" : "url(#wireGrad)"}
                    strokeWidth="2.5"
                    fill="none"
                  />
                </g>
              );
            })}
          </svg>

          {/* Render Node Cards */}
          {nodes.map((node: CompositingNode) => {
            const isSelected = selectedNodeId === node.id;
            const nodeIcons: Record<CompositingNodeKind, string> = {
              merge: "🔀",
              transform: "📐",
              keyer: "🟩",
              mask: "✂️",
              particles: "✨",
              media_source: "🎬",
              output_sink: "🖥️",
            };

            return (
              <div
                key={node.id}
                className={`fusion-node-card ${isSelected ? "selected" : ""} kind-${node.kind}`}
                style={{ left: `${node.pos_x}px`, top: `${node.pos_y}px` }}
                onClick={(e) => {
                  e.stopPropagation();
                  setSelectedNodeId(node.id);
                }}
              >
                <div
                  className="fusion-node-header"
                  onMouseDown={(e) => handleNodeDragStart(e, node)}
                >
                  <span className="node-icon">{nodeIcons[node.kind as CompositingNodeKind] || "🔷"}</span>
                  <strong className="node-title">{node.title}</strong>
                  <InteriorButton
                    type="button"
                    className="node-delete-btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      void removeFusionNode(node.id);
                    }}
                    title="Delete Node"
                  >
                    ×
                  </InteriorButton>
                </div>

                <div className="fusion-node-body">
                  {/* Left In sockets */}
                  <div className="node-sockets-left">
                    {node.kind === "merge" ? (
                      <>
                        <div
                          className={`socket-port in ${connectingSocket?.nodeId === node.id && connectingSocket?.socket === "fg" ? "active" : ""}`}
                          onClick={(e) => handleSocketClick(e, node.id, "fg", false)}
                          title="Foreground Input"
                        >
                          <span className="socket-dot fg" />
                          <small>FG</small>
                        </div>
                        <div
                          className={`socket-port in ${connectingSocket?.nodeId === node.id && connectingSocket?.socket === "bg" ? "active" : ""}`}
                          onClick={(e) => handleSocketClick(e, node.id, "bg", false)}
                          title="Background Input"
                        >
                          <span className="socket-dot bg" />
                          <small>BG</small>
                        </div>
                        <div
                          className={`socket-port in mask ${connectingSocket?.nodeId === node.id && connectingSocket?.socket === "mask" ? "active" : ""}`}
                          onClick={(e) => handleSocketClick(e, node.id, "mask", false)}
                          title="Mask Input"
                        >
                          <span className="socket-dot mask" />
                          <small>Mask</small>
                        </div>
                      </>
                    ) : node.kind !== "media_source" && node.kind !== "particles" ? (
                      <>
                        <div
                          className={`socket-port in ${connectingSocket?.nodeId === node.id && connectingSocket?.socket === "input" ? "active" : ""}`}
                          onClick={(e) => handleSocketClick(e, node.id, "input", false)}
                          title="Image Input"
                        >
                          <span className="socket-dot in" />
                          <small>In</small>
                        </div>
                        <div
                          className={`socket-port in mask ${connectingSocket?.nodeId === node.id && connectingSocket?.socket === "mask" ? "active" : ""}`}
                          onClick={(e) => handleSocketClick(e, node.id, "mask", false)}
                          title="Mask Input"
                        >
                          <span className="socket-dot mask" />
                          <small>Mask</small>
                        </div>
                      </>
                    ) : null}
                  </div>

                  {/* Right Out sockets */}
                  <div className="node-sockets-right">
                    {node.kind !== "output_sink" ? (
                      <div
                        className={`socket-port out ${connectingSocket?.nodeId === node.id && connectingSocket?.socket === "output" ? "active" : ""}`}
                        onClick={(e) => handleSocketClick(e, node.id, "output", true)}
                        title="Image Output"
                      >
                        <small>Out</small>
                        <span className="socket-dot out" />
                      </div>
                    ) : (
                      <span className="sink-label">Master Out</span>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {/* Right Inspector Sidebar for Selected Node */}
        <aside className="fusion-inspector-panel">
          {selectedNode ? (
            <div className="fusion-node-inspector">
              <div className="inspector-title-row">
                <h3>{selectedNode.title}</h3>
                <span className="node-kind-badge">{selectedNode.kind.toUpperCase()}</span>
              </div>

              <div className="inspector-controls">
                {selectedNode.kind === "keyer" && (
                  <div className="node-param-group">
                    <h4>Chroma Key Settings</h4>
                    <label className="param-field">
                      <span>Key Color:</span>
                      <input
                        type="color"
                        value={(selectedNode.params?.color as string) || "#00ff00"}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            color: e.target.value,
                          })
                        }
                      />
                    </label>
                    <label className="param-field">
                      <span>Tolerance: {((selectedNode.params?.tolerance as number) ?? 40).toFixed(0)}</span>
                      <input
                        type="range"
                        min="0"
                        max="100"
                        value={(selectedNode.params?.tolerance as number) ?? 40}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            tolerance: Number(e.target.value),
                          })
                        }
                      />
                    </label>
                    <label className="param-field">
                      <span>
                        Spill Suppression: {(((selectedNode.params?.spill_suppression as number) ?? 0.75) * 100).toFixed(0)}%
                      </span>
                      <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.05"
                        value={(selectedNode.params?.spill_suppression as number) ?? 0.75}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            spill_suppression: Number(e.target.value),
                          })
                        }
                      />
                    </label>
                  </div>
                )}

                {selectedNode.kind === "transform" && (
                  <div className="node-param-group">
                    <h4>Spatial Transform</h4>
                    <label className="param-field">
                      <span>Scale: {((selectedNode.params?.scale as number) ?? 1.0).toFixed(2)}x</span>
                      <input
                        type="range"
                        min="0.1"
                        max="3"
                        step="0.05"
                        value={(selectedNode.params?.scale as number) ?? 1.0}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            scale: Number(e.target.value),
                          })
                        }
                      />
                    </label>
                    <label className="param-field">
                      <span>X Offset: {((selectedNode.params?.x as number) ?? 0).toFixed(0)}px</span>
                      <input
                        type="range"
                        min="-500"
                        max="500"
                        value={(selectedNode.params?.x as number) ?? 0}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            x: Number(e.target.value),
                          })
                        }
                      />
                    </label>
                    <label className="param-field">
                      <span>Rotation: {((selectedNode.params?.rotation as number) ?? 0).toFixed(0)}°</span>
                      <input
                        type="range"
                        min="-180"
                        max="180"
                        value={(selectedNode.params?.rotation as number) ?? 0}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            rotation: Number(e.target.value),
                          })
                        }
                      />
                    </label>
                  </div>
                )}

                {selectedNode.kind === "merge" && (
                  <div className="node-param-group">
                    <h4>Multi-Layer Compositing</h4>
                    <label className="param-field">
                      <span>Blend Mode:</span>
                      <select
                        value={(selectedNode.params?.blend_mode as string) || "normal"}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            blend_mode: e.target.value,
                          })
                        }
                      >
                        <option value="normal">Normal (Source Over)</option>
                        <option value="multiply">Multiply</option>
                        <option value="screen">Screen</option>
                        <option value="add">Add</option>
                        <option value="overlay">Overlay</option>
                      </select>
                    </label>
                    <label className="param-field">
                      <span>
                        Foreground Opacity: {(((selectedNode.params?.opacity as number) ?? 1.0) * 100).toFixed(0)}%
                      </span>
                      <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.05"
                        value={(selectedNode.params?.opacity as number) ?? 1.0}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            opacity: Number(e.target.value),
                          })
                        }
                      />
                    </label>
                  </div>
                )}

                {selectedNode.kind === "mask" && (
                  <div className="node-param-group">
                    <h4>Roto-Spline Masking</h4>
                    <label className="param-field">
                      <span>Edge Feather: {fusionMaskFeather.toFixed(1)}px</span>
                      <input
                        type="range"
                        min="0"
                        max="30"
                        step="0.5"
                        value={fusionMaskFeather}
                        onChange={(e) => {
                          setFusionMaskFeather(Number(e.target.value));
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            feather: Number(e.target.value),
                          });
                        }}
                      />
                    </label>
                    <label className="checkbox-field">
                      <input
                        type="checkbox"
                        checked={fusionMaskInverted}
                        onChange={(e) => {
                          setFusionMaskInverted(e.target.checked);
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            inverted: e.target.checked,
                          });
                        }}
                      />
                      <span>Invert Mask Matte</span>
                    </label>
                    <InteriorButton
                      className="secondary-btn"
                      style={{ marginTop: "8px" }}
                      onClick={() =>
                        void addRotoSplineMask(fusionMaskPoints, fusionMaskFeather, true, fusionMaskInverted)
                      }
                    >
                      + Create Spline Polygon
                    </InteriorButton>
                  </div>
                )}

                {selectedNode.kind === "particles" && (
                  <div className="node-param-group">
                    <h4>Particle Emitter Simulator</h4>
                    <label className="param-field">
                      <span>Emission Speed: {((selectedNode.params?.speed as number) ?? 120).toFixed(0)} px/s</span>
                      <input
                        type="range"
                        min="20"
                        max="300"
                        value={(selectedNode.params?.speed as number) ?? 120}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            speed: Number(e.target.value),
                          })
                        }
                      />
                    </label>
                    <label className="param-field">
                      <span>Gravity Y: {((selectedNode.params?.gravity as number) ?? 50).toFixed(0)}</span>
                      <input
                        type="range"
                        min="-100"
                        max="200"
                        value={(selectedNode.params?.gravity as number) ?? 50}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            gravity: Number(e.target.value),
                          })
                        }
                      />
                    </label>
                  </div>
                )}

                {selectedNode.kind === "media_source" && (
                  <div className="node-param-group">
                    <h4>Source Media Feed</h4>
                    <label className="param-field">
                      <span>Media Asset:</span>
                      <select
                        value={(selectedNode.params?.media_id as number) || (media[0]?.id ?? 1)}
                        onChange={(e) =>
                          void updateFusionNodeParams(selectedNode.id, {
                            ...selectedNode.params,
                            media_id: Number(e.target.value),
                          })
                        }
                      >
                        {media.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.name}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                )}

                <InteriorButton
                  className="danger-button"
                  style={{ marginTop: "16px" }}
                  onClick={() => void removeFusionNode(selectedNode.id)}
                >
                  Remove Node
                </InteriorButton>
              </div>
            </div>
          ) : (
            <div className="fusion-empty-inspector">
              <span className="empty-icon">🔲</span>
              <p>Select any node on the graph canvas to inspect parameters and routing sockets.</p>
            </div>
          )}
        </aside>
      </div>
    </main>
  );
}
