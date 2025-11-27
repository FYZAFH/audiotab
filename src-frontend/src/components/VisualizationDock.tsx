import { useState, useEffect } from 'react';
import { Plus } from 'lucide-react';
import { Button } from './ui/button';
import { VisualizationPanel, type VisualizationPanelData } from './VisualizationPanel';

const STORAGE_KEY = 'audiotab-visualization-panels';

interface VisualizationDockProps {
  onPanelCountChange?: (count: number) => void;
}

export function VisualizationDock({ onPanelCountChange }: VisualizationDockProps) {
  const [panels, setPanels] = useState<VisualizationPanelData[]>([]);

  // Load panels from localStorage on mount
  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try {
        const parsedPanels = JSON.parse(stored);
        setPanels(parsedPanels);
      } catch (err) {
        console.error('Failed to load panels from localStorage:', err);
      }
    }
  }, []);

  // Save panels to localStorage whenever they change
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(panels));
    onPanelCountChange?.(panels.length);
  }, [panels, onPanelCountChange]);

  const addPanel = (type: 'waveform' | 'spectrogram') => {
    const newPanel: VisualizationPanelData = {
      id: `${type}-${Date.now()}`,
      type,
      title: type === 'waveform'
        ? `Waveform - Channel ${panels.filter(p => p.type === 'waveform').length}`
        : `Spectrogram - Channel ${panels.filter(p => p.type === 'spectrogram').length}`,
      channel: 0, // Default to channel 0
    };

    setPanels(prev => [...prev, newPanel]);
  };

  const removePanel = (id: string) => {
    setPanels(prev => prev.filter(p => p.id !== id));
  };

  const clearAll = () => {
    if (panels.length === 0) return;
    if (window.confirm('Remove all visualization panels?')) {
      setPanels([]);
    }
  };

  return (
    <div className="space-y-4">
      {/* Controls */}
      <div className="flex items-center justify-between">
        <div className="flex gap-2">
          <Button
            onClick={() => addPanel('waveform')}
            variant="default"
            className="flex items-center gap-2"
          >
            <Plus className="h-4 w-4" />
            Add Waveform
          </Button>
          <Button
            onClick={() => addPanel('spectrogram')}
            variant="default"
            className="flex items-center gap-2"
          >
            <Plus className="h-4 w-4" />
            Add Spectrogram
          </Button>
        </div>

        {panels.length > 0 && (
          <Button
            onClick={clearAll}
            variant="outline"
            className="text-slate-400 hover:text-white"
          >
            Clear All
          </Button>
        )}
      </div>

      {/* Panels Grid */}
      {panels.length === 0 ? (
        <div className="min-h-[300px] border-2 border-dashed border-slate-600 rounded-lg flex items-center justify-center">
          <div className="text-center">
            <p className="text-slate-400 mb-2">No visualization panels docked</p>
            <p className="text-slate-500 text-sm">Click "Add Waveform" or "Add Spectrogram" to get started</p>
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {panels.map(panel => (
            <VisualizationPanel
              key={panel.id}
              panel={panel}
              onRemove={removePanel}
            />
          ))}
        </div>
      )}

      {/* Info Footer */}
      {panels.length > 0 && (
        <div className="text-center text-slate-500 text-sm">
          {panels.length} visualization panel{panels.length !== 1 ? 's' : ''} active
        </div>
      )}
    </div>
  );
}
