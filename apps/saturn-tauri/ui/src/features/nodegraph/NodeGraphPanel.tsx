import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CompositingNode, NodeGraph, NodeConnection } from "../../engine/playbackEngine";
import { InteriorButton } from "../../components/interior/interior-button";
import { Icon } from "../../components/Icon";
import { NODE_KINDS, type NodeKind } from "../../app/useNodeGraph";

export interface NodeGraphPanelProps {
  graph: NodeGraph | null;
  onAddNode: (kind: NodeKind) => void;
  onRemoveNode: (nodeId: number) => void;
  onConnect: (
    fromNode: number,
    fromSocket: string,
    toNode: number,
    toSocket: string,
  ) => void;
  onDisconnect: (connection: NodeConnection) => void;
  /** Commits an enabled toggle or a completed node drag to the engine. */
  onUpdateNode: (nodeId: number, patch: { enabled?: boolean; pos_x?: number; pos_y?: number }) => void;
  onAddMask: () => void;
}

const NODE_WIDTH = 132;
const NODE_HEIGHT = 52;
const CANVAS_PADDING = 24;

type DragState = {
  nodeId: number;
  offsetX: number;
  offsetY: number;
};

const kindTitle = (kind: string) =>
  kind
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");

/**
 * Fusion-style compositing graph for the sequence: draggable node cards on a
 * canvas, socket connections drawn as SVG lines, and a roto-spline mask list.
 * Node positions are engine state (`nodegraph.update_node`), so drags and
 * toggles commit through it and survive undo.
 */
export function NodeGraphPanel({
  graph,
  onAddNode,
  onRemoveNode,
  onConnect,
  onDisconnect,
  onUpdateNode,
  onAddMask,
}: NodeGraphPanelProps) {
  const nodes = graph?.nodes ?? [];
  const connections = graph?.connections ?? [];
  const masks = graph?.masks ?? [];

  const [positions, setPositions] = useState<Record<number, { x: number; y: number }>>({});
  const [drag, setDrag] = useState<DragState | null>(null);
  const [addKind, setAddKind] = useState<NodeKind>("merge");
  const [fromNode, setFromNode] = useState<number | "">("");
  const [toNode, setToNode] = useState<number | "">("");
  const [fromSocket, setFromSocket] = useState("output");
  const [toSocket, setToSocket] = useState("input");
  const canvasRef = useRef<HTMLDivElement | null>(null);

  // Re-seed local drag positions whenever the engine graph changes shape.
  useEffect(() => {
    if (drag) return; // never clobber an in-flight drag
    setPositions(
      Object.fromEntries(nodes.map((node) => [node.id, { x: node.pos_x, y: node.pos_y }])),
    );
    // `nodes` derives from `graph`; re-seed on graph refreshes only.
  }, [graph]); // eslint-disable-line react-hooks/exhaustive-deps

  const nodePos = useCallback(
    (node: CompositingNode) => positions[node.id] ?? { x: node.pos_x, y: node.pos_y },
    [positions],
  );

  const canvasSize = useMemo(() => {
    let width = 320;
    let height = 200;
    for (const node of nodes) {
      const { x, y } = nodePos(node);
      width = Math.max(width, x + NODE_WIDTH + CANVAS_PADDING);
      height = Math.max(height, y + NODE_HEIGHT + CANVAS_PADDING);
    }
    return { width, height };
  }, [nodes, nodePos]);

  const onPointerDown = (node: CompositingNode, event: React.PointerEvent<HTMLDivElement>) => {
    const { x, y } = nodePos(node);
    setDrag({ nodeId: node.id, offsetX: event.clientX - x, offsetY: event.clientY - y });
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!drag) return;
    setPositions((current) => ({
      ...current,
      [drag.nodeId]: {
        x: Math.max(0, event.clientX - drag.offsetX),
        y: Math.max(0, event.clientY - drag.offsetY),
      },
    }));
  };

  const onPointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!drag) return;
    event.currentTarget.releasePointerCapture(event.pointerId);
    const node = nodes.find((n) => n.id === drag.nodeId);
    const pos = positions[drag.nodeId];
    setDrag(null);
    if (node && pos && (pos.x !== node.pos_x || pos.y !== node.pos_y)) {
      onUpdateNode(node.id, { pos_x: pos.x, pos_y: pos.y });
    }
  };

  const handleConnect = () => {
    if (fromNode === "" || toNode === "") {
      return;
    }
    onConnect(Number(fromNode), fromSocket.trim() || "output", Number(toNode), toSocket.trim() || "input");
  };

  const nodeById = (id: number) => nodes.find((n) => n.id === id);

  return (
    <section className="nodegraph-panel" aria-label="Node graph">
      <div className="nodegraph-toolbar">
        <select
          className="nodegraph-kind-select"
          aria-label="Node kind"
          value={addKind}
          onChange={(event) => setAddKind(event.currentTarget.value as NodeKind)}
        >
          {NODE_KINDS.map((kind) => (
            <option key={kind} value={kind}>
              {kindTitle(kind)}
            </option>
          ))}
        </select>
        <InteriorButton size="sm" onClick={() => onAddNode(addKind)} title="Add a compositing node">
          <Icon name="add" /> Node
        </InteriorButton>
        <InteriorButton size="sm" onClick={onAddMask} title="Add a four-point roto-spline mask">
          <Icon name="pen" /> Mask
        </InteriorButton>
      </div>

      <div className="nodegraph-connect">
        <select
          className="nodegraph-select"
          aria-label="From node"
          value={fromNode}
          onChange={(event) =>
            setFromNode(event.currentTarget.value === "" ? "" : Number(event.currentTarget.value))
          }
          disabled={nodes.length === 0}
        >
          <option value="">From…</option>
          {nodes.map((node) => (
            <option key={node.id} value={node.id}>
              {node.title}
            </option>
          ))}
        </select>
        <input
          className="nodegraph-socket"
          aria-label="From socket"
          value={fromSocket}
          onChange={(event) => setFromSocket(event.currentTarget.value)}
        />
        <span aria-hidden="true">→</span>
        <select
          className="nodegraph-select"
          aria-label="To node"
          value={toNode}
          onChange={(event) =>
            setToNode(event.currentTarget.value === "" ? "" : Number(event.currentTarget.value))
          }
          disabled={nodes.length === 0}
        >
          <option value="">To…</option>
          {nodes.map((node) => (
            <option key={node.id} value={node.id}>
              {node.title}
            </option>
          ))}
        </select>
        <input
          className="nodegraph-socket"
          aria-label="To socket"
          value={toSocket}
          onChange={(event) => setToSocket(event.currentTarget.value)}
        />
        <InteriorButton
          size="sm"
          onClick={handleConnect}
          disabled={fromNode === "" || toNode === ""}
          title="Connect the two nodes"
        >
          <Icon name="link" />
        </InteriorButton>
      </div>

      <div className="nodegraph-canvas-wrap">
        <div
          ref={canvasRef}
          className="nodegraph-canvas"
          style={{ width: canvasSize.width, height: canvasSize.height }}
        >
          {nodes.length === 0 ? (
            <p className="nodegraph-empty">No nodes yet — add one from the toolbar.</p>
          ) : (
            <svg className="nodegraph-wires" width={canvasSize.width} height={canvasSize.height} aria-hidden="true">
              {connections.map((conn) => {
                const from = nodeById(conn.from_node);
                const to = nodeById(conn.to_node);
                if (!from || !to) return null;
                const a = nodePos(from);
                const b = nodePos(to);
                return (
                  <line
                    key={`${conn.from_node}.${conn.from_socket}-${conn.to_node}.${conn.to_socket}`}
                    x1={a.x + NODE_WIDTH}
                    y1={a.y + NODE_HEIGHT / 2}
                    x2={b.x}
                    y2={b.y + NODE_HEIGHT / 2}
                    className="nodegraph-wire"
                  />
                );
              })}
            </svg>
          )}
          {nodes.map((node) => {
            const { x, y } = nodePos(node);
            return (
              <div
                key={node.id}
                className={`nodegraph-node${node.enabled === false ? " is-disabled" : ""}${drag?.nodeId === node.id ? " is-dragging" : ""}`}
                style={{ left: x, top: y }}
                onPointerDown={(event) => onPointerDown(node, event)}
                onPointerMove={onPointerMove}
                onPointerUp={onPointerUp}
              >
                <div className="nodegraph-node-head">
                  <span className="nodegraph-node-title">{node.title}</span>
                  <button
                    type="button"
                    className="nodegraph-node-toggle"
                    title={node.enabled === false ? "Enable this node" : "Disable this node"}
                    onClick={(event) => {
                      event.stopPropagation();
                      onUpdateNode(node.id, { enabled: node.enabled === false });
                    }}
                  >
                    {node.enabled === false ? "○" : "●"}
                  </button>
                  <button
                    type="button"
                    className="nodegraph-node-remove"
                    title="Remove this node"
                    onClick={(event) => {
                      event.stopPropagation();
                      onRemoveNode(node.id);
                    }}
                  >
                    <Icon name="close" />
                  </button>
                </div>
                <div className="nodegraph-node-body">
                  <span className="nodegraph-node-kind">{kindTitle(node.kind)}</span>
                  <span className="nodegraph-node-io">in · out</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {connections.length > 0 ? (
        <ul className="nodegraph-connections">
          {connections.map((conn) => {
            const from = nodeById(conn.from_node);
            const to = nodeById(conn.to_node);
            return (
              <li key={`${conn.from_node}.${conn.from_socket}-${conn.to_node}.${conn.to_socket}`}>
                <span>
                  {from?.title ?? conn.from_node}.{conn.from_socket} → {to?.title ?? conn.to_node}.
                  {conn.to_socket}
                </span>
                <button
                  type="button"
                  className="nodegraph-conn-remove"
                  title="Disconnect"
                  onClick={() => onDisconnect(conn)}
                >
                  <Icon name="unlink" />
                </button>
              </li>
            );
          })}
        </ul>
      ) : null}

      {masks.length > 0 ? (
        <div className="nodegraph-masks">
          <span className="nodegraph-masks-label">Masks</span>
          <ul>
            {masks.map((mask) => (
              <li key={mask.id}>
                {mask.name} · {mask.points.length} pts
                {mask.feather ? ` · feather ${mask.feather}` : ""}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
}
