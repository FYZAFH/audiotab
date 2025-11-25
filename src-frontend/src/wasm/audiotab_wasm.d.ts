/* tslint:disable */
/* eslint-disable */
export class RingBufferReader {
  free(): void;
  [Symbol.dispose](): void;
  get_waveform(channel: number, num_points: number): Float64Array;
  get_spectrogram(channel: number, window_size: number, hop_size: number, num_windows: number): Float64Array;
  get_write_sequence(): bigint;
  constructor(buffer: Uint8Array);
  readonly sample_rate: bigint;
  readonly channels: number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_ringbufferreader_free: (a: number, b: number) => void;
  readonly ringbufferreader_channels: (a: number) => number;
  readonly ringbufferreader_get_spectrogram: (a: number, b: number, c: number, d: number, e: number) => [number, number];
  readonly ringbufferreader_get_waveform: (a: number, b: number, c: number) => [number, number];
  readonly ringbufferreader_get_write_sequence: (a: number) => bigint;
  readonly ringbufferreader_new: (a: number, b: number) => number;
  readonly ringbufferreader_sample_rate: (a: number) => bigint;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
