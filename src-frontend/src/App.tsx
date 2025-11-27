import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter, Routes, Route, Link, useLocation } from 'react-router-dom';
import { Button } from './components/ui/button';
import { Home } from './pages/Home';
import { VisualizationDemo } from './pages/VisualizationDemo';
import { HardwareManager } from './pages/HardwareManager';
import FlowEditor from './components/FlowEditor/FlowEditor';
import NodePalette from './components/NodePalette/NodePalette';
import { useFlowStore } from './stores/flowStore';
import { useDeployGraph } from './hooks/useTauriCommands';
import { usePipelineStatusEvents } from './hooks/useTauriEvents';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { useState } from 'react';
import { HomeIcon, Activity, Cpu, Workflow } from 'lucide-react';

const queryClient = new QueryClient();

function NavigationBar() {
  const location = useLocation();

  const isActive = (path: string) => location.pathname === path;

  return (
    <div className="h-14 bg-slate-800 border-b border-slate-700 flex items-center px-4">
      <h1 className="text-white text-xl font-bold mr-8">StreamLab Core</h1>

      {/* Navigation Links */}
      <nav className="flex gap-2 flex-1">
        <Link to="/">
          <Button
            variant={isActive('/') ? 'default' : 'outline'}
            size="sm"
            className="gap-2"
          >
            <HomeIcon className="h-4 w-4" />
            Home
          </Button>
        </Link>
        <Link to="/editor">
          <Button
            variant={isActive('/editor') ? 'default' : 'outline'}
            size="sm"
            className="gap-2"
          >
            <Workflow className="h-4 w-4" />
            Flow Editor
          </Button>
        </Link>
        <Link to="/hardware">
          <Button
            variant={isActive('/hardware') ? 'default' : 'outline'}
            size="sm"
            className="gap-2"
          >
            <Cpu className="h-4 w-4" />
            Hardware
          </Button>
        </Link>
        <Link to="/viz-demo">
          <Button
            variant={isActive('/viz-demo') ? 'default' : 'outline'}
            size="sm"
            className="gap-2"
          >
            <Activity className="h-4 w-4" />
            Viz Demo
          </Button>
        </Link>
      </nav>
    </div>
  );
}

function EditorPage() {
  useKeyboardShortcuts();
  const [lastStatus, setLastStatus] = useState<string>('');
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
    <div className="flex flex-col h-full">
      {/* Editor Toolbar */}
      <div className="h-12 bg-slate-800 border-b border-slate-700 flex items-center px-4 gap-2">
        <Button onClick={undo} disabled={!canUndo} variant="outline" size="sm">
          Undo
        </Button>
        <Button onClick={redo} disabled={!canRedo} variant="outline" size="sm">
          Redo
        </Button>
        <div className="flex-1" />
        <Button onClick={handleDeploy} disabled={deployMutation.isPending} size="sm">
          {deployMutation.isPending ? 'Deploying...' : 'Deploy'}
        </Button>
      </div>

      {/* Editor Content */}
      <div className="flex flex-1 overflow-hidden">
        <NodePalette />
        <div className="flex-1">
          <FlowEditor />
        </div>
      </div>

      {/* Status Bar */}
      <div className="h-8 bg-slate-800 border-t border-slate-700 flex items-center px-4">
        <span className="text-slate-400 text-sm">{lastStatus || 'Ready'}</span>
      </div>
    </div>
  );
}

function AppContent() {
  return (
    <div className="flex flex-col h-screen bg-slate-900">
      <NavigationBar />

      <div className="flex-1 overflow-hidden">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/editor" element={<EditorPage />} />
          <Route path="/hardware" element={<HardwareManager />} />
          <Route path="/viz-demo" element={<VisualizationDemo />} />
        </Routes>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AppContent />
      </BrowserRouter>
    </QueryClientProvider>
  );
}
