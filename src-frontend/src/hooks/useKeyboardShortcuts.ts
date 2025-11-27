import { useEffect } from 'react';
import { useFlowStore } from '../stores/flowStore';

export function useKeyboardShortcuts(enabled: boolean = true) {
  useEffect(() => {
    if (!enabled) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      // Undo: Ctrl+Z / Cmd+Z
      if ((event.ctrlKey || event.metaKey) && event.key === 'z' && !event.shiftKey) {
        event.preventDefault();
        useFlowStore.getState().undo();
      }

      // Redo: Ctrl+Shift+Z / Cmd+Shift+Z
      if ((event.ctrlKey || event.metaKey) && event.key === 'z' && event.shiftKey) {
        event.preventDefault();
        useFlowStore.getState().redo();
      }

      // Delete: Delete / Backspace
      if (event.key === 'Delete' || event.key === 'Backspace') {
        event.preventDefault();
        useFlowStore.getState().deleteSelected();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [enabled]);
}
