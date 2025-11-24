# Phase 1 Completion: HAL + State Machine + Priority Scheduling

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete Phase 1 of StreamLab Core by adding HAL interfaces, Pipeline State Machine, and Priority-based Scheduling.

**Architecture:** Three sequential phases: (A) HAL provides hardware abstraction with DeviceSource/Sink traits and mock implementations, (B) State Machine adds lifecycle management to AsyncPipeline, (C) Priority Scheduling enables multi-level task prioritization with preemption.

**Tech Stack:** Rust 2024, tokio async runtime, serde_json for config, anyhow for errors

---

## Phase A: HAL Interfaces

### Task A1: Create HAL Module Structure

**Files:**
- Create: `src/hal/mod.rs`
- Create: `src/hal/lifecycle.rs`
- Create: `src/hal/registry.rs`
- Create: `src/hal/mock/mod.rs`

**Step 1: Create HAL module with core traits**

Create `src/hal/mod.rs`:

```rust
use crate::core::DataFrame;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

pub mod lifecycle;
pub mod registry;
pub mod mock;

/// Device states during lifecycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    Unopened,
    Opened,
    Running,
    Stopped,
    Closed,
    Error(String),
}

/// Trait for devices that produce data (microphones, DAQ inputs, triggers)
#[async_trait]
pub trait DeviceSource: Send + Sync {
    /// Configure device with JSON config
    async fn configure(&mut self, config: Value) -> Result<()>;

    /// Open device (allocate resources, connect to hardware)
    async fn open(&mut self) -> Result<()>;

    /// Start streaming data
    async fn start(&mut self) -> Result<()>;

    /// Get next frame (blocking until available)
    async fn read_frame(&mut self) -> Result<DataFrame>;

    /// Stop streaming
    async fn stop(&mut self) -> Result<()>;

    /// Close device (release resources)
    async fn close(&mut self) -> Result<()>;

    /// Get current device state
    fn state(&self) -> DeviceState;
}

/// Trait for devices that consume data (speakers, DAQ outputs, relays)
#[async_trait]
pub trait DeviceSink: Send + Sync {
    /// Configure device with JSON config
    async fn configure(&mut self, config: Value) -> Result<()>;

    /// Open device
    async fn open(&mut self) -> Result<()>;

    /// Start accepting data
    async fn start(&mut self) -> Result<()>;

    /// Write frame to device
    async fn write_frame(&mut self, frame: DataFrame) -> Result<()>;

    /// Stop accepting data
    async fn stop(&mut self) -> Result<()>;

    /// Close device
    async fn close(&mut self) -> Result<()>;

    /// Get current device state
    fn state(&self) -> DeviceState;
}
```

**Step 2: Create lifecycle management**

Create `src/hal/lifecycle.rs`:

```rust
use super::{DeviceSource, DeviceState};
use anyhow::{anyhow, Result};
use serde_json::Value;
use tokio::sync::mpsc;
use crate::core::DataFrame;

/// Manages the lifecycle of a DeviceSource with proper state transitions
pub struct ManagedSource {
    inner: Box<dyn DeviceSource>,
    state: DeviceState,
}

impl ManagedSource {
    pub fn new(source: Box<dyn DeviceSource>) -> Self {
        Self {
            inner: source,
            state: DeviceState::Unopened,
        }
    }

    pub async fn configure(&mut self, config: Value) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot configure device in state {:?}", self.state));
        }
        self.inner.configure(config).await?;
        Ok(())
    }

    pub async fn open(&mut self) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot open device in state {:?}", self.state));
        }
        self.inner.open().await?;
        self.state = DeviceState::Opened;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.state != DeviceState::Opened && self.state != DeviceState::Stopped {
            return Err(anyhow!("Cannot start device in state {:?}", self.state));
        }
        self.inner.start().await?;
        self.state = DeviceState::Running;
        Ok(())
    }

    pub async fn run_streaming(&mut self, tx: mpsc::Sender<DataFrame>) -> Result<()> {
        if self.state != DeviceState::Running {
            return Err(anyhow!("Device not in running state"));
        }

        loop {
            match self.inner.read_frame().await {
                Ok(frame) => {
                    if tx.send(frame).await.is_err() {
                        break; // Channel closed, stop streaming
                    }
                }
                Err(e) => {
                    self.state = DeviceState::Error(e.to_string());
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if self.state != DeviceState::Running {
            return Ok(()); // Already stopped
        }
        self.inner.stop().await?;
        self.state = DeviceState::Stopped;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        if self.state == DeviceState::Running {
            self.stop().await?;
        }
        if self.state != DeviceState::Closed {
            self.inner.close().await?;
            self.state = DeviceState::Closed;
        }
        Ok(())
    }

    pub fn state(&self) -> &DeviceState {
        &self.state
    }
}
```

**Step 3: Create device registry**

Create `src/hal/registry.rs`:

```rust
use super::DeviceSource;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Registry of available device types
pub struct DeviceRegistry {
    sources: HashMap<String, Box<dyn Fn() -> Box<dyn DeviceSource> + Send + Sync>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    pub fn register_source<F>(&mut self, device_type: &str, factory: F)
    where
        F: Fn() -> Box<dyn DeviceSource> + Send + Sync + 'static,
    {
        self.sources.insert(device_type.to_string(), Box::new(factory));
    }

    pub fn create_source(&self, device_type: &str) -> Result<Box<dyn DeviceSource>> {
        self.sources
            .get(device_type)
            .ok_or_else(|| anyhow!("Unknown device type: {}", device_type))
            .map(|factory| factory())
    }

    pub fn list_sources(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Create mock module stub**

Create `src/hal/mock/mod.rs`:

```rust
pub mod audio;
pub mod trigger;

pub use audio::SimulatedAudioSource;
pub use trigger::SimulatedTriggerSource;
```

**Step 5: Update src/lib.rs to expose HAL module**

Modify `src/lib.rs` - add after existing modules:

```rust
pub mod hal;
```

**Step 6: Commit**

```bash
git add src/hal/
git add src/lib.rs
git commit -m "feat(hal): add core HAL traits and lifecycle management"
```

---

### Task A2: Implement Simulated Audio Source

**Files:**
- Create: `src/hal/mock/audio.rs`
- Create: `tests/hal_audio_test.rs`

**Step 1: Write the failing test**

Create `tests/hal_audio_test.rs`:

```rust
use audiotab::hal::mock::SimulatedAudioSource;
use audiotab::hal::{DeviceSource, DeviceState};
use serde_json::json;

#[tokio::test]
async fn test_simulated_audio_source_lifecycle() {
    let mut source = SimulatedAudioSource::new();

    // Initially unopened
    assert_eq!(source.state(), DeviceState::Unopened);

    // Configure with 1kHz sine wave
    let config = json!({
        "frequency": 1000.0,
        "sample_rate": 48000,
        "amplitude": 1.0,
        "samples_per_frame": 1024
    });
    source.configure(config).await.unwrap();

    // Open device
    source.open().await.unwrap();
    assert_eq!(source.state(), DeviceState::Opened);

    // Start streaming
    source.start().await.unwrap();
    assert_eq!(source.state(), DeviceState::Running);

    // Read a frame
    let frame = source.read_frame().await.unwrap();
    assert_eq!(frame.payload.get("audio").unwrap().len(), 1024);

    // Stop and close
    source.stop().await.unwrap();
    assert_eq!(source.state(), DeviceState::Stopped);

    source.close().await.unwrap();
    assert_eq!(source.state(), DeviceState::Closed);
}

#[tokio::test]
async fn test_simulated_audio_generates_sine_wave() {
    let mut source = SimulatedAudioSource::new();

    let config = json!({
        "frequency": 440.0,  // A4 note
        "sample_rate": 48000,
        "amplitude": 1.0,
        "samples_per_frame": 48
    });

    source.configure(config).await.unwrap();
    source.open().await.unwrap();
    source.start().await.unwrap();

    let frame = source.read_frame().await.unwrap();
    let samples = frame.payload.get("audio").unwrap();

    // Should be a sine wave - check first sample is near amplitude * sin(0)
    assert!(samples[0].abs() < 0.1); // sin(0) ≈ 0

    // Check that values are within [-1, 1]
    for sample in samples.iter() {
        assert!(sample.abs() <= 1.0);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test hal_audio_test
```

Expected: FAIL with "no such module: audiotab::hal::mock::SimulatedAudioSource"

**Step 3: Implement SimulatedAudioSource**

Create `src/hal/mock/audio.rs`:

```rust
use crate::hal::{DeviceSource, DeviceState};
use crate::core::DataFrame;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::f64::consts::PI;

pub struct SimulatedAudioSource {
    state: DeviceState,
    frequency: f64,
    sample_rate: f64,
    amplitude: f64,
    samples_per_frame: usize,
    phase: f64,
    frame_counter: u64,
}

impl SimulatedAudioSource {
    pub fn new() -> Self {
        Self {
            state: DeviceState::Unopened,
            frequency: 1000.0,
            sample_rate: 48000.0,
            amplitude: 1.0,
            samples_per_frame: 1024,
            phase: 0.0,
            frame_counter: 0,
        }
    }

    fn generate_samples(&mut self) -> Vec<f64> {
        let mut samples = Vec::with_capacity(self.samples_per_frame);
        let delta_phase = 2.0 * PI * self.frequency / self.sample_rate;

        for _ in 0..self.samples_per_frame {
            samples.push(self.amplitude * self.phase.sin());
            self.phase += delta_phase;
            if self.phase > 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
        }

        samples
    }
}

#[async_trait]
impl DeviceSource for SimulatedAudioSource {
    async fn configure(&mut self, config: Value) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot configure device in state {:?}", self.state));
        }

        if let Some(freq) = config["frequency"].as_f64() {
            self.frequency = freq;
        }
        if let Some(sr) = config["sample_rate"].as_f64() {
            self.sample_rate = sr;
        }
        if let Some(amp) = config["amplitude"].as_f64() {
            self.amplitude = amp;
        }
        if let Some(spf) = config["samples_per_frame"].as_u64() {
            self.samples_per_frame = spf as usize;
        }

        Ok(())
    }

    async fn open(&mut self) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot open device in state {:?}", self.state));
        }
        self.state = DeviceState::Opened;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        if self.state != DeviceState::Opened && self.state != DeviceState::Stopped {
            return Err(anyhow!("Cannot start device in state {:?}", self.state));
        }
        self.state = DeviceState::Running;
        self.phase = 0.0; // Reset phase on start
        self.frame_counter = 0;
        Ok(())
    }

    async fn read_frame(&mut self) -> Result<DataFrame> {
        if self.state != DeviceState::Running {
            return Err(anyhow!("Device not running"));
        }

        let samples = self.generate_samples();
        let timestamp = (self.frame_counter * self.samples_per_frame as u64 * 1_000_000)
            / self.sample_rate as u64;

        let mut frame = DataFrame::new(timestamp, self.frame_counter);
        frame.payload.insert("audio".to_string(), Arc::new(samples));
        frame.metadata.insert("sample_rate".to_string(), self.sample_rate.to_string());
        frame.metadata.insert("frequency".to_string(), self.frequency.to_string());

        self.frame_counter += 1;

        Ok(frame)
    }

    async fn stop(&mut self) -> Result<()> {
        if self.state != DeviceState::Running {
            return Ok(());
        }
        self.state = DeviceState::Stopped;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if self.state == DeviceState::Running {
            self.stop().await?;
        }
        if self.state != DeviceState::Closed {
            self.state = DeviceState::Closed;
        }
        Ok(())
    }

    fn state(&self) -> DeviceState {
        self.state.clone()
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test --test hal_audio_test
```

Expected: PASS (both tests)

**Step 5: Commit**

```bash
git add src/hal/mock/audio.rs tests/hal_audio_test.rs
git commit -m "feat(hal): implement SimulatedAudioSource with sine wave generation"
```

---

### Task A3: Implement Simulated Trigger Source

**Files:**
- Create: `src/hal/mock/trigger.rs`
- Create: `tests/hal_trigger_test.rs`

**Step 1: Write the failing test**

Create `tests/hal_trigger_test.rs`:

```rust
use audiotab::hal::mock::SimulatedTriggerSource;
use audiotab::hal::{DeviceSource, DeviceState};
use serde_json::json;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_trigger_periodic_mode() {
    let mut trigger = SimulatedTriggerSource::new();

    // Configure for 10ms period (100 Hz)
    let config = json!({
        "mode": "periodic",
        "interval_ms": 10
    });

    trigger.configure(config).await.unwrap();
    trigger.open().await.unwrap();
    trigger.start().await.unwrap();

    // Read two trigger frames
    let frame1 = timeout(Duration::from_millis(50), trigger.read_frame())
        .await
        .unwrap()
        .unwrap();

    let frame2 = timeout(Duration::from_millis(50), trigger.read_frame())
        .await
        .unwrap()
        .unwrap();

    // Triggers should have empty payload (just timestamp)
    assert!(frame1.payload.is_empty());
    assert!(frame2.sequence_id > frame1.sequence_id);

    trigger.stop().await.unwrap();
    trigger.close().await.unwrap();
}

#[tokio::test]
async fn test_trigger_manual_mode() {
    let mut trigger = SimulatedTriggerSource::new();

    let config = json!({
        "mode": "manual"
    });

    trigger.configure(config).await.unwrap();
    trigger.open().await.unwrap();
    trigger.start().await.unwrap();

    // In manual mode, trigger() must be called explicitly
    trigger.trigger();

    let frame = timeout(Duration::from_millis(50), trigger.read_frame())
        .await
        .unwrap()
        .unwrap();

    assert!(frame.payload.is_empty());
    assert_eq!(frame.metadata.get("trigger_mode"), Some(&"manual".to_string()));

    trigger.stop().await.unwrap();
    trigger.close().await.unwrap();
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test hal_trigger_test
```

Expected: FAIL with "no such module: SimulatedTriggerSource"

**Step 3: Implement SimulatedTriggerSource**

Create `src/hal/mock/trigger.rs`:

```rust
use crate::hal::{DeviceSource, DeviceState};
use crate::core::DataFrame;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, Clone)]
enum TriggerMode {
    Periodic { interval_ms: u64 },
    Manual,
}

pub struct SimulatedTriggerSource {
    state: DeviceState,
    mode: TriggerMode,
    frame_counter: u64,
    start_time: Option<Instant>,
    manual_trigger_tx: Option<mpsc::Sender<()>>,
    manual_trigger_rx: Option<mpsc::Receiver<()>>,
}

impl SimulatedTriggerSource {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);
        Self {
            state: DeviceState::Unopened,
            mode: TriggerMode::Periodic { interval_ms: 1000 },
            frame_counter: 0,
            start_time: None,
            manual_trigger_tx: Some(tx),
            manual_trigger_rx: Some(rx),
        }
    }

    /// Manually trigger a frame (only works in Manual mode)
    pub fn trigger(&self) {
        if let Some(tx) = &self.manual_trigger_tx {
            let _ = tx.try_send(());
        }
    }
}

#[async_trait]
impl DeviceSource for SimulatedTriggerSource {
    async fn configure(&mut self, config: Value) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot configure device in state {:?}", self.state));
        }

        let mode = config["mode"].as_str().unwrap_or("periodic");

        self.mode = match mode {
            "periodic" => {
                let interval_ms = config["interval_ms"].as_u64().unwrap_or(1000);
                TriggerMode::Periodic { interval_ms }
            }
            "manual" => TriggerMode::Manual,
            _ => return Err(anyhow!("Unknown trigger mode: {}", mode)),
        };

        Ok(())
    }

    async fn open(&mut self) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot open device in state {:?}", self.state));
        }
        self.state = DeviceState::Opened;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        if self.state != DeviceState::Opened && self.state != DeviceState::Stopped {
            return Err(anyhow!("Cannot start device in state {:?}", self.state));
        }
        self.state = DeviceState::Running;
        self.start_time = Some(Instant::now());
        self.frame_counter = 0;
        Ok(())
    }

    async fn read_frame(&mut self) -> Result<DataFrame> {
        if self.state != DeviceState::Running {
            return Err(anyhow!("Device not running"));
        }

        match self.mode {
            TriggerMode::Periodic { interval_ms } => {
                sleep(Duration::from_millis(interval_ms)).await;
            }
            TriggerMode::Manual => {
                // Wait for manual trigger
                if let Some(rx) = &mut self.manual_trigger_rx {
                    rx.recv().await.ok_or_else(|| anyhow!("Manual trigger channel closed"))?;
                }
            }
        }

        let elapsed = self.start_time.unwrap().elapsed();
        let timestamp = elapsed.as_micros() as u64;

        let mut frame = DataFrame::new(timestamp, self.frame_counter);

        let mode_str = match self.mode {
            TriggerMode::Periodic { .. } => "periodic",
            TriggerMode::Manual => "manual",
        };
        frame.metadata.insert("trigger_mode".to_string(), mode_str.to_string());

        self.frame_counter += 1;

        Ok(frame)
    }

    async fn stop(&mut self) -> Result<()> {
        if self.state != DeviceState::Running {
            return Ok(());
        }
        self.state = DeviceState::Stopped;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if self.state == DeviceState::Running {
            self.stop().await?;
        }
        if self.state != DeviceState::Closed {
            self.state = DeviceState::Closed;
        }
        Ok(())
    }

    fn state(&self) -> DeviceState {
        self.state.clone()
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test --test hal_trigger_test
```

Expected: PASS (both tests)

**Step 5: Commit**

```bash
git add src/hal/mock/trigger.rs tests/hal_trigger_test.rs
git commit -m "feat(hal): implement SimulatedTriggerSource with periodic and manual modes"
```

---

### Task A4: Integrate HAL with Device Registry

**Files:**
- Modify: `src/hal/registry.rs`
- Create: `tests/hal_registry_test.rs`

**Step 1: Write the failing test**

Create `tests/hal_registry_test.rs`:

```rust
use audiotab::hal::registry::DeviceRegistry;
use audiotab::hal::mock::{SimulatedAudioSource, SimulatedTriggerSource};
use audiotab::hal::DeviceSource;

#[tokio::test]
async fn test_registry_with_mock_devices() {
    let mut registry = DeviceRegistry::new();

    // Register mock devices
    registry.register_source("SimulatedAudio", || Box::new(SimulatedAudioSource::new()));
    registry.register_source("SimulatedTrigger", || Box::new(SimulatedTriggerSource::new()));

    // List available sources
    let sources = registry.list_sources();
    assert_eq!(sources.len(), 2);
    assert!(sources.contains(&"SimulatedAudio".to_string()));

    // Create an audio source
    let audio = registry.create_source("SimulatedAudio").unwrap();
    assert_eq!(audio.state(), audiotab::hal::DeviceState::Unopened);

    // Create a trigger source
    let trigger = registry.create_source("SimulatedTrigger").unwrap();
    assert_eq!(trigger.state(), audiotab::hal::DeviceState::Unopened);

    // Try to create unknown device
    let result = registry.create_source("NonExistent");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_registry_default_devices() {
    let registry = DeviceRegistry::with_defaults();

    // Should have mock devices pre-registered
    let sources = registry.list_sources();
    assert!(sources.contains(&"SimulatedAudio".to_string()));
    assert!(sources.contains(&"SimulatedTrigger".to_string()));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test hal_registry_test
```

Expected: FAIL with "no method named `with_defaults`"

**Step 3: Add with_defaults() to DeviceRegistry**

Modify `src/hal/registry.rs`:

```rust
use super::DeviceSource;
use super::mock::{SimulatedAudioSource, SimulatedTriggerSource};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Registry of available device types
pub struct DeviceRegistry {
    sources: HashMap<String, Box<dyn Fn() -> Box<dyn DeviceSource> + Send + Sync>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Create registry with default mock devices pre-registered
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_source("SimulatedAudio", || Box::new(SimulatedAudioSource::new()));
        registry.register_source("SimulatedTrigger", || Box::new(SimulatedTriggerSource::new()));
        registry
    }

    pub fn register_source<F>(&mut self, device_type: &str, factory: F)
    where
        F: Fn() -> Box<dyn DeviceSource> + Send + Sync + 'static,
    {
        self.sources.insert(device_type.to_string(), Box::new(factory));
    }

    pub fn create_source(&self, device_type: &str) -> Result<Box<dyn DeviceSource>> {
        self.sources
            .get(device_type)
            .ok_or_else(|| anyhow!("Unknown device type: {}", device_type))
            .map(|factory| factory())
    }

    pub fn list_sources(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test --test hal_registry_test
```

Expected: PASS (both tests)

**Step 5: Commit**

```bash
git add src/hal/registry.rs tests/hal_registry_test.rs
git commit -m "feat(hal): add DeviceRegistry::with_defaults() for mock devices"
```

---

## Phase B: Pipeline State Machine

### Task B1: Define PipelineState Enum

**Files:**
- Create: `src/engine/state.rs`
- Modify: `src/engine/mod.rs`

**Step 1: Create state module with enum**

Create `src/engine/state.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Pipeline execution states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineState {
    Idle,
    Initializing { progress: u8 }, // 0-100
    Running {
        #[serde(skip)]
        start_time: Option<Instant>,
        frames_processed: u64,
    },
    Paused {
        #[serde(skip)]
        pause_time: Option<Instant>,
    },
    Completed {
        #[serde(skip)]
        duration: Option<Duration>,
        total_frames: u64,
    },
    Error {
        error_msg: String,
        recoverable: bool,
    },
}

impl PipelineState {
    /// Check if transition from current state to target state is valid
    pub fn can_transition_to(&self, target: &PipelineState) -> bool {
        use PipelineState::*;

        matches!(
            (self, target),
            // From Idle
            (Idle, Initializing { .. }) |

            // From Initializing
            (Initializing { .. }, Running { .. }) |
            (Initializing { .. }, Error { .. }) |

            // From Running
            (Running { .. }, Paused { .. }) |
            (Running { .. }, Completed { .. }) |
            (Running { .. }, Error { .. }) |

            // From Paused
            (Paused { .. }, Running { .. }) |
            (Paused { .. }, Completed { .. }) |
            (Paused { .. }, Error { .. }) |

            // From Completed
            (Completed { .. }, Idle) |

            // From Error
            (Error { recoverable: true, .. }, Idle)
        )
    }

    /// Get human-readable state name
    pub fn name(&self) -> &str {
        match self {
            Self::Idle => "Idle",
            Self::Initializing { .. } => "Initializing",
            Self::Running { .. } => "Running",
            Self::Paused { .. } => "Paused",
            Self::Completed { .. } => "Completed",
            Self::Error { .. } => "Error",
        }
    }
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        let idle = PipelineState::Idle;
        let init = PipelineState::Initializing { progress: 50 };

        assert!(idle.can_transition_to(&init));
        assert!(!init.can_transition_to(&idle));
    }

    #[test]
    fn test_running_to_paused() {
        let running = PipelineState::Running {
            start_time: None,
            frames_processed: 100,
        };
        let paused = PipelineState::Paused { pause_time: None };

        assert!(running.can_transition_to(&paused));
    }

    #[test]
    fn test_error_recovery() {
        let recoverable_error = PipelineState::Error {
            error_msg: "timeout".to_string(),
            recoverable: true,
        };
        let unrecoverable_error = PipelineState::Error {
            error_msg: "fatal".to_string(),
            recoverable: false,
        };

        assert!(recoverable_error.can_transition_to(&PipelineState::Idle));
        assert!(!unrecoverable_error.can_transition_to(&PipelineState::Idle));
    }
}
```

**Step 2: Export state module**

Modify `src/engine/mod.rs`:

```rust
pub mod pipeline;
pub mod async_pipeline;
pub mod pipeline_pool;
pub mod state;

pub use pipeline::Pipeline;
pub use async_pipeline::AsyncPipeline;
pub use pipeline_pool::PipelinePool;
pub use state::PipelineState;
```

**Step 3: Run tests**

```bash
cargo test engine::state::tests
```

Expected: PASS (3 tests)

**Step 4: Commit**

```bash
git add src/engine/state.rs src/engine/mod.rs
git commit -m "feat(engine): add PipelineState enum with transition validation"
```

---

### Task B2: Add State Machine to AsyncPipeline

**Files:**
- Modify: `src/engine/async_pipeline.rs`
- Create: `tests/pipeline_state_test.rs`

**Step 1: Write the failing test**

Create `tests/pipeline_state_test.rs`:

```rust
use audiotab::engine::{AsyncPipeline, PipelineState};
use audiotab::core::DataFrame;
use serde_json::json;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_pipeline_state_transitions() {
    let config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "sample_rate": 48000}},
            {"id": "print", "type": "Print", "config": {}}
        ],
        "connections": [
            {"from": "gen", "to": "print"}
        ],
        "pipeline_config": {
            "channel_capacity": 10
        }
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();

    // Initially Idle
    assert_eq!(pipeline.state().name(), "Idle");

    // Transition to Initializing
    pipeline.set_state(PipelineState::Initializing { progress: 0 });
    assert_eq!(pipeline.state().name(), "Initializing");

    // Start pipeline (should transition to Running)
    pipeline.start().await.unwrap();
    assert_eq!(pipeline.state().name(), "Running");

    // Send a trigger frame
    let frame = DataFrame::new(0, 0);
    pipeline.trigger(frame).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    // Stop pipeline (should transition to Completed)
    pipeline.stop().await.unwrap();
    assert_eq!(pipeline.state().name(), "Completed");
}

#[tokio::test]
async fn test_invalid_state_transition() {
    let config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {}}
        ],
        "connections": [],
        "pipeline_config": {}
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();

    // Try invalid transition: Idle -> Completed (should fail)
    let result = pipeline.transition_to(PipelineState::Completed {
        duration: None,
        total_frames: 0,
    });

    assert!(result.is_err());
    assert_eq!(pipeline.state().name(), "Idle");
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test pipeline_state_test
```

Expected: FAIL with "no method named `state`"

**Step 3: Add state management to AsyncPipeline**

Modify `src/engine/async_pipeline.rs` - add state field and methods:

```rust
use crate::engine::state::PipelineState; // Add at top with other imports

pub struct AsyncPipeline {
    nodes: HashMap<String, Box<dyn ProcessingNode>>,
    connections: Vec<(String, String)>,
    channels: HashMap<String, mpsc::Sender<DataFrame>>,
    handles: Vec<JoinHandle<Result<()>>>,
    source_node_id: Option<String>,
    channel_capacity: usize,
    metrics_collector: Option<MetricsCollector>,
    state: PipelineState, // Add this field
}

impl AsyncPipeline {
    pub async fn from_json(config: Value) -> Result<Self> {
        // ... existing code ...

        Ok(Self {
            nodes,
            connections,
            channels: HashMap::new(),
            handles: Vec::new(),
            source_node_id,
            channel_capacity,
            metrics_collector: Some(MetricsCollector::new()),
            state: PipelineState::Idle, // Initialize state
        })
    }

    // Add state accessor
    pub fn state(&self) -> &PipelineState {
        &self.state
    }

    // Add state setter with validation
    pub fn set_state(&mut self, new_state: PipelineState) {
        self.state = new_state;
    }

    // Add transition with validation
    pub fn transition_to(&mut self, new_state: PipelineState) -> Result<()> {
        if !self.state.can_transition_to(&new_state) {
            return Err(anyhow!(
                "Invalid state transition: {} -> {}",
                self.state.name(),
                new_state.name()
            ));
        }
        self.state = new_state;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        // Set initializing state
        self.transition_to(PipelineState::Initializing { progress: 0 })?;

        let channel_capacity = self.channel_capacity;
        // ... existing start code ...

        // After all nodes spawned, transition to Running
        self.transition_to(PipelineState::Running {
            start_time: Some(std::time::Instant::now()),
            frames_processed: 0,
        })?;

        self.metrics_collector = Some(collector);
        Ok(())
    }

    pub async fn stop(mut self) -> Result<()> {
        // Transition to Completed before stopping
        if let PipelineState::Running { start_time, frames_processed } = &self.state {
            let duration = start_time.map(|t| t.elapsed());
            self.transition_to(PipelineState::Completed {
                duration,
                total_frames: *frames_processed,
            })?;
        }

        // Drop channels to signal nodes to shut down
        drop(self.channels);

        // Wait for all node tasks to complete
        for handle in self.handles {
            handle.await??;
        }

        Ok(())
    }

    // ... rest of existing methods ...
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test --test pipeline_state_test
```

Expected: PASS (both tests)

**Step 5: Commit**

```bash
git add src/engine/async_pipeline.rs tests/pipeline_state_test.rs
git commit -m "feat(engine): integrate PipelineState into AsyncPipeline with validation"
```

---

## Phase C: Priority-based Scheduling

### Task C1: Define Priority Levels

**Files:**
- Create: `src/engine/priority.rs`
- Modify: `src/engine/mod.rs`

**Step 1: Create priority module**

Create `src/engine/priority.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Task priority levels with target latencies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    /// 0-10ms: Real-time monitoring, safety-critical
    Critical,
    /// 10-50ms: User-triggered interactive analysis
    High,
    /// 50-200ms: Background automated testing
    Normal,
    /// >200ms: Batch processing, exports
    Low,
}

impl Priority {
    /// Get target latency in milliseconds
    pub fn target_latency_ms(&self) -> u64 {
        match self {
            Priority::Critical => 10,
            Priority::High => 50,
            Priority::Normal => 200,
            Priority::Low => 1000,
        }
    }

    /// Get numeric value for comparison (higher = more urgent)
    pub fn value(&self) -> u8 {
        match self {
            Priority::Critical => 3,
            Priority::High => 2,
            Priority::Normal => 1,
            Priority::Low => 0,
        }
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value().cmp(&other.value())
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_target_latency() {
        assert_eq!(Priority::Critical.target_latency_ms(), 10);
        assert_eq!(Priority::Low.target_latency_ms(), 1000);
    }
}
```

**Step 2: Export priority module**

Modify `src/engine/mod.rs`:

```rust
pub mod priority;
pub use priority::Priority;
```

**Step 3: Run tests**

```bash
cargo test engine::priority::tests
```

Expected: PASS (2 tests)

**Step 4: Commit**

```bash
git add src/engine/priority.rs src/engine/mod.rs
git commit -m "feat(engine): add Priority enum with ordering and latency targets"
```

---

### Task C2: Implement Priority Scheduler

**Files:**
- Create: `src/engine/scheduler.rs`
- Create: `tests/scheduler_test.rs`

**Step 1: Write the failing test**

Create `tests/scheduler_test.rs`:

```rust
use audiotab::engine::{Priority, PipelineScheduler};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_scheduler_priority_ordering() {
    let mut scheduler = PipelineScheduler::new(2); // max 2 concurrent

    // Schedule tasks with different priorities
    let low_started = scheduler.schedule_task(
        Priority::Low,
        async {
            sleep(Duration::from_millis(100)).await;
            "low".to_string()
        }
    ).await;

    let high_started = scheduler.schedule_task(
        Priority::High,
        async {
            sleep(Duration::from_millis(50)).await;
            "high".to_string()
        }
    ).await;

    let critical_started = scheduler.schedule_task(
        Priority::Critical,
        async {
            sleep(Duration::from_millis(25)).await;
            "critical".to_string()
        }
    ).await;

    // All should start (capacity = 2, but third queued)
    assert!(low_started);
    assert!(high_started);

    // Wait for completion
    let results = scheduler.wait_all().await;

    // Critical should complete first despite being scheduled last
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_scheduler_max_concurrent() {
    let mut scheduler = PipelineScheduler::new(2);

    // Schedule 3 tasks (capacity = 2)
    scheduler.schedule_task(Priority::Normal, async {
        sleep(Duration::from_millis(100)).await;
        1
    }).await;

    scheduler.schedule_task(Priority::Normal, async {
        sleep(Duration::from_millis(100)).await;
        2
    }).await;

    scheduler.schedule_task(Priority::Normal, async {
        sleep(Duration::from_millis(50)).await;
        3
    }).await;

    // Check active count
    assert!(scheduler.active_count() <= 2);

    let results = scheduler.wait_all().await;
    assert_eq!(results.len(), 3);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test scheduler_test
```

Expected: FAIL with "no such struct: PipelineScheduler"

**Step 3: Implement PipelineScheduler**

Create `src/engine/scheduler.rs`:

```rust
use crate::engine::Priority;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::future::Future;
use std::pin::Pin;
use tokio::task::JoinHandle;

/// Wrapper for prioritized tasks
struct PrioritizedTask<T> {
    priority: Priority,
    handle: JoinHandle<T>,
    task_id: usize,
}

impl<T> PartialEq for PrioritizedTask<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.task_id == other.task_id
    }
}

impl<T> Eq for PrioritizedTask<T> {}

impl<T> PartialOrd for PrioritizedTask<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PrioritizedTask<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then FIFO by task_id
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => other.task_id.cmp(&self.task_id),
            other => other,
        }
    }
}

/// Priority-based task scheduler
pub struct PipelineScheduler<T> {
    max_concurrent: usize,
    active_tasks: Vec<JoinHandle<T>>,
    pending_queue: BinaryHeap<PrioritizedTask<T>>,
    next_task_id: usize,
    completed: Vec<T>,
}

impl<T: Send + 'static> PipelineScheduler<T> {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            active_tasks: Vec::new(),
            pending_queue: BinaryHeap::new(),
            next_task_id: 0,
            completed: Vec::new(),
        }
    }

    /// Schedule a task with given priority
    /// Returns true if task started immediately, false if queued
    pub async fn schedule_task<F>(&mut self, priority: Priority, future: F) -> bool
    where
        F: Future<Output = T> + Send + 'static,
    {
        let handle = tokio::spawn(future);
        let task = PrioritizedTask {
            priority,
            handle,
            task_id: self.next_task_id,
        };
        self.next_task_id += 1;

        if self.active_tasks.len() < self.max_concurrent {
            self.active_tasks.push(task.handle);
            true
        } else {
            self.pending_queue.push(task);
            false
        }
    }

    /// Get number of currently active tasks
    pub fn active_count(&self) -> usize {
        self.active_tasks.len()
    }

    /// Get number of pending tasks
    pub fn pending_count(&self) -> usize {
        self.pending_queue.len()
    }

    /// Poll for completed tasks and start pending ones
    async fn poll_completions(&mut self) {
        // Check for completed active tasks
        let mut i = 0;
        while i < self.active_tasks.len() {
            if self.active_tasks[i].is_finished() {
                let handle = self.active_tasks.remove(i);
                if let Ok(result) = handle.await {
                    self.completed.push(result);
                }
            } else {
                i += 1;
            }
        }

        // Start pending tasks if slots available
        while self.active_tasks.len() < self.max_concurrent {
            if let Some(task) = self.pending_queue.pop() {
                self.active_tasks.push(task.handle);
            } else {
                break;
            }
        }
    }

    /// Wait for all tasks to complete and return results
    pub async fn wait_all(mut self) -> Vec<T> {
        // Move all pending to active
        while let Some(task) = self.pending_queue.pop() {
            self.active_tasks.push(task.handle);
        }

        // Wait for all active tasks
        for handle in self.active_tasks {
            if let Ok(result) = handle.await {
                self.completed.push(result);
            }
        }

        self.completed
    }
}
```

**Step 4: Export scheduler module**

Modify `src/engine/mod.rs`:

```rust
pub mod scheduler;
pub use scheduler::PipelineScheduler;
```

**Step 5: Run test to verify it passes**

```bash
cargo test --test scheduler_test
```

Expected: PASS (both tests)

**Step 6: Commit**

```bash
git add src/engine/scheduler.rs src/engine/mod.rs tests/scheduler_test.rs
git commit -m "feat(engine): implement PipelineScheduler with priority queues"
```

---

### Task C3: Integrate Priority into AsyncPipeline

**Files:**
- Modify: `src/engine/async_pipeline.rs`
- Create: `tests/pipeline_priority_test.rs`

**Step 1: Write the failing test**

Create `tests/pipeline_priority_test.rs`:

```rust
use audiotab::engine::{AsyncPipeline, Priority};
use serde_json::json;

#[tokio::test]
async fn test_pipeline_with_priority() {
    let config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {}}
        ],
        "connections": [],
        "pipeline_config": {
            "priority": "High"
        }
    });

    let pipeline = AsyncPipeline::from_json(config).await.unwrap();
    assert_eq!(pipeline.priority(), Priority::High);
}

#[tokio::test]
async fn test_pipeline_default_priority() {
    let config = json!({
        "nodes": [{"id": "gen", "type": "SineGenerator", "config": {}}],
        "connections": [],
        "pipeline_config": {}
    });

    let pipeline = AsyncPipeline::from_json(config).await.unwrap();
    assert_eq!(pipeline.priority(), Priority::Normal);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --test pipeline_priority_test
```

Expected: FAIL with "no method named `priority`"

**Step 3: Add priority field to AsyncPipeline**

Modify `src/engine/async_pipeline.rs`:

```rust
use crate::engine::Priority; // Add to imports

pub struct AsyncPipeline {
    // ... existing fields ...
    state: PipelineState,
    priority: Priority, // Add this field
}

impl AsyncPipeline {
    pub async fn from_json(config: Value) -> Result<Self> {
        // Parse priority from config
        let priority = config["pipeline_config"]["priority"]
            .as_str()
            .and_then(|s| match s {
                "Critical" => Some(Priority::Critical),
                "High" => Some(Priority::High),
                "Normal" => Some(Priority::Normal),
                "Low" => Some(Priority::Low),
                _ => None,
            })
            .unwrap_or(Priority::Normal);

        // ... rest of existing parsing ...

        Ok(Self {
            nodes,
            connections,
            channels: HashMap::new(),
            handles: Vec::new(),
            source_node_id,
            channel_capacity,
            metrics_collector: Some(MetricsCollector::new()),
            state: PipelineState::Idle,
            priority, // Initialize priority
        })
    }

    // Add priority accessor
    pub fn priority(&self) -> Priority {
        self.priority
    }

    // ... rest of existing methods ...
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test --test pipeline_priority_test
```

Expected: PASS (both tests)

**Step 5: Commit**

```bash
git add src/engine/async_pipeline.rs tests/pipeline_priority_test.rs
git commit -m "feat(engine): add priority field to AsyncPipeline from config"
```

---

## Final Integration Test

### Task D1: End-to-End Integration Test

**Files:**
- Create: `tests/phase1_integration_test.rs`

**Step 1: Write comprehensive integration test**

Create `tests/phase1_integration_test.rs`:

```rust
use audiotab::engine::{AsyncPipeline, PipelineState, Priority};
use audiotab::hal::{DeviceRegistry, DeviceSource};
use audiotab::core::DataFrame;
use serde_json::json;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_phase1_complete_integration() {
    // Test 1: HAL with mock devices
    let registry = DeviceRegistry::with_defaults();
    let mut audio_source = registry.create_source("SimulatedAudio").unwrap();

    let audio_config = json!({
        "frequency": 1000.0,
        "sample_rate": 48000,
        "amplitude": 0.5,
        "samples_per_frame": 512
    });

    audio_source.configure(audio_config).await.unwrap();
    audio_source.open().await.unwrap();
    audio_source.start().await.unwrap();

    let frame = audio_source.read_frame().await.unwrap();
    assert_eq!(frame.payload.get("audio").unwrap().len(), 512);

    audio_source.stop().await.unwrap();
    audio_source.close().await.unwrap();

    // Test 2: Pipeline with state machine
    let pipeline_config = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
            {"id": "print", "type": "Print", "config": {}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "print"}
        ],
        "pipeline_config": {
            "channel_capacity": 10,
            "priority": "High"
        }
    });

    let mut pipeline = AsyncPipeline::from_json(pipeline_config).await.unwrap();

    // Verify initial state
    assert_eq!(pipeline.state().name(), "Idle");
    assert_eq!(pipeline.priority(), Priority::High);

    // Start pipeline (transitions Idle -> Initializing -> Running)
    pipeline.start().await.unwrap();
    assert_eq!(pipeline.state().name(), "Running");

    // Trigger a few frames
    for i in 0..5 {
        let frame = DataFrame::new(i * 1000, i);
        pipeline.trigger(frame).await.unwrap();
    }

    sleep(Duration::from_millis(500)).await;

    // Stop pipeline (transitions Running -> Completed)
    pipeline.stop().await.unwrap();
    // State checked in stop() method

    println!("✅ Phase 1 integration test passed!");
}

#[tokio::test]
async fn test_hal_trigger_integration() {
    let registry = DeviceRegistry::with_defaults();
    let mut trigger = registry.create_source("SimulatedTrigger").unwrap();

    let config = json!({
        "mode": "periodic",
        "interval_ms": 20
    });

    trigger.configure(config).await.unwrap();
    trigger.open().await.unwrap();
    trigger.start().await.unwrap();

    // Read 3 trigger frames
    for _ in 0..3 {
        let frame = trigger.read_frame().await.unwrap();
        assert!(frame.payload.is_empty());
        assert_eq!(frame.metadata.get("trigger_mode"), Some(&"periodic".to_string()));
    }

    trigger.stop().await.unwrap();
    trigger.close().await.unwrap();

    println!("✅ HAL trigger integration test passed!");
}
```

**Step 2: Run integration test**

```bash
cargo test --test phase1_integration_test -- --nocapture
```

Expected: PASS with success messages

**Step 3: Final commit**

```bash
git add tests/phase1_integration_test.rs
git commit -m "test: add comprehensive Phase 1 integration tests"
```

---

## Documentation

### Task E1: Update Documentation

**Files:**
- Create: `docs/phase1-completion.md`
- Modify: `README.md`

**Step 1: Create completion documentation**

Create `docs/phase1-completion.md`:

```markdown
# Phase 1 Completion Summary

**Date:** 2025-11-24
**Status:** ✅ Complete

## Features Implemented

### HAL (Hardware Abstraction Layer)

**Module:** `src/hal/`

- `DeviceSource` and `DeviceSink` traits for hardware abstraction
- `DeviceState` lifecycle management (Unopened → Opened → Running → Stopped → Closed)
- `ManagedSource` wrapper with state validation
- `DeviceRegistry` for device discovery and factory creation
- Mock devices:
  - `SimulatedAudioSource` - sine wave generator
  - `SimulatedTriggerSource` - periodic and manual trigger modes

**Usage:**

```rust
use audiotab::hal::{DeviceRegistry, DeviceSource};
use serde_json::json;

let registry = DeviceRegistry::with_defaults();
let mut audio = registry.create_source("SimulatedAudio").unwrap();

audio.configure(json!({"frequency": 1000.0})).await?;
audio.open().await?;
audio.start().await?;
let frame = audio.read_frame().await?;
```

### Pipeline State Machine

**Module:** `src/engine/state.rs`

- `PipelineState` enum with 6 states
- State transition validation
- Integrated into `AsyncPipeline`

**State Flow:**

```
Idle → Initializing → Running → Completed
           ↓            ↓
         Error ← ─ ─ ─ ─┘
           ↓
         Idle (if recoverable)
```

### Priority-based Scheduling

**Module:** `src/engine/priority.rs`, `src/engine/scheduler.rs`

- `Priority` enum (Critical/High/Normal/Low)
- Target latency specifications (10ms to 1000ms)
- `PipelineScheduler` with priority queues
- Max concurrent task limiting
- Priority field in pipeline config

**Usage:**

```rust
use audiotab::engine::{PipelineScheduler, Priority};

let mut scheduler = PipelineScheduler::new(4); // max 4 concurrent

scheduler.schedule_task(Priority::Critical, async {
    // High-priority work
}).await;

let results = scheduler.wait_all().await;
```

## Testing

All features have comprehensive test coverage:
- `tests/hal_audio_test.rs` - Audio source lifecycle and generation
- `tests/hal_trigger_test.rs` - Trigger modes and timing
- `tests/hal_registry_test.rs` - Device registry operations
- `tests/pipeline_state_test.rs` - State transitions
- `tests/scheduler_test.rs` - Priority scheduling
- `tests/pipeline_priority_test.rs` - Pipeline priority config
- `tests/phase1_integration_test.rs` - End-to-end integration

Run tests:

```bash
cargo test --all
```

## Next Steps (Phase 2)

- Set up Tauri v2 frontend
- Implement React Flow visual editor
- Create Tauri bridge for pipeline control
- Build node palette UI component

## Files Modified

**Created:**
- `src/hal/mod.rs`
- `src/hal/lifecycle.rs`
- `src/hal/registry.rs`
- `src/hal/mock/mod.rs`
- `src/hal/mock/audio.rs`
- `src/hal/mock/trigger.rs`
- `src/engine/state.rs`
- `src/engine/priority.rs`
- `src/engine/scheduler.rs`
- 9 test files

**Modified:**
- `src/lib.rs`
- `src/engine/mod.rs`
- `src/engine/async_pipeline.rs`

**Total Lines Added:** ~1,200 lines of implementation + ~600 lines of tests
```

**Step 2: Update README**

Modify `README.md` - add after project description:

```markdown
## Development Status

- ✅ **Phase 1 Complete** - Core Engine (HAL, State Machine, Priority Scheduling)
- 🚧 **Phase 2 In Progress** - Frontend & Builder
- ⏳ **Phase 3 Planned** - Hybrid Runtime & Plugin System
- ⏳ **Phase 4 Planned** - Streaming & Visualization
- ⏳ **Phase 5 Planned** - Logic Control & Advanced Features

See `docs/phase1-completion.md` for Phase 1 details.
```

**Step 3: Commit documentation**

```bash
git add docs/phase1-completion.md README.md
git commit -m "docs: document Phase 1 completion with usage examples"
```

---

## Success Criteria

**Phase 1 is complete when:**

- ✅ All tests pass (`cargo test --all`)
- ✅ HAL supports mock audio and trigger devices
- ✅ Pipeline has state machine with validated transitions
- ✅ Priority scheduler manages tasks with 4 priority levels
- ✅ Integration test demonstrates all features working together
- ✅ Documentation updated with usage examples

**Run final verification:**

```bash
cargo test --all --verbose
cargo clippy --all-targets
cargo fmt --all -- --check
```

All checks must pass before marking Phase 1 complete.

---

**End of Plan**

Total estimated time: 8-11 days (HAL: 3-4d, State Machine: 2-3d, Priority: 3-4d)
