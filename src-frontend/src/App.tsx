import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter, Routes, Route, Link, useLocation, useNavigate } from 'react-router-dom';
import { Button } from './components/ui/button';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from './components/ui/dropdown-menu';
import { Home } from './pages/Home';
import { VisualizationDemo } from './pages/VisualizationDemo';
import { HardwareManager } from './pages/HardwareManager';
import { ProcessConfiguration } from './pages/ProcessConfiguration';
import { HomeIcon, ChevronDown } from 'lucide-react';

const queryClient = new QueryClient();

function NavigationBar() {
  const location = useLocation();
  const navigate = useNavigate();

  const isActive = (path: string) => {
    if (path === '/') return location.pathname === '/';
    return location.pathname.startsWith(path);
  };

  const isConfigureActive = isActive('/configure');
  const isViewActive = isActive('/view');

  return (
    <div className="h-14 bg-slate-800 border-b border-slate-700 flex items-center px-4">
      <h1 className="text-white text-xl font-bold mr-8">StreamLab Core</h1>

      {/* Navigation Menubar */}
      <nav className="flex gap-1 flex-1">
        {/* Home */}
        <Link to="/">
          <Button
            variant={isActive('/') && !isConfigureActive && !isViewActive ? 'default' : 'ghost'}
            size="sm"
            className="gap-2"
          >
            <HomeIcon className="h-4 w-4" />
            Home
          </Button>
        </Link>

        {/* Configure Dropdown */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant={isConfigureActive ? 'default' : 'ghost'}
              size="sm"
              className="gap-1"
            >
              Configure
              <ChevronDown className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            <DropdownMenuItem onClick={() => navigate('/configure/process')}>
              Process Configuration
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => navigate('/configure/hardware')}>
              Hardware Manager
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        {/* View Dropdown */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant={isViewActive ? 'default' : 'ghost'}
              size="sm"
              className="gap-1"
            >
              View
              <ChevronDown className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            <DropdownMenuItem onClick={() => navigate('/view/visualization')}>
              Visualization Demo
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        {/* Help Dropdown */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              className="gap-1"
            >
              Help
              <ChevronDown className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            <DropdownMenuItem onClick={() => window.open('https://github.com/your-repo/audiotab', '_blank')}>
              Documentation
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => window.open('https://github.com/your-repo/audiotab/issues', '_blank')}>
              Report Issue
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => alert('StreamLab Core v0.1.0')}>
              About
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </nav>
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
          <Route path="/configure/process" element={<ProcessConfiguration />} />
          <Route path="/configure/hardware" element={<HardwareManager />} />
          <Route path="/view/visualization" element={<VisualizationDemo />} />
          {/* Legacy routes for backward compatibility */}
          <Route path="/editor" element={<ProcessConfiguration />} />
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
