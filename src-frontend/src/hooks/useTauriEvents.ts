import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface PipelineStatusEvent {
  id: string;
  state: string;
  error?: string;
}

export function usePipelineStatusEvents(
  callback: (event: PipelineStatusEvent) => void
) {
  useEffect(() => {
    const unlisten = listen<PipelineStatusEvent>('pipeline-status', (event) => {
      callback(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [callback]);
}
