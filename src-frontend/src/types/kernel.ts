// Kernel management types

export type KernelStatus =
  | 'Stopped'
  | 'Initializing'
  | 'Running'
  | 'Error';

export interface KernelStatusResponse {
  status: KernelStatus;
  active_devices: number;
}
