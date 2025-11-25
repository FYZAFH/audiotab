# Phase 4: Streaming & Visualization Design

**Date:** 2025-11-25
**Status:** Design Complete, Ready for Implementation

## Overview

Phase 4 implements real-time audio visualization using a shared memory architecture with WebAssembly processing. This enables multiple visualization types (waveform, spectrogram) to efficiently read from a single data source without IPC overhead.

## Design Decisions

### Architecture Choice: Pure Client-Side Processing

**Selected Approach:** Rust backend writes raw samples to memory-mapped file, WASM module performs all processing (STFT, decimation), JavaScript renders.

**Rationale:**
- Maximum flexibility for interactive zoom/parameter changes
- Simpler backend (focused on audio processing only)
- Modern WASM performance is sufficient (rustfft compiled to WASM)
- Write-once, read-many pattern supports multiple visualization consumers

**Alternatives Considered:**
- Hybrid processing (backend computes STFT on-demand) - rejected due to request/response latency
- Backend-driven prep (pre-computed visualization data) - rejected due to inflexibility

### Data Storage: Raw Samples Only

**Decision:** Ring buffer stores only raw time-domain samples, WASM computes STFT on-demand.

**Rationale:**
- STFT parameters (window size, overlap, frequency range) vary by use case
- Pre-computing locks us into fixed parameters
- Users need different views (zoom levels, frequency resolutions)
- Raw samples provide maximum flexibility

### Ring Buffer Configuration

- **Duration:** 30 seconds of history
- **Layout:** Planar (separate buffer per channel)
- **Memory:** ~12 MB for 8 channels @ 48kHz

**Rationale:**
- 30s allows interactive scrubbing without excessive memory
- Planar layout simplifies single-channel FFT operations
- Easy channel extraction for WASM processing

---

## System Architecture

### Layer 1: Rust Backend (Data Producer)

**Responsibilities:**
- Audio processing pipeline generates raw PCM samples
- Writes samples to memory-mapped file
- Maintains atomic write cursor (sequence number)
- No visualization logic

**Key Components:**
- `RingBufferWriter` - manages mmap file and atomic writes
- Located at `/tmp/audiotab_ringbuf` (platform-specific temp dir)

### Layer 2: WASM Processing Module (Data Transformer)

**Responsibilities:**
- Reads raw samples from shared memory ring buffer
- Performs DSP operations (decimation, STFT, color mapping)
- Runs in Web Worker to avoid blocking UI thread
- Exposes JavaScript API

**Key Components:**
- `RingBufferReader` - reads from mmap, tracks sequence numbers
- `compute_stft()` - STFT implementation using rustfft
- Compiled from Rust using `wasm32-unknown-unknown` target

### Layer 3: React Visualization Components (Renderers)

**Responsibilities:**
- Pure rendering (no data processing)
- Poll WASM at 60fps (16ms intervals)
- Display results using uPlot (waveform) and Canvas 2D (spectrogram)

**Key Components:**
- `<WaveformViewer>` - Time-domain display
- `<SpectrogramViewer>` - Time-frequency display

---

## Memory-Mapped File Format

```
Offset 0-4096: Header (metadata)
  [0-7]:     Magic number (0x4155444954414221) "AUDITAB!"
  [8-15]:    Version (u64, currently 1)
  [16-23]:   Sample rate (u64, e.g., 48000)
  [24-31]:   Channels (u64, e.g., 8)
  [32-39]:   Buffer capacity in samples (u64, e.g., 1,440,000 for 30s @ 48kHz)
  [40-47]:   Write sequence (AtomicU64) - increments with each write
  [48-55]:   Samples per write (u64, e.g., 1024)
  [56-4095]: Reserved for future use

Offset 4096+: Channel data (planar layout)
  Channel 0: [sample_0, sample_1, ..., sample_N]  (f64 array)
  Channel 1: [sample_0, sample_1, ..., sample_N]  (f64 array)
  ...
```

### Ring Buffer Math

- For 30 seconds @ 48kHz: 1,440,000 samples per channel
- 8 channels × 1,440,000 samples × 8 bytes = 92.16 MB
- Ring wrapping: `index = (write_sequence * samples_per_write) % buffer_capacity`

### Atomic Synchronization

- Backend atomically increments `write_sequence` after each write
- WASM reads `write_sequence`, calculates newest samples
- No locks - readers never block writers
- Stale reads acceptable (visualization lag is okay)

---

## Implementation Details

### Rust Backend: Ring Buffer Writer

```rust
pub struct RingBufferWriter {
    mmap: MmapMut,
    sample_rate: u64,
    channels: usize,
    capacity: usize,
    samples_per_write: usize,
    write_sequence: *mut AtomicU64,
}
```

**Key Methods:**
- `new()` - Creates mmap file, writes header
- `write()` - Writes samples to ring buffer, increments sequence atomically

**Files:**
- `src/visualization/ring_buffer.rs` - Ring buffer implementation
- `src/visualization/mod.rs` - Module exports

### WASM Module: Ring Buffer Reader

```rust
#[wasm_bindgen]
pub struct RingBufferReader {
    memory: Vec<u8>,
    sample_rate: u64,
    channels: usize,
    capacity: usize,
    last_read_sequence: u64,
}
```

**Key Methods:**
- `new(buffer: &[u8])` - Parses header, initializes reader
- `get_waveform(channel, num_points)` - Returns decimated waveform
- `get_spectrogram(channel, window_size, hop_size, num_windows)` - Computes STFT

**Files:**
- `wasm-module/src/lib.rs` - Main WASM exports
- `wasm-module/src/stft.rs` - STFT implementation
- `wasm-module/Cargo.toml` - WASM build configuration

### STFT Implementation

**Algorithm:**
1. Apply Hann window to time-domain samples
2. Compute FFT using rustfft
3. Calculate magnitude spectrum in dB (20 × log10)
4. Return flattened [num_windows × num_bins] array

**Parameters:**
- Window size: 2048 samples (default, user-configurable)
- Hop size: 512 samples (75% overlap)
- Window function: Hann (good frequency resolution)

**Performance:**
- Target: <16ms for 2048-point FFT (60fps)
- rustfft compiled to WASM achieves ~5-10ms
- Adaptive quality: reduce window size if falling behind

### Frontend Components

**WaveformViewer:**
- Uses uPlot for high-performance time-domain plotting
- Updates at 60fps via `setInterval(16ms)`
- Decimates data to screen resolution before plotting

**SpectrogramViewer:**
- Uses Canvas 2D ImageData for pixel-level control
- Color mapping: dB magnitude → RGB (Viridis-like colormap)
- Waterfall mode: scrolling time-frequency display

**Custom Hook:**
```typescript
export function useVisualizationReader() {
  const [reader, setReader] = useState<RingBufferReader | null>(null);

  useEffect(() => {
    async function init() {
      await wasmInit();
      const mmapData = await invoke('get_ringbuffer_data');
      const sharedBuffer = new SharedArrayBuffer(mmapData.byteLength);
      new Uint8Array(sharedBuffer).set(new Uint8Array(mmapData));
      setReader(new RingBufferReader(new Uint8Array(sharedBuffer)));
    }
    init();
  }, []);

  return reader;
}
```

---

## Error Handling & Edge Cases

### Synchronization Issues

**Problem:** Reader reads while writer is writing
**Solution:** Use atomic sequence number to detect concurrent writes, retry up to 3 times

### Buffer Overruns

**Problem:** Backend writes faster than WASM reads, old data overwritten
**Detection:** WASM tracks `last_read_sequence`, detects gaps
**Action:** Show UI warning: "Data loss detected (buffer overrun)"

### Performance Degradation

**Problem:** STFT takes >16ms, can't maintain 60fps
**Solution:** Adaptive quality - reduce window size if falling behind
**Monitoring:** Use `performance.now()` to measure WASM processing time

### Memory Leaks

**Problem:** WASM heap grows over time
**Prevention:** Careful buffer management, no circular references
**Testing:** Run for 1+ hour, monitor WASM heap size

---

## Testing Strategy

### Unit Tests

**Rust Backend:**
- Ring buffer wrapping at capacity boundary
- Atomic sequence updates (concurrent readers/writers)
- Planar channel indexing correctness

**WASM Module:**
- Decimation accuracy (compare with scipy.signal.decimate)
- STFT correctness (test with known sine wave, verify FFT peaks)
- Color mapping edge cases (NaN, Inf handling)

### Integration Tests

**End-to-End:**
- Backend writes test signal (440Hz sine) → WASM reads → verify waveform matches
- Multi-channel synchronization (all channels aligned)
- Ring wrapping: write 60s of data to 30s buffer, verify no corruption

**Performance Tests:**
- WASM STFT latency for various window sizes (512, 1024, 2048, 4096)
- Stress test: 64 channels @ 192kHz, verify no dropped frames
- Memory leak test: run visualization for 1 hour, check heap growth

### Manual Testing

- Visual inspection of waveform (should look smooth)
- Spectrogram of known signals (chirp, white noise, music)
- Interactive zoom/pan (verify no artifacts)

---

## Implementation Order

### Phase 4.1: Ring Buffer Infrastructure
1. Implement `RingBufferWriter` in Rust
2. Create mmap file with proper header
3. Write unit tests for ring wrapping
4. Integrate with audio processing pipeline

### Phase 4.2: WASM Reader Module
1. Set up WASM build environment (`wasm-pack`)
2. Implement `RingBufferReader` with basic read operations
3. Add decimation logic for waveform
4. Test with synthetic data from Rust

### Phase 4.3: Basic Waveform Visualization
1. Create `<WaveformViewer>` React component
2. Integrate uPlot for rendering
3. Implement 60fps update loop
4. Add Tauri command to expose mmap data to frontend
5. Verify data flows from Rust → WASM → React → uPlot

### Phase 4.4: STFT Implementation
1. Add rustfft dependency to WASM module
2. Implement Hann window generation
3. Implement `compute_stft()` function
4. Add unit tests with known FFT results
5. Benchmark performance (target <16ms)

### Phase 4.5: Spectrogram Visualization
1. Create `<SpectrogramViewer>` React component
2. Implement Canvas 2D rendering
3. Add color mapping (dB → RGB)
4. Integrate WASM STFT computation
5. Add interactive controls (window size, colormap)

### Phase 4.6: Polish & Optimization
1. Error handling (buffer overruns, sync issues)
2. Performance monitoring and adaptive quality
3. UI controls (channel selection, zoom, pan)
4. Documentation and examples

---

## Success Criteria

- [ ] Waveform displays in real-time at 60fps for 8 channels @ 48kHz
- [ ] Spectrogram updates smoothly with user-adjustable STFT parameters
- [ ] Multiple visualizations can read from same ring buffer simultaneously
- [ ] No memory leaks after 1+ hour of continuous operation
- [ ] STFT latency <16ms for 2048-point window
- [ ] Ring buffer wrapping works correctly (no data corruption)
- [ ] UI remains responsive during heavy visualization (tested with 64 channels)

---

## Dependencies

**Rust:**
- `memmap2` - Memory-mapped file I/O
- `rustfft` - Fast Fourier Transform (also compiled to WASM)

**WASM:**
- `wasm-bindgen` - Rust-JavaScript interop
- `wasm-pack` - Build tooling

**Frontend:**
- `uplot` - High-performance plotting library
- React 19 - UI framework
- TypeScript - Type safety

---

## Future Enhancements (Post-Phase 4)

- WebGL rendering for spectrogram (higher performance)
- Interactive zoom/pan with gesture support
- Multiple colormap options (Viridis, Plasma, Inferno)
- Export visualization as PNG/video
- Waterfall mode with configurable scroll speed
- Phase correlation display for multi-channel analysis
- Real-time peak detection and annotation

---

## Notes

- Shared memory approach chosen over WebSocket due to multiple consumers and multi-channel requirements
- WASM provides zero-copy reads and compiled performance for STFT
- 30-second ring buffer balances memory usage with interactive scrubbing
- Planar layout simplifies single-channel FFT operations
- Pure client-side processing maximizes flexibility for parameter tuning
