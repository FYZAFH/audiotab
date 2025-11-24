import { useCallback, useRef } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  ConnectionMode,
  useReactFlow,
  ReactFlowProvider,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useFlowStore } from '../../stores/flowStore';
import BaseNode from './BaseNode';
import type { NodeMetadata } from '../../types/nodes';

const nodeTypes = {
  custom: BaseNode,
};

function FlowEditorInner() {
  const { nodes, edges, onNodesChange, onEdgesChange, onConnect } = useFlowStore();
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  const { screenToFlowPosition } = useReactFlow();

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback((event: React.DragEvent) => {
    event.preventDefault();

    let data = event.dataTransfer.getData('application/reactflow');
    if (!data) {
      data = event.dataTransfer.getData('text/plain');
    }
    if (!data) {
      return;
    }

    let metadata: NodeMetadata;
    try {
      metadata = JSON.parse(data);
    } catch {
      console.error('Failed to parse dropped node payload:', data);
      return;
    }

    const position = screenToFlowPosition({
      x: event.clientX,
      y: event.clientY,
    });

    useFlowStore.getState().addNode(metadata.id, position, metadata);
  }, [screenToFlowPosition]);

  return (
    <div ref={reactFlowWrapper} className="w-full h-full bg-slate-900">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onDrop={onDrop}
        onDragOver={onDragOver}
        nodeTypes={nodeTypes}
        connectionMode={ConnectionMode.Loose}
        fitView
      >
        <Background />
        <Controls />
        <MiniMap />
      </ReactFlow>
    </div>
  );
}

export default function FlowEditor() {
  return (
    <ReactFlowProvider>
      <FlowEditorInner />
    </ReactFlowProvider>
  );
}
