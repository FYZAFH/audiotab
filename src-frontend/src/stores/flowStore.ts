import { create } from 'zustand';
import { addEdge, applyNodeChanges, applyEdgeChanges } from '@xyflow/react';
import type { Node, Edge, Connection, NodeChange, EdgeChange } from '@xyflow/react';
import type { NodeMetadata } from '../types/nodes';

interface FlowNodeData extends Record<string, unknown> {
  label: string;
  metadata: NodeMetadata;
  parameters: Record<string, unknown>;
}

type FlowNode = Node<FlowNodeData>;

interface FlowSnapshot {
  nodes: FlowNode[];
  edges: Edge[];
}

interface FlowState {
  nodes: FlowNode[];
  edges: Edge[];
  history: FlowSnapshot[];
  historyIndex: number;
  onNodesChange: (changes: NodeChange<FlowNode>[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (type: string, position: { x: number; y: number }, metadata: NodeMetadata) => void;
  updateNodeData: (nodeId: string, newData: Partial<FlowNodeData>) => void;
  deleteSelected: () => void;
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
  exportGraph: () => { nodes: unknown[]; edges: unknown[] };
}

const saveHistory = (state: FlowState): void => {
  const snapshot = { nodes: state.nodes, edges: state.edges };
  const newHistory = state.history.slice(0, state.historyIndex + 1);
  newHistory.push(snapshot);
  state.history = newHistory.slice(-50); // Keep last 50
  state.historyIndex = state.history.length - 1;
};

export const useFlowStore = create<FlowState>((set, get) => ({
  nodes: [],
  edges: [],
  history: [{ nodes: [], edges: [] }],
  historyIndex: 0,

  onNodesChange: (changes) => {
    set((state) => {
      const newNodes = applyNodeChanges(changes, state.nodes) as FlowNode[];

      // Check if any nodes were removed
      const removedNodeIds = changes
        .filter((change) => change.type === 'remove')
        .map((change) => (change as any).id);

      // Filter out edges connected to removed nodes
      const newEdges = removedNodeIds.length > 0
        ? state.edges.filter(
            (edge) =>
              !removedNodeIds.includes(edge.source) &&
              !removedNodeIds.includes(edge.target)
          )
        : state.edges;

      // Only save history for meaningful changes (not during active dragging)
      const shouldSaveHistory = changes.some((change) =>
        change.type === 'add' ||
        change.type === 'remove' ||
        (change.type === 'position' && change.dragging === false)
      );

      if (shouldSaveHistory) {
        saveHistory(state);
      }

      return { nodes: newNodes, edges: newEdges };
    });
  },

  onEdgesChange: (changes) => {
    set((state) => {
      const newEdges = applyEdgeChanges(changes, state.edges);

      // Only save history for meaningful changes (not for selection changes)
      const shouldSaveHistory = changes.some((change) =>
        change.type === 'add' ||
        change.type === 'remove'
      );

      if (shouldSaveHistory) {
        saveHistory(state);
      }

      return { edges: newEdges };
    });
  },

  onConnect: (connection) => {
    set((state) => {
      const newEdges = addEdge(connection, state.edges);
      saveHistory(state);
      return { edges: newEdges };
    });
  },

  addNode: (type, position, metadata) => {
    set((state) => {
      const newNode: FlowNode = {
        id: `${type}-${Date.now()}`,
        type: 'custom',
        position,
        data: { label: metadata.name, metadata, parameters: {} },
      };
      saveHistory(state);
      return { nodes: [...state.nodes, newNode] };
    });
  },

  updateNodeData: (nodeId, newData) => {
    set((state) => {
      const updatedNodes = state.nodes.map((node) =>
        node.id === nodeId
          ? { ...node, data: { ...node.data, ...newData } }
          : node
      );
      saveHistory(state);
      return { nodes: updatedNodes };
    });
  },

  deleteSelected: () => {
    set((state) => {
      const selectedNodeIds = new Set(state.nodes.filter((n) => n.selected).map((n) => n.id));
      const hasSelection = selectedNodeIds.size > 0 || state.edges.some((e) => e.selected);

      if (hasSelection) {
        saveHistory(state);
      }

      const remainingNodes = state.nodes.filter((n) => !n.selected);
      const remainingEdges = state.edges.filter(
        (e) =>
          !e.selected &&
          !selectedNodeIds.has(e.source) &&
          !selectedNodeIds.has(e.target),
      );

      return {
        nodes: remainingNodes,
        edges: remainingEdges,
      };
    });
  },

  undo: () => {
    const { history, historyIndex } = get();
    if (historyIndex > 0) {
      const snapshot = history[historyIndex - 1];
      set({
        nodes: snapshot.nodes,
        edges: snapshot.edges,
        historyIndex: historyIndex - 1,
      });
    }
  },

  redo: () => {
    const { history, historyIndex } = get();
    if (historyIndex < history.length - 1) {
      const snapshot = history[historyIndex + 1];
      set({
        nodes: snapshot.nodes,
        edges: snapshot.edges,
        historyIndex: historyIndex + 1,
      });
    }
  },

  canUndo: () => get().historyIndex > 0,
  canRedo: () => get().historyIndex < get().history.length - 1,

  exportGraph: () => {
    const { nodes, edges } = get();
    return {
      nodes: nodes.map((n) => ({
        id: n.id,
        type: n.data.metadata.id,
        position: n.position,
        parameters: {
          ...n.data.parameters,
          // Include device_profile_id if present
          ...((n.data as any).device_profile_id ? { device_profile_id: (n.data as any).device_profile_id } : {}),
        },
      })),
      edges: edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle,
      })),
    };
  },
}));
