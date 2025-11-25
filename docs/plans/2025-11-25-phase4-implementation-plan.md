# Phase 4: Streaming & Visualization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement real-time audio visualization using shared memory ring buffer, WebAssembly processing, and React components for waveform and spectrogram display.

**Architecture:** Rust backend writes raw PCM samples to memory-mapped file (planar layout, 30s ring buffer). WASM module reads from shared memory, performs DSP (decimation, STFT), exposes JavaScript API. React components poll WASM at 60fps and render using uPlot (waveform) and Canvas 2D (spectrogram).

**Tech Stack:** Rust (memmap2), WebAssembly (wasm-bindgen, rustfft), React 19, TypeScript, uPlot

---

## Task 1: Ring Buffer Module Structure

**Files:**
- Create: `src/visualization/mod.rs`
- Create: `src/visualization/ring_buffer.rs`
- Modify: `src/lib.rs` (add visualization module)

**Step 1: Create visualization module**

Create `src/visualization/mod.rs`:
```rust
pub mod ring_buffer;

pub use ring_buffer::RingBufferWriter;
```

**Step 2: Add module to lib.rs**

Modify `src/lib.rs`, add:
```rust
pub mod visualization;
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: SUCCESS (no errors)

**Step 4: Commit module structure**

```bash
git add src/visualization/mod.rs src/lib.rs
git commit -m "feat(viz): add visualization module structure"
```

---

## Task 2: Ring Buffer Writer - Data Structures

**Files:**
- Create: `src/visualization/ring_buffer.rs`
- Create: `Cargo.toml` (add memmap2 dependency)

**Step 1: Add memmap2 dependency**

Modify `Cargo.toml`, add to `[dependencies]`:
```toml
memmap2 = "0.9"
```

**Step 2: Write test for RingBufferWriter creation**

Create `src/visualization/ring_buffer.rs`:
```rust
use anyhow::Result;
use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_ring_buffer() {
        let path = "/tmp/test_ringbuf_create";
        let _ = fs::remove_file(path); // cleanup if exists

        let writer = RingBufferWriter::new(path, 48000, 2, 1).unwrap();

        // Verify file exists
        assert!(Path::new(path).exists());

        // Verify header values
        assert_eq!(writer.sample_rate, 48000);
        assert_eq!(writer.channels, 2);
        assert_eq!(writer.capacity, 48000); // 1 second

        // Cleanup
        drop(writer);
        fs::remove_file(path).unwrap();
    }
}

pub struct RingBufferWriter {
    sample_rate: u64,
    channels: usize,
    capacity: usize,
}

impl RingBufferWriter {
    pub fn new(
        path: impl AsRef<Path>,
        sample_rate: u64,
        channels: usize,
        duration_secs: u64,
    ) -> Result<Self> {
        todo!("Implement in next step")
    }
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test test_create_ring_buffer`
Expected: FAIL with "not yet implemented"

**Step 4: Implement RingBufferWriter::new()**

Replace the `impl RingBufferWriter` block:
```rust
impl RingBufferWriter {
    pub fn new(
        path: impl AsRef<Path>,
        sample_rate: u64,
        channels: usize,
        duration_secs: u64,
    ) -> Result<Self> {
        let capacity = (sample_rate * duration_secs) as usize;
        let header_size = 4096;
        let data_size = channels * capacity * 8; // 8 bytes per f64
        let total_size = header_size + data_size;

        // Create memory-mapped file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        file.set_len(total_size as u64)?;

        let mut mmap = unsafe { MmapMut::map_mut(&file)? };

        // Write header
        mmap[0..8].copy_from_slice(b"AUDITAB!");
        mmap[8..16].copy_from_slice(&1u64.to_le_bytes()); // version
        mmap[16..24].copy_from_slice(&sample_rate.to_le_bytes());
        mmap[24..32].copy_from_slice(&(channels as u64).to_le_bytes());
        mmap[32..40].copy_from_slice(&(capacity as u64).to_le_bytes());

        // Initialize write_sequence to 0
        let write_seq_ptr = &mut mmap[40..48];
        write_seq_ptr.copy_from_slice(&0u64.to_le_bytes());

        Ok(Self {
            sample_rate,
            channels,
            capacity,
        })
    }
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test test_create_ring_buffer`
Expected: PASS

**Step 6: Commit**

```bash
git add src/visualization/ring_buffer.rs Cargo.toml
git commit -m "feat(viz): implement RingBufferWriter creation and header writing"
```

---

## Task 3: Ring Buffer Writer - Write Operation

**Files:**
- Modify: `src/visualization/ring_buffer.rs`

**Step 1: Add fields to RingBufferWriter**

Modify `RingBufferWriter` struct:
```rust
pub struct RingBufferWriter {
    _mmap: MmapMut,
    sample_rate: u64,
    channels: usize,
    capacity: usize,
    samples_per_write: usize,
    write_sequence: *mut AtomicU64,
}
```

And update `new()` to store mmap and write_sequence pointer:
```rust
// At end of new(), before Ok(Self):
let write_sequence = unsafe {
    &mut *(mmap[40..48].as_mut_ptr() as *mut AtomicU64)
};

Ok(Self {
    _mmap: mmap,
    sample_rate,
    channels,
    capacity,
    samples_per_write: 1024,
    write_sequence,
})
```

**Step 2: Write test for write operation**

Add to tests module:
```rust
#[test]
fn test_write_samples() {
    let path = "/tmp/test_ringbuf_write";
    let _ = fs::remove_file(path);

    let mut writer = RingBufferWriter::new(path, 48000, 2, 1).unwrap();

    // Write 1024 samples to each channel
    let samples = vec![
        vec![1.0; 1024], // channel 0
        vec![2.0; 1024], // channel 1
    ];

    writer.write(&samples).unwrap();

    // Verify write_sequence incremented
    let seq = writer.get_write_sequence();
    assert_eq!(seq, 1);

    // Cleanup
    drop(writer);
    fs::remove_file(path).unwrap();
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test test_write_samples`
Expected: FAIL with "no method named `write`"

**Step 4: Implement write() method**

Add to `impl RingBufferWriter`:
```rust
pub fn write(&mut self, samples: &[Vec<f64>]) -> Result<()> {
    use anyhow::ensure;

    ensure!(
        samples.len() == self.channels,
        "Expected {} channels, got {}",
        self.channels,
        samples.len()
    );

    let seq = unsafe { (*self.write_sequence).load(Ordering::Acquire) };
    let start_idx = ((seq as usize) * self.samples_per_write) % self.capacity;

    // Write each channel
    for (ch_id, ch_samples) in samples.iter().enumerate() {
        let ch_offset = 4096 + (ch_id * self.capacity * 8);

        for (i, &sample) in ch_samples.iter().enumerate() {
            let idx = (start_idx + i) % self.capacity;
            let offset = ch_offset + (idx * 8);
            self._mmap[offset..offset + 8].copy_from_slice(&sample.to_le_bytes());
        }
    }

    // Atomically increment sequence
    unsafe {
        (*self.write_sequence).fetch_add(1, Ordering::Release);
    }

    Ok(())
}

pub fn get_write_sequence(&self) -> u64 {
    unsafe { (*self.write_sequence).load(Ordering::Acquire) }
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test test_write_samples`
Expected: PASS

**Step 6: Commit**

```bash
git add src/visualization/ring_buffer.rs
git commit -m "feat(viz): implement ring buffer write operation with atomic sequencing"
```

---

## Task 4: WASM Module Setup

**Files:**
- Create: `wasm-module/Cargo.toml`
- Create: `wasm-module/src/lib.rs`
- Create: `wasm-module/.cargo/config.toml`

**Step 1: Create WASM module directory and Cargo.toml**

Create `wasm-module/Cargo.toml`:
```toml
[package]
name = "audiotab-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde-wasm-bindgen = "0.6"

[profile.release]
opt-level = 3
lto = true
```

**Step 2: Create WASM config for build**

Create `wasm-module/.cargo/config.toml`:
```toml
[build]
target = "wasm32-unknown-unknown"
```

**Step 3: Create basic WASM module**

Create `wasm-module/src/lib.rs`:
```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct RingBufferReader {
    sample_rate: u64,
    channels: usize,
    capacity: usize,
}

#[wasm_bindgen]
impl RingBufferReader {
    #[wasm_bindgen(constructor)]
    pub fn new(buffer: &[u8]) -> Self {
        // Parse header
        let sample_rate = u64::from_le_bytes(buffer[16..24].try_into().unwrap());
        let channels = u64::from_le_bytes(buffer[24..32].try_into().unwrap()) as usize;
        let capacity = u64::from_le_bytes(buffer[32..40].try_into().unwrap()) as usize;

        Self {
            sample_rate,
            channels,
            capacity,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn sample_rate(&self) -> u64 {
        self.sample_rate
    }

    #[wasm_bindgen(getter)]
    pub fn channels(&self) -> usize {
        self.channels
    }
}
```

**Step 4: Verify WASM builds**

Run: `cd wasm-module && cargo build --target wasm32-unknown-unknown`
Expected: SUCCESS

**Step 5: Install wasm-pack**

Run: `cargo install wasm-pack`
Expected: wasm-pack installed

**Step 6: Build with wasm-pack**

Run: `cd wasm-module && wasm-pack build --target web`
Expected: SUCCESS, creates `pkg/` directory

**Step 7: Commit**

```bash
git add wasm-module/
git commit -m "feat(wasm): add WASM module with RingBufferReader structure"
```

---

## Task 5: WASM Reader - Waveform Decimation

**Files:**
- Modify: `wasm-module/src/lib.rs`

**Step 1: Add memory field to store buffer**

Modify `RingBufferReader`:
```rust
#[wasm_bindgen]
pub struct RingBufferReader {
    memory: Vec<u8>,
    sample_rate: u64,
    channels: usize,
    capacity: usize,
}
```

Update constructor:
```rust
#[wasm_bindgen(constructor)]
pub fn new(buffer: &[u8]) -> Self {
    let sample_rate = u64::from_le_bytes(buffer[16..24].try_into().unwrap());
    let channels = u64::from_le_bytes(buffer[24..32].try_into().unwrap()) as usize;
    let capacity = u64::from_le_bytes(buffer[32..40].try_into().unwrap()) as usize;

    Self {
        memory: buffer.to_vec(),
        sample_rate,
        channels,
        capacity,
    }
}
```

**Step 2: Implement get_waveform method**

Add to `impl RingBufferReader`:
```rust
#[wasm_bindgen]
pub fn get_waveform(&self, channel: usize, num_points: usize) -> Vec<f64> {
    let ch_offset = 4096 + (channel * self.capacity * 8);
    let decimation = self.capacity / num_points;

    let mut result = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let idx = (i * decimation) % self.capacity;
        let offset = ch_offset + (idx * 8);

        if offset + 8 <= self.memory.len() {
            let sample = f64::from_le_bytes(
                self.memory[offset..offset + 8].try_into().unwrap()
            );
            result.push(sample);
        }
    }

    result
}
```

**Step 3: Build WASM module**

Run: `cd wasm-module && wasm-pack build --target web`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add wasm-module/src/lib.rs
git commit -m "feat(wasm): implement waveform decimation for visualization"
```

---

## Task 6: Tauri Command for Ring Buffer Access

**Files:**
- Modify: `src-tauri/src/main.rs`
- Create: `src-tauri/src/commands/visualization.rs`
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Create visualization commands module**

Create `src-tauri/src/commands/visualization.rs`:
```rust
use std::fs;
use tauri::State;

#[tauri::command]
pub async fn get_ringbuffer_data() -> Result<Vec<u8>, String> {
    let path = "/tmp/audiotab_ringbuf";

    fs::read(path).map_err(|e| format!("Failed to read ring buffer: {}", e))
}
```

**Step 2: Add to commands module**

Modify `src-tauri/src/commands/mod.rs`, add:
```rust
pub mod visualization;
```

**Step 3: Register command in main.rs**

Modify `src-tauri/src/main.rs`, in `.invoke_handler()`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    commands::visualization::get_ringbuffer_data,
])
```

**Step 4: Build to verify**

Run: `cd src-tauri && cargo check`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add src-tauri/src/commands/visualization.rs src-tauri/src/commands/mod.rs src-tauri/src/main.rs
git commit -m "feat(tauri): add command to expose ring buffer to frontend"
```

---

## Task 7: Frontend - WASM Integration Hook

**Files:**
- Create: `src-frontend/src/hooks/useVisualizationReader.ts`
- Copy: `wasm-module/pkg/*` to `src-frontend/src/wasm/`

**Step 1: Copy WASM build artifacts to frontend**

Run:
```bash
mkdir -p src-frontend/src/wasm
cp wasm-module/pkg/audiotab_wasm.js src-frontend/src/wasm/
cp wasm-module/pkg/audiotab_wasm_bg.wasm src-frontend/src/wasm/
cp wasm-module/pkg/audiotab_wasm.d.ts src-frontend/src/wasm/
```

**Step 2: Create visualization reader hook**

Create `src-frontend/src/hooks/useVisualizationReader.ts`:
```typescript
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
```

**Step 3: Verify TypeScript compiles**

Run: `cd src-frontend && npm run build`
Expected: SUCCESS (or type errors if WASM types not found - acceptable for now)

**Step 4: Commit**

```bash
git add src-frontend/src/hooks/useVisualizationReader.ts src-frontend/src/wasm/
git commit -m "feat(frontend): add WASM visualization reader hook"
```

---

## Task 8: Waveform Viewer Component

**Files:**
- Create: `src-frontend/src/components/WaveformViewer.tsx`
- Modify: `src-frontend/package.json` (add uplot dependency)

**Step 1: Add uPlot dependency**

Run: `cd src-frontend && npm install uplot`

**Step 2: Create WaveformViewer component**

Create `src-frontend/src/components/WaveformViewer.tsx`:
```tsx
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
  const { reader, error } = useVisualizationReader();

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

  if (!reader) {
    return <div>Loading visualization...</div>;
  }

  return <div ref={plotRef} className="waveform-viewer" />;
}
```

**Step 3: Verify TypeScript compiles**

Run: `cd src-frontend && npm run build`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src-frontend/src/components/WaveformViewer.tsx src-frontend/package.json
git commit -m "feat(frontend): add WaveformViewer component with uPlot"
```

---

## Task 9: STFT Implementation in WASM

**Files:**
- Modify: `wasm-module/Cargo.toml` (add rustfft)
- Create: `wasm-module/src/stft.rs`
- Modify: `wasm-module/src/lib.rs`

**Step 1: Add rustfft dependency**

Modify `wasm-module/Cargo.toml`, add to `[dependencies]`:
```toml
rustfft = "6.1"
```

**Step 2: Create STFT module**

Create `wasm-module/src/stft.rs`:
```rust
use rustfft::{FftPlanner, num_complex::Complex};
use std::f64::consts::PI;

pub fn compute_stft(
    samples: &[f64],
    window_size: usize,
    hop_size: usize,
) -> Vec<f64> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    let num_windows = (samples.len().saturating_sub(window_size)) / hop_size + 1;
    let num_bins = window_size / 2 + 1;

    let mut result = Vec::with_capacity(num_windows * num_bins);
    let hann_window = create_hann_window(window_size);

    for i in 0..num_windows {
        let start = i * hop_size;
        let end = start + window_size;

        if end > samples.len() {
            break;
        }

        // Apply window function
        let mut windowed: Vec<Complex<f64>> = samples[start..end]
            .iter()
            .zip(hann_window.iter())
            .map(|(&s, &w)| Complex::new(s * w, 0.0))
            .collect();

        // Compute FFT
        fft.process(&mut windowed);

        // Compute magnitude spectrum (dB)
        for bin in windowed.iter().take(num_bins) {
            let magnitude = bin.norm();
            let db = 20.0 * (magnitude + 1e-10).log10();
            result.push(db);
        }
    }

    result
}

fn create_hann_window(size: usize) -> Vec<f64> {
    (0..size)
        .map(|i| 0.5 * (1.0 - ((2.0 * PI * i as f64) / (size - 1) as f64).cos()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hann_window() {
        let window = create_hann_window(4);
        assert_eq!(window.len(), 4);
        assert!(window[0] < 0.1); // First value near 0
        assert!(window[2] > 0.9); // Middle value near 1
    }

    #[test]
    fn test_stft_dimensions() {
        let samples: Vec<f64> = (0..8192).map(|_| 0.0).collect();
        let window_size = 2048;
        let hop_size = 512;

        let result = compute_stft(&samples, window_size, hop_size);

        let num_bins = window_size / 2 + 1;
        let num_windows = (samples.len() - window_size) / hop_size + 1;

        assert_eq!(result.len(), num_windows * num_bins);
    }
}
```

**Step 3: Add STFT module to lib.rs**

Modify `wasm-module/src/lib.rs`, add at top:
```rust
mod stft;
use stft::compute_stft;
```

Add to `impl RingBufferReader`:
```rust
#[wasm_bindgen]
pub fn get_spectrogram(
    &self,
    channel: usize,
    window_size: usize,
    hop_size: usize,
    num_windows: usize,
) -> Vec<f64> {
    // Read samples for STFT
    let sample_count = window_size + (num_windows - 1) * hop_size;
    let samples = self.read_channel_samples(channel, sample_count);

    // Compute STFT
    compute_stft(&samples, window_size, hop_size)
}

fn read_channel_samples(&self, channel: usize, count: usize) -> Vec<f64> {
    let ch_offset = 4096 + (channel * self.capacity * 8);
    let mut samples = Vec::with_capacity(count);

    for i in 0..count {
        let idx = i % self.capacity;
        let offset = ch_offset + (idx * 8);

        if offset + 8 <= self.memory.len() {
            samples.push(f64::from_le_bytes(
                self.memory[offset..offset + 8].try_into().unwrap()
            ));
        }
    }

    samples
}
```

**Step 4: Run WASM tests**

Run: `cd wasm-module && cargo test`
Expected: PASS (2 tests)

**Step 5: Build WASM module**

Run: `cd wasm-module && wasm-pack build --target web`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add wasm-module/
git commit -m "feat(wasm): implement STFT with rustfft for spectrogram"
```

---

## Task 10: Spectrogram Viewer Component

**Files:**
- Create: `src-frontend/src/components/SpectrogramViewer.tsx`
- Create: `src-frontend/src/utils/colormap.ts`

**Step 1: Create colormap utility**

Create `src-frontend/src/utils/colormap.ts`:
```typescript
export interface RGB {
  r: number;
  g: number;
  b: number;
}

/**
 * Maps dB magnitude to RGB color (Viridis-like colormap)
 * @param db - Magnitude in dB (typically -80 to 0)
 * @returns RGB color
 */
export function magnitudeToColor(db: number): RGB {
  // Map -80dB to 0dB → 0 to 1 range
  const normalized = (db + 80) / 80;
  const clamped = Math.max(0, Math.min(1, normalized));

  // Simple linear interpolation (blue → cyan → green → yellow → red)
  if (clamped < 0.25) {
    const t = clamped * 4;
    return { r: 0, g: 0, b: Math.floor(255 * (1 - t)) };
  } else if (clamped < 0.5) {
    const t = (clamped - 0.25) * 4;
    return { r: 0, g: Math.floor(255 * t), b: 255 };
  } else if (clamped < 0.75) {
    const t = (clamped - 0.5) * 4;
    return { r: Math.floor(255 * t), g: 255, b: Math.floor(255 * (1 - t)) };
  } else {
    const t = (clamped - 0.75) * 4;
    return { r: 255, g: Math.floor(255 * (1 - t)), b: 0 };
  }
}
```

**Step 2: Create SpectrogramViewer component**

Create `src-frontend/src/components/SpectrogramViewer.tsx`:
```tsx
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
  const { reader, error } = useVisualizationReader();
  const animationRef = useRef<number>();

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

  if (!reader) {
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
```

**Step 3: Verify TypeScript compiles**

Run: `cd src-frontend && npm run build`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src-frontend/src/components/SpectrogramViewer.tsx src-frontend/src/utils/colormap.ts
git commit -m "feat(frontend): add SpectrogramViewer with Canvas 2D rendering"
```

---

## Task 11: Integration - Connect Backend to Frontend

**Files:**
- Modify: `src/nodes/audio_source.rs` (write to ring buffer)
- Modify: `src-tauri/src/main.rs` (initialize ring buffer)

**Step 1: Add ring buffer writer to AudioSourceNode**

Modify `src/nodes/audio_source.rs`:
```rust
use crate::visualization::RingBufferWriter;
use std::sync::{Arc, Mutex};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Audio Source", category = "Sources")]
pub struct AudioSourceNode {
    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "48000", min = 8000.0, max = 192000.0)]
    pub sample_rate: u32,

    #[param(default = "1024", min = 64.0, max = 8192.0)]
    pub buffer_size: u32,

    #[serde(skip)]
    sequence: u64,

    #[serde(skip)]
    ring_buffer: Option<Arc<Mutex<RingBufferWriter>>>,
}
```

Update `process()` to write to ring buffer:
```rust
async fn process(&mut self, mut frame: DataFrame) -> Result<DataFrame> {
    // Generate silent audio for now (will be replaced with real capture)
    let samples = vec![0.0; self.buffer_size as usize];

    // Write to ring buffer
    if let Some(rb) = &self.ring_buffer {
        let mut writer = rb.lock().unwrap();
        let _ = writer.write(&vec![samples.clone()]); // Single channel for now
    }

    frame.payload.insert(
        "main_channel".to_string(),
        std::sync::Arc::new(samples),
    );

    self.sequence += 1;
    frame.sequence_id = self.sequence;

    Ok(frame)
}
```

**Step 2: Initialize ring buffer in main.rs**

Modify `src-tauri/src/main.rs`:
```rust
use audiotab::visualization::RingBufferWriter;

fn main() {
    // Initialize ring buffer
    let ring_buffer = RingBufferWriter::new(
        "/tmp/audiotab_ringbuf",
        48000,
        2,
        30,
    ).expect("Failed to create ring buffer");

    tauri::Builder::default()
        .manage(ring_buffer)
        // ... rest of setup
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Build to verify**

Run: `cargo check`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src/nodes/audio_source.rs src-tauri/src/main.rs
git commit -m "feat(integration): connect audio source to ring buffer writer"
```

---

## Task 12: Demo Page

**Files:**
- Create: `src-frontend/src/pages/VisualizationDemo.tsx`
- Modify: `src-frontend/src/App.tsx` (add route)

**Step 1: Create demo page**

Create `src-frontend/src/pages/VisualizationDemo.tsx`:
```tsx
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
```

**Step 2: Add route to App.tsx**

Modify `src-frontend/src/App.tsx` to include the demo page (adjust based on your routing setup).

**Step 3: Test the application**

Run: `cd src-tauri && cargo tauri dev`
Expected: Application launches with visualization demo

**Step 4: Commit**

```bash
git add src-frontend/src/pages/VisualizationDemo.tsx src-frontend/src/App.tsx
git commit -m "feat(demo): add visualization demo page with waveform and spectrogram"
```

---

## Success Criteria

After completing all tasks:

- [ ] Ring buffer successfully writes audio data to memory-mapped file
- [ ] WASM module can read from ring buffer and parse header
- [ ] Waveform displays in real-time at ~60fps
- [ ] Spectrogram updates smoothly with configurable STFT parameters
- [ ] No memory leaks (verified by running for 5+ minutes)
- [ ] Frontend components render without errors

---

## Testing Plan

**Unit Tests:**
- Ring buffer creation and header writing
- Ring buffer write operation and sequence incrementing
- WASM STFT computation (known sine wave → expected FFT peaks)

**Integration Tests:**
- End-to-end: Backend writes → WASM reads → Verify data matches

**Manual Tests:**
- Visual inspection of waveform (smooth display)
- Spectrogram of known signals (verify frequency content)
- Performance (no stuttering, maintains 60fps)

---

## Next Steps

After Phase 4 completion:
- Real audio capture (replace silent audio with CPAL/hardware)
- Interactive controls (zoom, pan, parameter adjustment)
- Multiple visualization windows
- Export capabilities (PNG, video)
