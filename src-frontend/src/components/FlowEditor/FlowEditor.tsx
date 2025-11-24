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

  const onDrop = useCallback((event: React.DragEvent) => {
    event.preventDefault();

    const metadata: NodeMetadata = JSON.parse(event.dataTransfer.getData('application/reactflow'));

    const position = screenToFlowPosition({
      x: event.clientX,
      y: event.clientY,
    });

    useFlowStore.getState().addNode(metadata.id, position, metadata);
  }, [screenToFlowPosition]);

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  return (
    <div ref={reactFlowWrapper} className="w-full h-full bg-slate-900" onDrop={onDrop} onDragOver={onDragOver}>
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
