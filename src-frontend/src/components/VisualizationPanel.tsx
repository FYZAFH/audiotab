import { X } from 'lucide-react';
import { Button } from './ui/button';
import { WaveformViewer } from './WaveformViewer';
import { SpectrogramViewer } from './SpectrogramViewer';

export interface VisualizationPanelData {
  id: string;
  type: 'waveform' | 'spectrogram';
  title: string;
  channel: number;
}

interface VisualizationPanelProps {
  panel: VisualizationPanelData;
  onRemove: (id: string) => void;
}

export function VisualizationPanel({ panel, onRemove }: VisualizationPanelProps) {
  const renderVisualization = () => {
    switch (panel.type) {
      case 'waveform':
        return (
          <WaveformViewer
            channel={panel.channel}
            width={500}
            height={250}
          />
        );
      case 'spectrogram':
        return (
          <SpectrogramViewer
            channel={panel.channel}
            width={500}
            height={300}
            windowSize={2048}
            hopSize={512}
          />
        );
      default:
        return <div className="text-slate-400">Unknown visualization type</div>;
    }
  };

  return (
    <div className="bg-slate-800 rounded-lg border border-slate-700 overflow-hidden">
      {/* Panel Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-slate-750 border-b border-slate-700">
        <h3 className="text-white font-semibold">{panel.title}</h3>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onRemove(panel.id)}
          className="h-8 w-8 p-0 hover:bg-slate-600"
          title="Remove panel"
        >
          <X className="h-4 w-4 text-slate-400 hover:text-white" />
        </Button>
      </div>

      {/* Panel Content */}
      <div className="p-4 flex items-center justify-center bg-slate-850">
        {renderVisualization()}
      </div>
    </div>
  );
}
