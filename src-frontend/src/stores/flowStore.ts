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
      saveHistory(state);
      return { nodes: newNodes };
    });
  },

  onEdgesChange: (changes) => {
    set((state) => {
      const newEdges = applyEdgeChanges(changes, state.edges);
      saveHistory(state);
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

  deleteSelected: () => {
    set((state) => {
      saveHistory(state);
      return {
        nodes: state.nodes.filter((n) => !n.selected),
        edges: state.edges.filter((e) => !e.selected),
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
        parameters: n.data.parameters,
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
