import { useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import FlowEditor from './components/FlowEditor/FlowEditor';
import NodePalette from './components/NodePalette/NodePalette';
import { Button } from './components/ui/button';
import { useFlowStore } from './stores/flowStore';
import { useDeployGraph } from './hooks/useTauriCommands';
import { usePipelineStatusEvents } from './hooks/useTauriEvents';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { VisualizationDemo } from './pages/VisualizationDemo';
import { HardwareManager } from './pages/HardwareManager';

const queryClient = new QueryClient();

function AppContent() {
  useKeyboardShortcuts();
  const [lastStatus, setLastStatus] = useState<string>('');
  const [showVizDemo, setShowVizDemo] = useState<boolean>(false);
  const [showHardware, setShowHardware] = useState<boolean>(false);
  const exportGraph = useFlowStore((state) => state.exportGraph);
  const undo = useFlowStore((state) => state.undo);
  const redo = useFlowStore((state) => state.redo);
  const canUndo = useFlowStore((state) => state.canUndo());
  const canRedo = useFlowStore((state) => state.canRedo());
  const deployMutation = useDeployGraph();

  usePipelineStatusEvents((event) => {
    setLastStatus(`Pipeline ${event.id}: ${event.state}`);
  });

  const handleDeploy = async () => {
    const graph = exportGraph();
    try {
      const pipelineId = await deployMutation.mutateAsync(graph);
      console.log('Deployed pipeline:', pipelineId);
    } catch (error) {
      console.error('Deploy failed:', error);
    }
  };

  return (
    <div className="flex flex-col h-screen bg-slate-900">
      {/* Top Bar */}
      <div className="h-14 bg-slate-800 border-b border-slate-700 flex items-center px-4">
        <h1 className="text-white text-xl font-bold">StreamLab Core</h1>
        <div className="ml-auto space-x-2">
          <Button onClick={() => setShowVizDemo(!showVizDemo)} variant="outline">
            {showVizDemo ? 'Editor' : 'Viz Demo'}
          </Button>
          <Button onClick={() => setShowHardware(!showHardware)} variant="outline">
            {showHardware ? 'Editor' : 'Hardware'}
          </Button>
          {!showVizDemo && !showHardware && (
            <>
              <Button onClick={undo} disabled={!canUndo} variant="outline">
                Undo
              </Button>
              <Button onClick={redo} disabled={!canRedo} variant="outline">
                Redo
              </Button>
              <Button onClick={handleDeploy} disabled={deployMutation.isPending}>
                {deployMutation.isPending ? 'Deploying...' : 'Deploy'}
              </Button>
            </>
          )}
        </div>
      </div>

      {/* Main Content */}
      {showVizDemo ? (
        <div className="flex-1 overflow-auto">
          <VisualizationDemo />
        </div>
      ) : showHardware ? (
        <div className="flex-1 overflow-auto">
          <HardwareManager />
        </div>
      ) : (
        <div className="flex flex-1 overflow-hidden">
          <NodePalette />
          <div className="flex-1">
            <FlowEditor />
          </div>
        </div>
      )}

      {/* Status Bar */}
      <div className="h-8 bg-slate-800 border-t border-slate-700 flex items-center px-4">
        <span className="text-slate-400 text-sm">{lastStatus || 'Ready'}</span>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AppContent />
    </QueryClientProvider>
  );
}
