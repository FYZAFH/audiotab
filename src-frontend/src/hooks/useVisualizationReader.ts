import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import init, { RingBufferReader } from '../wasm/audiotab_wasm';

export function useVisualizationReader() {
  const [reader, setReader] = useState<RingBufferReader | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function initReader() {
      try {
        // Initialize WASM module
        await init();

        // Get ring buffer data from Tauri
        const mmapData = await invoke<number[]>('get_ringbuffer_data');
        const buffer = new Uint8Array(mmapData);

        // Create reader
        const reader = new RingBufferReader(buffer);
        setReader(reader);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
        console.error('Failed to initialize visualization reader:', err);
      }
    }

    initReader();
  }, []);

  return { reader, error };
}
