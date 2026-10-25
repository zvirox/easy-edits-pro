import { useCallback, useMemo } from "react";
import type { NodeGraph, RotoSplineMask } from "../engine/playbackEngine";
import type { ProjectDocument } from "./projectTypes";
import { PANEL_OPEN_EVENT } from "../features/dock/panelEvents";

export const NODEGRAPH_PANEL_ID = "nodegraph";

/** The compositing node kinds the engine's schema supports. */
export const NODE_KINDS = [
  "merge",
  "transform",
  "keyer",
  "mask",
  "particles",
  "media_source",
  "output_sink",
] as const;

export type NodeKind = (typeof NODE_KINDS)[number];

export interface UseNodeGraphOptions {
  project: ProjectDocument["project"] | null;
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  refreshProject: () => Promise<unknown>;
  setNotice: (message: string) => void;
}

export interface NodePatch {
  enabled?: boolean;
  pos_x?: number;
  pos_y?: number;
}

export interface NodeGraphOperations {
  /** The sequence's compositing graph; null until a node or mask exists. */
  graph: NodeGraph | null;
  addNode: (kind: NodeKind) => Promise<void>;
  removeNode: (nodeId: number) => Promise<void>;
  connectNodes: (
    fromNode: number,
    fromSocket: string,
    toNode: number,
    toSocket: string,
  ) => Promise<void>;
  disconnectNodes: (
    fromNode: number,
    fromSocket: string,
    toNode: number,
    toSocket: string,
  ) => Promise<void>;
  updateNode: (nodeId: number, patch: NodePatch) => Promise<void>;
  addMask: () => Promise<void>;
  /** Handles the nodegraph menu commands that need UI gestures; false = not ours. */
  handleMenuCommand: (id: string) => Promise<boolean>;
  openPanel: () => void;
}

const kindLabel = (kind: string) =>
  kind
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");

/** A fresh four-point rectangular matte, laid out like the engine test's. */
const defaultMask = (name: string): RotoSplineMask => ({
  id: 0,
  name,
  closed: true,
  inverted: false,
  feather: 8,
  opacity: 1,
  points: [
    { x: 10, y: 10, handle_in: [0, 0], handle_out: [0, 0] },
    { x: 100, y: 10, handle_in: [0, 0], handle_out: [0, 0] },
    { x: 100, y: 100, handle_in: [0, 0], handle_out: [0, 0] },
    { x: 10, y: 100, handle_in: [0, 0], handle_out: [0, 0] },
  ],
});

export function useNodeGraph({
  project,
  runCommand,
  refreshProject,
  setNotice,
}: UseNodeGraphOptions): NodeGraphOperations {
  const graph = useMemo(() => project?.sequence.node_graph ?? null, [project?.sequence.node_graph]);
  const nodes = graph?.nodes ?? [];
  const masks = graph?.masks ?? [];

  const addNode = useCallback(
    async (kind: NodeKind) => {
      try {
        const res = await runCommand<{ node_id: number }>("nodegraph.add_node", {
          kind,
          title: `${kindLabel(kind)} ${nodes.length + 1}`,
          pos_x: 40 + (nodes.length % 6) * 140,
          pos_y: 40 + (nodes.length % 4) * 90,
        });
        await refreshProject();
        setNotice(`Node ${res?.node_id ?? ""} added`);
      } catch (error) {
        setNotice(String(error));
      }
    },
    [nodes.length, runCommand, refreshProject, setNotice],
  );

  const removeNode = useCallback(
    async (nodeId: number) => {
      try {
        await runCommand("nodegraph.remove_node", { node_id: nodeId });
        await refreshProject();
        setNotice("Node removed");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const connectNodes = useCallback(
    async (fromNode: number, fromSocket: string, toNode: number, toSocket: string) => {
      if (fromNode === toNode) {
        setNotice("A node cannot connect to itself");
        return;
      }
      try {
        await runCommand("nodegraph.connect", {
          from_node: fromNode,
          from_socket: fromSocket,
          to_node: toNode,
          to_socket: toSocket,
        });
        await refreshProject();
        setNotice("Nodes connected");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const disconnectNodes = useCallback(
    async (fromNode: number, fromSocket: string, toNode: number, toSocket: string) => {
      try {
        await runCommand("nodegraph.disconnect", {
          from_node: fromNode,
          from_socket: fromSocket,
          to_node: toNode,
          to_socket: toSocket,
        });
        await refreshProject();
        setNotice("Connection removed");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const updateNode = useCallback(
    async (nodeId: number, patch: NodePatch) => {
      try {
        await runCommand("nodegraph.update_node", { node_id: nodeId, ...patch });
        await refreshProject();
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const addMask = useCallback(async () => {
    try {
      const res = await runCommand<{ mask_id: number }>("nodegraph.add_mask", {
        mask: defaultMask(`Mask ${masks.length + 1}`),
      });
      await refreshProject();
      setNotice(`Mask ${res?.mask_id ?? ""} added`);
    } catch (error) {
      setNotice(String(error));
    }
  }, [masks.length, runCommand, refreshProject, setNotice]);

  const openPanel = useCallback(() => {
    const announce = () =>
      window.dispatchEvent(new CustomEvent(PANEL_OPEN_EVENT, { detail: { id: NODEGRAPH_PANEL_ID } }));
    announce();
    // The panel is hosted by the Edit workspace; re-announce in case the
    // caller switches to it in the same tick (matches the Captions panel).
    window.setTimeout(announce, 120);
  }, []);

  /** Menu entry points for the shipped nodegraph commands (panel gestures). */
  const handleMenuCommand = useCallback(
    async (id: string): Promise<boolean> => {
      if (id === "nodegraph.add_node") {
        await addNode("merge");
        return true;
      }
      if (id === "nodegraph.add_mask") {
        await addMask();
        return true;
      }
      if (
        id === "nodegraph.remove_node" ||
        id === "nodegraph.connect" ||
        id === "nodegraph.disconnect" ||
        id === "nodegraph.update_node"
      ) {
        // These pick a specific node/socket, so they are panel gestures.
        openPanel();
        setNotice("Use the Nodes panel to manage node connections");
        return true;
      }
      return false;
    },
    [addNode, addMask, openPanel, setNotice],
  );

  return {
    graph,
    addNode,
    removeNode,
    connectNodes,
    disconnectNodes,
    updateNode,
    addMask,
    handleMenuCommand,
    openPanel,
  };
}
