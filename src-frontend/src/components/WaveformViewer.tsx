import { useEffect, useRef } from 'react';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import { useVisualizationReader } from '../hooks/useVisualizationReader';

interface WaveformViewerProps {
  channel: number;
  width?: number;
  height?: number;
}

export function WaveformViewer({
  channel,
  width = 800,
  height = 300
}: WaveformViewerProps) {
  const plotRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<uPlot | null>(null);
  const { reader, error, isLoading } = useVisualizationReader();

  useEffect(() => {
    if (!plotRef.current || !reader) return;

    // Initialize uPlot
    const opts: uPlot.Options = {
      width,
      height,
      series: [
        {},  // x-axis (time)
        {
          stroke: 'cyan',
          label: `Channel ${channel}`,
          width: 2,
        }
      ],
      axes: [
        { label: 'Time (s)' },
        { label: 'Amplitude', scale: 'amp' }
      ],
      scales: {
        amp: {
          auto: true,
        }
      },
    };

    chartRef.current = new uPlot(opts, [[], []], plotRef.current);

    // Start 60fps update loop
    const intervalId = setInterval(() => {
      if (!reader || !chartRef.current) return;

      try {
        const waveform = reader.get_waveform(channel, width);
        const timeAxis = Array.from({ length: waveform.length }, (_, i) => i / 60);

        chartRef.current.setData([timeAxis, Array.from(waveform)]);
      } catch (err) {
        console.error('Failed to update waveform:', err);
      }
    }, 16);  // ~60fps

    return () => {
      clearInterval(intervalId);
      chartRef.current?.destroy();
    };
  }, [channel, width, height, reader]);

  if (error) {
    return <div className="text-red-500">Error: {error}</div>;
  }

  if (isLoading) {
    return <div>Loading visualization...</div>;
  }

  return <div ref={plotRef} className="waveform-viewer" />;
}
