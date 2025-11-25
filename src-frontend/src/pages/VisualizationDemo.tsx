import { WaveformViewer } from '../components/WaveformViewer';
import { SpectrogramViewer } from '../components/SpectrogramViewer';

export function VisualizationDemo() {
  return (
    <div className="p-4 space-y-4">
      <h1 className="text-2xl font-bold">Phase 4: Visualization Demo</h1>

      <div className="space-y-2">
        <h2 className="text-xl font-semibold">Waveform (Channel 0)</h2>
        <WaveformViewer channel={0} width={800} height={200} />
      </div>

      <div className="space-y-2">
        <h2 className="text-xl font-semibold">Spectrogram (Channel 0)</h2>
        <SpectrogramViewer
          channel={0}
          width={800}
          height={300}
          windowSize={2048}
          hopSize={512}
        />
      </div>
    </div>
  );
}
