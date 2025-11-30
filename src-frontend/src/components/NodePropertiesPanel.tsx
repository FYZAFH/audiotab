import { useDeviceProfiles } from '../hooks/useDeviceManagement';
import { useFlowStore } from '../stores/flowStore';

export function NodePropertiesPanel() {
  const { nodes } = useFlowStore();
  const { data: deviceProfiles } = useDeviceProfiles();

  // Get selected node (if any)
  const selectedNode = nodes.find(n => n.selected);

  if (!selectedNode) {
    return (
      <div className="w-64 bg-slate-800 border-l border-slate-700 p-4">
        <p className="text-slate-400 text-sm">No node selected</p>
      </div>
    );
  }

  const updateNodeData = (nodeId: string, newData: any) => {
    useFlowStore.getState().updateNodeData(nodeId, newData);
  };

  return (
    <div className="w-64 bg-slate-800 border-l border-slate-700 p-4 overflow-auto">
      <h3 className="text-white font-semibold mb-3">Node Properties</h3>

      <div className="space-y-3">
        <div>
          <label className="text-xs text-slate-400 block mb-1">Node Type</label>
          <div className="text-sm text-white">{selectedNode.data?.metadata?.name || 'Unknown'}</div>
        </div>

        {/* Device selection for AudioSourceNode */}
        {selectedNode.type === 'custom' && selectedNode.data?.metadata?.id === 'AudioSourceNode' && (
          <div>
            <label className="text-xs text-slate-400 block mb-1">Audio Device</label>
            <select
              className="w-full px-2 py-1 bg-slate-700 border border-slate-600 rounded text-sm text-white"
              value={((selectedNode.data as any).device_profile_id as string) || ''}
              onChange={(e) => {
                updateNodeData(selectedNode.id, {
                  ...selectedNode.data,
                  device_profile_id: e.target.value,
                } as any);
              }}
            >
              <option value="">-- Select Device --</option>
              {deviceProfiles?.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.alias}
                </option>
              ))}
            </select>
            {(selectedNode.data as any).device_profile_id && (
              <p className="text-xs text-slate-500 mt-1">
                Device profile selected
              </p>
            )}
          </div>
        )}

        {/* Other node parameters could be rendered here */}
        <div className="pt-2 border-t border-slate-700">
          <p className="text-xs text-slate-500">
            Additional parameters can be configured here
          </p>
        </div>
      </div>
    </div>
  );
}
