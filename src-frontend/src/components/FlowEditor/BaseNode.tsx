import { memo } from 'react';
import { Handle, Position, type NodeProps, type Node } from '@xyflow/react';
import { Card } from '../ui/card';
import type { NodeMetadata, PortMetadata } from '../../types/nodes';

interface BaseNodeData extends Record<string, unknown> {
  label: string;
  metadata: NodeMetadata;
  parameters: Record<string, unknown>;
}

type BaseNodeType = Node<BaseNodeData>;

function BaseNode({ data }: NodeProps<BaseNodeType>) {
  const metadata = data.metadata;

  return (
    <Card className="min-w-[150px] bg-slate-800 border-slate-600">
      <div className="p-3">
        <div className="font-semibold text-sm text-white mb-2">{data.label}</div>

        {/* Input Ports */}
        {metadata.inputs.map((input: PortMetadata, idx: number) => (
          <Handle
            key={input.id}
            type="target"
            position={Position.Left}
            id={input.id}
            style={{ top: `${((idx + 1) * 100) / (metadata.inputs.length + 1)}%` }}
            className="w-3 h-3 bg-blue-500"
          />
        ))}

        {/* Output Ports */}
        {metadata.outputs.map((output: PortMetadata, idx: number) => (
          <Handle
            key={output.id}
            type="source"
            position={Position.Right}
            id={output.id}
            style={{ top: `${((idx + 1) * 100) / (metadata.outputs.length + 1)}%` }}
            className="w-3 h-3 bg-green-500"
          />
        ))}

        {/* Parameter count indicator */}
        {Object.keys(metadata.parameters).length > 0 && (
          <div className="text-xs text-slate-400 mt-2">
            {Object.keys(metadata.parameters).length} params
          </div>
        )}
      </div>
    </Card>
  );
}

export default memo(BaseNode);
