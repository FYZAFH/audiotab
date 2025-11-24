import { useNodeRegistry } from '../../hooks/useTauriCommands';
import { Card } from '../ui/card';
import { Separator } from '../ui/separator';
import type { NodeMetadata } from '../../types/nodes';

export default function NodePalette() {
  const { data: nodes, isLoading } = useNodeRegistry();

  const onDragStart = (event: React.DragEvent, metadata: NodeMetadata) => {
    console.log('🎯 [NodePalette] onDragStart called for:', metadata.name);
    const payload = JSON.stringify(metadata);
    console.log('📦 [NodePalette] Payload:', payload.substring(0, 100) + '...');

    try {
      event.dataTransfer.setData('application/reactflow', payload);
      console.log('✅ [NodePalette] Set application/reactflow');
    } catch (e) {
      console.log('⚠️ [NodePalette] Failed to set application/reactflow:', e);
    }

    event.dataTransfer.setData('text/plain', payload);
    console.log('✅ [NodePalette] Set text/plain');
    event.dataTransfer.effectAllowed = 'move';
    console.log('✅ [NodePalette] Drag started successfully');
  };

  if (isLoading) {
    return (
      <div className="w-64 bg-slate-800 p-4">
        <div className="text-white">Loading nodes...</div>
      </div>
    );
  }

  const categories = [...new Set(nodes?.map((n) => n.category) || [])];

  return (
    <div className="w-64 bg-slate-800 p-4 overflow-y-auto">
      <h2 className="text-white font-bold text-lg mb-4">Node Palette</h2>
      {categories.map((category) => (
        <div key={category} className="mb-4">
          <h3 className="text-slate-300 font-semibold text-sm mb-2">{category}</h3>
          <div className="space-y-2">
            {nodes
              ?.filter((n) => n.category === category)
              .map((node) => (
                <Card
                  key={node.id}
                  draggable
                  onDragStart={(e) => onDragStart(e, node)}
                  className="p-3 cursor-move bg-slate-700 border-slate-600 hover:bg-slate-600"
                >
                  <div className="text-white text-sm pointer-events-none">{node.name}</div>
                  <div className="text-slate-400 text-xs mt-1 pointer-events-none">
                    {node.inputs.length} in, {node.outputs.length} out
                  </div>
                </Card>
              ))}
          </div>
          <Separator className="my-3 bg-slate-600" />
        </div>
      ))}
    </div>
  );
}
