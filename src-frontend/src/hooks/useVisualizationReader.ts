import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import init, { RingBufferReader } from '../wasm/audiotab_wasm';

export function useVisualizationReader() {
  const [reader, setReader] = useState<RingBufferReader | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let mounted = true;
    let currentReader: RingBufferReader | null = null;

    async function initReader() {
      try {
        setIsLoading(true);
        await init();

        const mmapData = await invoke<number[]>('get_ringbuffer_data');
        const buffer = new Uint8Array(mmapData);

        const newReader = new RingBufferReader(buffer);
        currentReader = newReader;

        if (mounted) {
          setReader(newReader);
          setError(null);
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : 'Unknown error');
          console.error('Failed to initialize visualization reader:', err);
        }
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    }

    initReader();

    return () => {
      mounted = false;
      if (currentReader) {
        currentReader.free();
      }
    };
  }, []);

  return { reader, error, isLoading };
}
