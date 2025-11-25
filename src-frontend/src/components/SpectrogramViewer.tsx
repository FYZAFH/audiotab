import { useEffect, useRef } from 'react';
import { useVisualizationReader } from '../hooks/useVisualizationReader';
import { magnitudeToColor } from '../utils/colormap';

interface SpectrogramViewerProps {
  channel: number;
  width?: number;
  height?: number;
  windowSize?: number;
  hopSize?: number;
}

export function SpectrogramViewer({
  channel,
  width = 800,
  height = 400,
  windowSize = 2048,
  hopSize = 512,
}: SpectrogramViewerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const { reader, error, isLoading } = useVisualizationReader();
  const animationRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    if (!canvasRef.current || !reader) return;

    const ctx = canvasRef.current.getContext('2d');
    if (!ctx) return;

    const numWindows = width;
    const numBins = windowSize / 2 + 1;

    const update = () => {
      try {
        // Get STFT data
        const stft = reader.get_spectrogram(channel, windowSize, hopSize, numWindows);

        // Create ImageData for fast rendering
        const imageData = ctx.createImageData(numWindows, numBins);

        for (let col = 0; col < numWindows; col++) {
          for (let row = 0; row < numBins; row++) {
            const magnitude = stft[col * numBins + row];
            const color = magnitudeToColor(magnitude);

            // Flip Y axis (low freq at bottom)
            const idx = ((numBins - 1 - row) * numWindows + col) * 4;
            imageData.data[idx] = color.r;
            imageData.data[idx + 1] = color.g;
            imageData.data[idx + 2] = color.b;
            imageData.data[idx + 3] = 255;
          }
        }

        ctx.putImageData(imageData, 0, 0);
      } catch (err) {
        console.error('Failed to update spectrogram:', err);
      }

      animationRef.current = requestAnimationFrame(update);
    };

    update();

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [channel, windowSize, hopSize, width, reader]);

  if (error) {
    return <div className="text-red-500">Error: {error}</div>;
  }

  if (isLoading || !reader) {
    return <div>Loading visualization...</div>;
  }

  return (
    <div>
      <canvas
        ref={canvasRef}
        width={width}
        height={height}
        className="spectrogram-viewer"
      />
    </div>
  );
}
