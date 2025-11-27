import { useState } from 'react';
import { Button } from '../components/ui/button';
import { Card } from '../components/ui/card';
import FlowEditor from '../components/FlowEditor/FlowEditor';
import NodePalette from '../components/NodePalette/NodePalette';
import { useFlowStore } from '../stores/flowStore';
import { useDeployGraph, useKernelStatus } from '../hooks/useTauriCommands';
import { usePipelineStatusEvents } from '../hooks/useTauriEvents';
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';
import { AlertCircle, Lock, Unlock } from 'lucide-react';

export function ProcessConfiguration() {
  useKeyboardShortcuts();
  const [lastStatus, setLastStatus] = useState<string>('');
  const [editMode, setEditMode] = useState(false);

  const exportGraph = useFlowStore((state) => state.exportGraph);
  const undo = useFlowStore((state) => state.undo);
  const redo = useFlowStore((state) => state.redo);
  const canUndo = useFlowStore((state) => state.canUndo());
  const canRedo = useFlowStore((state) => state.canRedo());

  const deployMutation = useDeployGraph();
  const { data: kernelStatus } = useKernelStatus();

  usePipelineStatusEvents((event) => {
    setLastStatus(`Pipeline ${event.id}: ${event.state}`);
  });

  const isKernelRunning = kernelStatus?.status === 'Running' || kernelStatus?.status === 'Initializing';
  const canEdit = editMode && !isKernelRunning;

  const handleDeploy = async () => {
    const graph = exportGraph();
    try {
      const pipelineId = await deployMutation.mutateAsync(graph);
      console.log('Deployed pipeline:', pipelineId);
      setLastStatus(`Successfully deployed pipeline: ${pipelineId}`);
    } catch (error) {
      console.error('Deploy failed:', error);
      setLastStatus(`Deploy failed: ${error}`);
    }
  };

  const toggleEditMode = () => {
    if (isKernelRunning) {
      setLastStatus('Cannot edit while kernel is running. Stop the kernel first.');
      return;
    }
    setEditMode(!editMode);
  };

  return (
    <div className="flex flex-col h-full">
      {/* Configuration Toolbar */}
      <div className="h-12 bg-slate-800 border-b border-slate-700 flex items-center px-4 gap-2">
        {/* Edit Mode Toggle */}
        <Button
          onClick={toggleEditMode}
          variant={editMode ? 'default' : 'outline'}
          size="sm"
          className="gap-2"
          disabled={isKernelRunning}
        >
          {editMode ? <Unlock className="h-4 w-4" /> : <Lock className="h-4 w-4" />}
          {editMode ? 'Edit Mode' : 'View Only'}
        </Button>

        {/* Undo/Redo - Only enabled in edit mode */}
        <div className="flex gap-2">
          <Button
            onClick={undo}
            disabled={!canEdit || !canUndo}
            variant="outline"
            size="sm"
          >
            Undo
          </Button>
          <Button
            onClick={redo}
            disabled={!canEdit || !canRedo}
            variant="outline"
            size="sm"
          >
            Redo
          </Button>
        </div>

        <div className="flex-1" />

        {/* Kernel Status Indicator */}
        <div className="flex items-center gap-2 px-3 py-1 bg-slate-700 rounded text-sm">
          <span className="text-slate-400">Kernel:</span>
          <span className={
            kernelStatus?.status === 'Running' ? 'text-green-400' :
            kernelStatus?.status === 'Initializing' ? 'text-yellow-400' :
            kernelStatus?.status === 'Error' ? 'text-red-400' :
            'text-slate-400'
          }>
            {kernelStatus?.status || 'Unknown'}
          </span>
        </div>

        {/* Deploy Button */}
        <Button
          onClick={handleDeploy}
          disabled={deployMutation.isPending || !editMode}
          size="sm"
        >
          {deployMutation.isPending ? 'Deploying...' : 'Deploy Configuration'}
        </Button>
      </div>

      {/* Warning Banner when Kernel is Running */}
      {isKernelRunning && (
        <div className="bg-yellow-900/20 border-b border-yellow-700/50 px-4 py-2 flex items-center gap-2">
          <AlertCircle className="h-4 w-4 text-yellow-500" />
          <span className="text-yellow-200 text-sm">
            Configuration is locked while kernel is running. Stop the kernel to make changes.
          </span>
        </div>
      )}

      {/* Editor Content */}
      <div className="flex flex-1 overflow-hidden">
        <NodePalette />
        <div className="flex-1 relative">
          <FlowEditor />
          {/* Overlay when not in edit mode */}
          {!canEdit && (
            <div className="absolute inset-0 bg-slate-900/50 flex items-center justify-center pointer-events-none">
              {!editMode && !isKernelRunning && (
                <Card className="bg-slate-800 border-slate-700 p-6 pointer-events-auto">
                  <div className="text-center space-y-3">
                    <Lock className="h-12 w-12 text-slate-400 mx-auto" />
                    <h3 className="text-lg font-semibold text-white">View Only Mode</h3>
                    <p className="text-slate-400 text-sm max-w-md">
                      Click "Edit Mode" above to modify the process configuration.
                    </p>
                  </div>
                </Card>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Status Bar */}
      <div className="h-8 bg-slate-800 border-t border-slate-700 flex items-center px-4">
        <span className="text-slate-400 text-sm">
          {lastStatus || (canEdit ? 'Edit mode enabled - Ready to configure' : 'View only mode')}
        </span>
      </div>
    </div>
  );
}
