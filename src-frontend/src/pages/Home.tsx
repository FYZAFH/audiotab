import { useKernelStatus, useStartKernel, useStopKernel } from '../hooks/useTauriCommands';
import { Button } from '../components/ui/button';
import { Separator } from '../components/ui/separator';
import { AlertCircle, CheckCircle, Loader2, StopCircle, Play, Square } from 'lucide-react';
import type { KernelStatus } from '../types/kernel';

export function Home() {
  const { data: kernelStatus, isError, error } = useKernelStatus();
  const startKernel = useStartKernel();
  const stopKernel = useStopKernel();

  const handleStartKernel = async () => {
    try {
      await startKernel.mutateAsync();
    } catch (err) {
      console.error('Start kernel error:', err);
    }
  };

  const handleStopKernel = async () => {
    try {
      await stopKernel.mutateAsync();
    } catch (err) {
      console.error('Stop kernel error:', err);
    }
  };

  const getStatusIcon = (status: KernelStatus | undefined) => {
    switch (status) {
      case 'Running':
        return <CheckCircle className="h-6 w-6 text-green-500" />;
      case 'Stopped':
        return <StopCircle className="h-6 w-6 text-slate-400" />;
      case 'Initializing':
        return <Loader2 className="h-6 w-6 text-blue-500 animate-spin" />;
      case 'Error':
        return <AlertCircle className="h-6 w-6 text-red-500" />;
      default:
        return <AlertCircle className="h-6 w-6 text-slate-400" />;
    }
  };

  const getStatusColor = (status: KernelStatus | undefined) => {
    switch (status) {
      case 'Running':
        return 'text-green-500';
      case 'Stopped':
        return 'text-slate-400';
      case 'Initializing':
        return 'text-blue-500';
      case 'Error':
        return 'text-red-500';
      default:
        return 'text-slate-400';
    }
  };

  const isStartDisabled =
    kernelStatus?.status === 'Running' ||
    kernelStatus?.status === 'Initializing' ||
    startKernel.isPending;

  const isStopDisabled =
    kernelStatus?.status === 'Stopped' ||
    stopKernel.isPending;

  return (
    <div className="flex flex-col h-full bg-slate-900 p-8">
      <div className="max-w-6xl mx-auto w-full space-y-8">
        {/* Header */}
        <div>
          <h1 className="text-4xl font-bold text-white mb-2">AudioTab Control Center</h1>
          <p className="text-slate-400">Manage your audio processing kernel and visualizations</p>
        </div>

        <Separator className="bg-slate-700" />

        {/* Kernel Status Section */}
        <div className="bg-slate-800 rounded-lg p-6 border border-slate-700">
          <h2 className="text-2xl font-semibold text-white mb-6">Kernel Status</h2>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Status Display */}
            <div className="space-y-4">
              <div className="flex items-center gap-4">
                {getStatusIcon(kernelStatus?.status)}
                <div>
                  <p className="text-sm text-slate-400">Status</p>
                  <p className={`text-xl font-semibold ${getStatusColor(kernelStatus?.status)}`}>
                    {isError ? 'Unknown' : kernelStatus?.status || 'Loading...'}
                  </p>
                </div>
              </div>

              <div className="flex items-center gap-4">
                <div className="h-6 w-6 flex items-center justify-center">
                  <div className="h-4 w-4 bg-blue-500 rounded"></div>
                </div>
                <div>
                  <p className="text-sm text-slate-400">Active Devices</p>
                  <p className="text-xl font-semibold text-white">
                    {isError ? '0' : kernelStatus?.active_devices ?? '0'}
                  </p>
                </div>
              </div>
            </div>

            {/* Control Buttons */}
            <div className="flex flex-col gap-3">
              <Button
                onClick={handleStartKernel}
                disabled={isStartDisabled}
                className="w-full h-12 text-base"
                variant="default"
              >
                {startKernel.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                    Starting...
                  </>
                ) : (
                  <>
                    <Play className="mr-2 h-5 w-5" />
                    Start Kernel
                  </>
                )}
              </Button>

              <Button
                onClick={handleStopKernel}
                disabled={isStopDisabled}
                className="w-full h-12 text-base"
                variant="destructive"
              >
                {stopKernel.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                    Stopping...
                  </>
                ) : (
                  <>
                    <Square className="mr-2 h-5 w-5" />
                    Stop Kernel
                  </>
                )}
              </Button>
            </div>
          </div>

          {/* Error Messages */}
          {(isError || startKernel.isError || stopKernel.isError) && (
            <div className="mt-6 p-4 bg-red-950 border border-red-800 rounded-lg">
              <div className="flex items-start gap-3">
                <AlertCircle className="h-5 w-5 text-red-500 mt-0.5" />
                <div className="flex-1">
                  <h3 className="text-red-500 font-semibold mb-1">Error</h3>
                  <div className="text-red-400 text-sm space-y-1">
                    {isError && <p>Failed to fetch kernel status: {error?.message || 'Unknown error'}</p>}
                    {startKernel.isError && <p>Failed to start kernel: {startKernel.error?.message || 'Unknown error'}</p>}
                    {stopKernel.isError && <p>Failed to stop kernel: {stopKernel.error?.message || 'Unknown error'}</p>}
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Visualization Panel Docking Area */}
        <div className="bg-slate-800 rounded-lg p-6 border border-slate-700">
          <h2 className="text-2xl font-semibold text-white mb-4">Visualization Panels</h2>
          <div className="min-h-[300px] border-2 border-dashed border-slate-600 rounded-lg flex items-center justify-center">
            <div className="text-center">
              <p className="text-slate-400 mb-2">Visualization docking area</p>
              <p className="text-slate-500 text-sm">This feature will be implemented in Phase 5</p>
            </div>
          </div>
        </div>

        {/* Quick Stats */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-slate-800 rounded-lg p-4 border border-slate-700">
            <p className="text-slate-400 text-sm mb-1">Kernel Uptime</p>
            <p className="text-2xl font-semibold text-white">
              {kernelStatus?.status === 'Running' ? 'Active' : 'N/A'}
            </p>
          </div>

          <div className="bg-slate-800 rounded-lg p-4 border border-slate-700">
            <p className="text-slate-400 text-sm mb-1">Processing Pipelines</p>
            <p className="text-2xl font-semibold text-white">0</p>
          </div>

          <div className="bg-slate-800 rounded-lg p-4 border border-slate-700">
            <p className="text-slate-400 text-sm mb-1">Active Visualizations</p>
            <p className="text-2xl font-semibold text-white">0</p>
          </div>
        </div>
      </div>
    </div>
  );
}
