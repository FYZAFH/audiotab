export interface PortMetadata {
  id: string;
  name: string;
  data_type: string;
}

export interface NodeMetadata {
  id: string;
  name: string;
  category: string;
  inputs: PortMetadata[];
  outputs: PortMetadata[];
  parameters: Record<string, any>;
}

export interface GraphNode {
  id: string;
  type: string;
  position: { x: number; y: number };
  data: {
    label: string;
    metadata: NodeMetadata;
    parameters: Record<string, any>;
  };
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  sourceHandle: string;
  targetHandle: string;
}

export interface GraphJson {
  nodes: any[];
  edges: any[];
}

export interface PipelineStatus {
  id: string;
  state: string;
  error?: string;
}

export type PipelineAction = 'start' | 'stop' | 'pause';
