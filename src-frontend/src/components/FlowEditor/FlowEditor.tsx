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
    console.log('🔄 [FlowEditor] onDragOver - drag in progress');
  }, []);

  const onDrop = useCallback((event: React.DragEvent) => {
    console.log('🎯 [FlowEditor] onDrop called');
    event.preventDefault();

    let data = event.dataTransfer.getData('application/reactflow');
    console.log('📦 [FlowEditor] application/reactflow data:', data ? data.substring(0, 50) + '...' : 'EMPTY');

    if (!data) {
      data = event.dataTransfer.getData('text/plain');
      console.log('📦 [FlowEditor] text/plain fallback data:', data ? data.substring(0, 50) + '...' : 'EMPTY');
    }

    if (!data) {
      console.error('❌ [FlowEditor] No data found in drop event!');
      console.log('📋 [FlowEditor] Available types:', event.dataTransfer.types);
      return;
    }

    let metadata: NodeMetadata;
    try {
      metadata = JSON.parse(data);
      console.log('✅ [FlowEditor] Parsed metadata:', metadata.name);
    } catch (e) {
      console.error('❌ [FlowEditor] Failed to parse dropped node payload:', data, e);
      return;
    }

    const position = screenToFlowPosition({
      x: event.clientX,
      y: event.clientY,
    });
    console.log('📍 [FlowEditor] Drop position:', position);

    useFlowStore.getState().addNode(metadata.id, position, metadata);
    console.log('✅ [FlowEditor] addNode called successfully');
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
