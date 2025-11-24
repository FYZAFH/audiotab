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

interface FlowState {
  nodes: FlowNode[];
  edges: Edge[];
  onNodesChange: (changes: NodeChange<FlowNode>[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (type: string, position: { x: number; y: number }, metadata: NodeMetadata) => void;
  deleteSelected: () => void;
  exportGraph: () => { nodes: unknown[]; edges: unknown[] };
}

export const useFlowStore = create<FlowState>((set, get) => ({
  nodes: [],
  edges: [],

  onNodesChange: (changes) => {
    set({
      nodes: applyNodeChanges(changes, get().nodes) as FlowNode[],
    });
  },

  onEdgesChange: (changes) => {
    set({
      edges: applyEdgeChanges(changes, get().edges),
    });
  },

  onConnect: (connection) => {
    set({
      edges: addEdge(connection, get().edges),
    });
  },

  addNode: (type, position, metadata) => {
    const newNode: FlowNode = {
      id: `${type}-${Date.now()}`,
      type: 'custom',
      position,
      data: {
        label: metadata.name,
        metadata,
        parameters: {},
      },
    };
    set({ nodes: [...get().nodes, newNode] });
  },

  deleteSelected: () => {
    const { nodes, edges } = get();
    set({
      nodes: nodes.filter((n) => !n.selected),
      edges: edges.filter((e) => !e.selected),
    });
  },

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
