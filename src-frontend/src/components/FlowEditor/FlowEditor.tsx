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

    const data = event.dataTransfer.getData('application/reactflow') ||
                 event.dataTransfer.getData('text/plain');
    if (!data) {
      return;
    }

    let metadata: NodeMetadata;
    try {
      metadata = JSON.parse(data);
    } catch (e) {
      console.error('Failed to parse dropped node payload:', e);
      return;
    }

    const position = screenToFlowPosition({
      x: event.clientX,
      y: event.clientY,
    });

    useFlowStore.getState().addNode(metadata.id, position, metadata);
  }, [screenToFlowPosition]);

  return (
    <div
      ref={reactFlowWrapper}
      className="w-full h-full bg-slate-900"
      onDrop={onDrop}
      onDragOver={onDragOver}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
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
