# Phase 5: Hardware Abstraction Layer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a hardware abstraction layer that enables streaming analysis from diverse hardware sources (audio, vibration, TCP) through a unified trait-based interface with efficient channel ping-pong buffer management.

**Architecture:** Trait-based plugin system where developers implement `HardwareDriver` for discovery and `Device` for streaming. Built-in audio driver uses CPAL for cross-platform support. Channel ping-pong pattern with bounded channels for zero-copy buffer reuse. Hardware manager UI separate from flow graph for complexity isolation.

**Tech Stack:** Rust (async-trait, crossbeam-channel, CPAL), React/TypeScript (hardware manager UI), Tauri (backend integration)

---

## Phase 5.1: Core HAL Infrastructure

### Task 1: HAL Module Structure

**Files:**
- Create: `src/hal/mod.rs`
- Create: `src/hal/traits.rs`
- Create: `src/hal/types.rs`
- Modify: `src/lib.rs`

**Step 1: Create HAL module declaration**

In `src/lib.rs`, add after `pub mod visualization;`:

```rust
pub mod hal;
```

**Step 2: Create traits module with HardwareDriver trait**

Create `src/hal/traits.rs`:

```rust
use async_trait::async_trait;
use anyhow::Result;
use super::types::{DeviceInfo, DeviceConfig, DeviceCapabilities, DeviceChannels, HardwareType};

/// Trait implemented by hardware drivers for device discovery and creation
#[async_trait]
pub trait HardwareDriver: Send + Sync {
    /// Unique driver identifier (e.g., "cpal-audio", "tcp-stream")
    fn driver_id(&self) -> &str;

    /// Hardware classification for framework support level
    fn hardware_type(&self) -> HardwareType {
        HardwareType::Special
    }

    /// Discover available devices (async for network discovery)
    async fn discover_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Create device instance with configuration
    fn create_device(
        &self,
        device_id: &str,
        config: DeviceConfig,
    ) -> Result<Box<dyn Device>>;
}

/// Trait implemented by device instances for data streaming
#[async_trait]
pub trait Device: Send {
    /// Start streaming data
    async fn start(&mut self) -> Result<()>;

    /// Stop streaming
    async fn stop(&mut self) -> Result<()>;

    /// Get channels for buffer ping-pong
    fn get_channels(&mut self) -> DeviceChannels;

    /// Device capabilities and metadata
    fn capabilities(&self) -> DeviceCapabilities;

    /// Check if device is currently streaming
    fn is_streaming(&self) -> bool;
}
```

**Step 3: Create types module with core data structures**

Create `src/hal/types.rs`:

```rust
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

/// Hardware classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareType {
    /// Full framework support - time-series samples
    Acoustic,
    /// Developer-defined usage
    Special,
}

/// Device discovery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub hardware_type: HardwareType,
    pub driver_id: String,
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub name: String,
    pub sample_rate: u64,
    pub format: SampleFormat,
    pub buffer_size: usize,
    pub channel_mapping: ChannelMapping,
    pub calibration: Calibration,
}

/// Sample data format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleFormat {
    I16,  // 16-bit PCM
    I24,  // 24-bit
    I32,  // 32-bit integer
    F32,  // 32-bit float
    F64,  // 64-bit float
    U8,   // 8-bit unsigned
}

/// Channel mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMapping {
    pub physical_channels: usize,
    pub virtual_channels: usize,
    pub routing: Vec<ChannelRoute>,
}

impl Default for ChannelMapping {
    fn default() -> Self {
        Self {
            physical_channels: 0,
            virtual_channels: 0,
            routing: Vec::new(),
        }
    }
}

/// Channel routing rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelRoute {
    Direct(usize),          // Phys[i] -> Virt[i]
    Reorder(Vec<usize>),    // Phys[1,2,3] -> Virt[3,2,1]
    Merge(Vec<usize>),      // Phys[1,2,3] -> Virt[1]
    Duplicate(usize),       // Phys[1] -> Virt[1,2,3]
}

/// Calibration settings
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Calibration {
    pub gain: f64,    // Multiply for voltage
    pub offset: f64,  // Add for SPL
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            gain: 1.0,
            offset: 0.0,
        }
    }
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub can_input: bool,
    pub can_output: bool,
    pub supported_formats: Vec<SampleFormat>,
    pub supported_sample_rates: Vec<u64>,
    pub max_channels: usize,
}

/// Channels for buffer ping-pong pattern
#[derive(Clone)]
pub struct DeviceChannels {
    /// Receive filled buffers from hardware
    pub filled_rx: Receiver<PacketBuffer>,
    /// Send empty buffers back to hardware
    pub empty_tx: Sender<PacketBuffer>,
}

/// Packet buffer for streaming data
#[derive(Debug, Clone)]
pub struct PacketBuffer {
    pub data: SampleData,
    pub sample_rate: u64,
    pub num_channels: usize,
    pub timestamp: Option<u64>,  // Nanoseconds
}

/// Sample data in native format
#[derive(Debug, Clone)]
pub enum SampleData {
    I16(Vec<i16>),
    I24(Vec<u8>),  // 3 bytes per sample
    I32(Vec<i32>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    U8(Vec<u8>),
    Bytes(Vec<u8>),  // For special hardware
}

impl PacketBuffer {
    pub fn new(format: SampleFormat, buffer_size: usize, num_channels: usize) -> Self {
        let capacity = buffer_size * num_channels;
        let data = match format {
            SampleFormat::I16 => SampleData::I16(vec![0i16; capacity]),
            SampleFormat::I24 => SampleData::I24(vec![0u8; capacity * 3]),
            SampleFormat::I32 => SampleData::I32(vec![0i32; capacity]),
            SampleFormat::F32 => SampleData::F32(vec![0.0f32; capacity]),
            SampleFormat::F64 => SampleData::F64(vec![0.0f64; capacity]),
            SampleFormat::U8 => SampleData::U8(vec![0u8; capacity]),
        };

        Self {
            data,
            sample_rate: 48000,  // Default
            num_channels,
            timestamp: None,
        }
    }

    /// Derive timestamp from packet index if not provided
    pub fn derive_timestamp(&self, packet_index: u64) -> u64 {
        if let Some(ts) = self.timestamp {
            return ts;
        }

        let samples_per_packet = match &self.data {
            SampleData::I16(v) => v.len() / self.num_channels,
            SampleData::I32(v) => v.len() / self.num_channels,
            SampleData::F32(v) => v.len() / self.num_channels,
            SampleData::F64(v) => v.len() / self.num_channels,
            SampleData::U8(v) => v.len() / self.num_channels,
            SampleData::I24(v) => (v.len() / 3) / self.num_channels,
            SampleData::Bytes(_) => 0,
        };

        let samples_elapsed = packet_index * samples_per_packet as u64;
        (samples_elapsed * 1_000_000_000) / self.sample_rate
    }
}
```

**Step 4: Create HAL module root**

Create `src/hal/mod.rs`:

```rust
pub mod traits;
pub mod types;

pub use traits::{HardwareDriver, Device};
pub use types::{
    HardwareType, DeviceInfo, DeviceConfig, DeviceCapabilities,
    DeviceChannels, PacketBuffer, SampleData, SampleFormat,
    ChannelMapping, ChannelRoute, Calibration,
};
```

**Step 5: Add dependencies to Cargo.toml**

In root `Cargo.toml`, add:

```toml
[dependencies]
# ... existing dependencies
async-trait = "0.1"
crossbeam-channel = "0.5"
cpal = "0.15"
```

**Step 6: Verify compilation**

```bash
cargo check
```

Expected: Compiles successfully

**Step 7: Commit**

```bash
git add src/lib.rs src/hal/mod.rs src/hal/traits.rs src/hal/types.rs Cargo.toml Cargo.lock
git commit -m "feat(hal): add core HAL traits and types

- HardwareDriver trait for device discovery
- Device trait for streaming
- PacketBuffer with SampleData enum
- Channel ping-pong types
- Hardware classification (Acoustic/Special)"
```

---

### Task 2: Hardware Registry

**Files:**
- Create: `src/hal/registry.rs`
- Modify: `src/hal/mod.rs`
- Create: `tests/hal_registry_tests.rs`

**Step 1: Write failing test for registry**

Create `tests/hal_registry_tests.rs`:

```rust
use audiotab::hal::{HardwareDriver, HardwareRegistry};

#[tokio::test]
async fn test_registry_register_and_list() {
    let mut registry = HardwareRegistry::new();

    // Initially empty
    assert_eq!(registry.list_drivers().len(), 0);

    // Register mock driver (will create in next test)
    // Skipped for now - test will fail
    assert!(false, "Registry not implemented");
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_registry_register_and_list
```

Expected: FAIL with "Registry not implemented"

**Step 3: Implement HardwareRegistry**

Create `src/hal/registry.rs`:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use super::traits::HardwareDriver;
use super::types::{DeviceInfo, DeviceConfig};
use super::Device;

/// Central registry for hardware drivers
pub struct HardwareRegistry {
    drivers: HashMap<String, Arc<dyn HardwareDriver>>,
}

impl HardwareRegistry {
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
        }
    }

    /// Register a hardware driver
    pub fn register(&mut self, driver: impl HardwareDriver + 'static) {
        let driver_id = driver.driver_id().to_string();
        self.drivers.insert(driver_id, Arc::new(driver));
    }

    /// List all registered drivers
    pub fn list_drivers(&self) -> Vec<String> {
        self.drivers.keys().cloned().collect()
    }

    /// Get driver by ID
    pub fn get_driver(&self, driver_id: &str) -> Option<Arc<dyn HardwareDriver>> {
        self.drivers.get(driver_id).cloned()
    }

    /// Discover devices from all drivers
    pub async fn discover_all(&self) -> Result<Vec<DeviceInfo>> {
        let mut all_devices = Vec::new();

        for driver in self.drivers.values() {
            match driver.discover_devices().await {
                Ok(devices) => all_devices.extend(devices),
                Err(e) => eprintln!("Driver {} discovery failed: {}", driver.driver_id(), e),
            }
        }

        Ok(all_devices)
    }

    /// Create device from any registered driver
    pub fn create_device(
        &self,
        driver_id: &str,
        device_id: &str,
        config: DeviceConfig,
    ) -> Result<Box<dyn Device>> {
        let driver = self.get_driver(driver_id)
            .ok_or_else(|| anyhow::anyhow!("Driver {} not found", driver_id))?;

        driver.create_device(device_id, config)
    }
}

impl Default for HardwareRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Export registry from hal module**

In `src/hal/mod.rs`, add:

```rust
pub mod registry;

pub use registry::HardwareRegistry;
```

**Step 5: Update test with mock driver**

Replace `tests/hal_registry_tests.rs` content:

```rust
use audiotab::hal::*;
use async_trait::async_trait;
use anyhow::Result;

struct MockDriver;

#[async_trait]
impl HardwareDriver for MockDriver {
    fn driver_id(&self) -> &str {
        "mock-driver"
    }

    async fn discover_devices(&self) -> Result<Vec<DeviceInfo>> {
        Ok(vec![DeviceInfo {
            id: "mock-device-1".to_string(),
            name: "Mock Device".to_string(),
            hardware_type: HardwareType::Acoustic,
            driver_id: "mock-driver".to_string(),
        }])
    }

    fn create_device(&self, _id: &str, _config: DeviceConfig) -> Result<Box<dyn Device>> {
        anyhow::bail!("Not implemented for mock")
    }
}

#[tokio::test]
async fn test_registry_register_and_list() {
    let mut registry = HardwareRegistry::new();

    // Initially empty
    assert_eq!(registry.list_drivers().len(), 0);

    // Register mock driver
    registry.register(MockDriver);
    assert_eq!(registry.list_drivers().len(), 1);
    assert!(registry.list_drivers().contains(&"mock-driver".to_string()));
}

#[tokio::test]
async fn test_registry_discover_all() {
    let mut registry = HardwareRegistry::new();
    registry.register(MockDriver);

    let devices = registry.discover_all().await.unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, "mock-device-1");
    assert_eq!(devices[0].hardware_type, HardwareType::Acoustic);
}
```

**Step 6: Add tokio dev-dependency**

In `Cargo.toml`:

```toml
[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
```

**Step 7: Run tests to verify they pass**

```bash
cargo test hal_registry
```

Expected: PASS (2 tests)

**Step 8: Commit**

```bash
git add src/hal/registry.rs src/hal/mod.rs tests/hal_registry_tests.rs Cargo.toml Cargo.lock
git commit -m "feat(hal): implement hardware registry

- HardwareRegistry for driver management
- Driver registration and discovery
- Tests with mock driver"
```

---

## Phase 5.2: Audio Driver (CPAL)

### Task 3: CPAL Audio Driver - Discovery

**Files:**
- Create: `src/hal/drivers/mod.rs`
- Create: `src/hal/drivers/audio.rs`
- Modify: `src/hal/mod.rs`
- Create: `tests/hal_audio_driver_tests.rs`

**Step 1: Write failing test for audio discovery**

Create `tests/hal_audio_driver_tests.rs`:

```rust
use audiotab::hal::*;

#[tokio::test]
async fn test_audio_driver_discovery() {
    let driver = AudioDriver::new();
    assert_eq!(driver.driver_id(), "cpal-audio");

    let devices = driver.discover_devices().await.unwrap();
    // Should find at least default input/output
    assert!(devices.len() >= 1, "No audio devices found");

    // Verify devices have acoustic type
    for device in devices {
        assert_eq!(device.hardware_type, HardwareType::Acoustic);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_audio_driver_discovery
```

Expected: FAIL with "AudioDriver not found"

**Step 3: Create audio driver module**

Create `src/hal/drivers/mod.rs`:

```rust
pub mod audio;

pub use audio::AudioDriver;
```

In `src/hal/mod.rs`, add:

```rust
pub mod drivers;

pub use drivers::AudioDriver;
```

**Step 4: Implement CPAL audio driver**

Create `src/hal/drivers/audio.rs`:

```rust
use async_trait::async_trait;
use anyhow::Result;
use cpal::traits::{HostTrait, DeviceTrait};
use crate::hal::traits::HardwareDriver;
use crate::hal::types::*;
use crate::hal::Device;

pub struct AudioDriver {
    host: cpal::Host,
}

impl AudioDriver {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }
}

#[async_trait]
impl HardwareDriver for AudioDriver {
    fn driver_id(&self) -> &str {
        "cpal-audio"
    }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::Acoustic
    }

    async fn discover_devices(&self) -> Result<Vec<DeviceInfo>> {
        let mut devices = Vec::new();

        // Input devices
        if let Ok(input_devices) = self.host.input_devices() {
            for (idx, device) in input_devices.enumerate() {
                if let Ok(name) = device.name() {
                    devices.push(DeviceInfo {
                        id: format!("input-{}", idx),
                        name: format!("{} (Input)", name),
                        hardware_type: HardwareType::Acoustic,
                        driver_id: "cpal-audio".to_string(),
                    });
                }
            }
        }

        // Output devices
        if let Ok(output_devices) = self.host.output_devices() {
            for (idx, device) in output_devices.enumerate() {
                if let Ok(name) = device.name() {
                    devices.push(DeviceInfo {
                        id: format!("output-{}", idx),
                        name: format!("{} (Output)", name),
                        hardware_type: HardwareType::Acoustic,
                        driver_id: "cpal-audio".to_string(),
                    });
                }
            }
        }

        Ok(devices)
    }

    fn create_device(&self, _id: &str, _config: DeviceConfig) -> Result<Box<dyn Device>> {
        // Will implement in next task
        anyhow::bail!("Not implemented yet")
    }
}

impl Default for AudioDriver {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cargo test test_audio_driver_discovery
```

Expected: PASS

**Step 6: Commit**

```bash
git add src/hal/drivers/ src/hal/mod.rs tests/hal_audio_driver_tests.rs
git commit -m "feat(hal): add CPAL audio driver with discovery

- AudioDriver using cpal for cross-platform audio
- Discovers input and output devices
- Returns acoustic hardware type"
```

---

### Task 4: Audio Device - Channel Ping-Pong Setup

**Files:**
- Create: `src/hal/drivers/audio_device.rs`
- Modify: `src/hal/drivers/audio.rs`
- Modify: `src/hal/drivers/mod.rs`

**Step 1: Create audio device structure**

Create `src/hal/drivers/audio_device.rs`:

```rust
use async_trait::async_trait;
use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::hal::{Device, DeviceChannels, DeviceCapabilities, PacketBuffer, SampleFormat};

pub struct AudioDevice {
    device_name: String,
    filled_rx: Receiver<PacketBuffer>,
    empty_tx: Sender<PacketBuffer>,
    is_streaming: Arc<AtomicBool>,
    capabilities: DeviceCapabilities,
}

impl AudioDevice {
    pub fn new(
        device_name: String,
        sample_rate: u64,
        format: SampleFormat,
        buffer_size: usize,
        num_channels: usize,
    ) -> Result<Self> {
        let (filled_tx, filled_rx) = bounded(2);  // Double buffer
        let (empty_tx, empty_rx) = bounded(2);

        // Pre-allocate ping-pong buffers
        for _ in 0..2 {
            let buffer = PacketBuffer::new(format, buffer_size, num_channels);
            empty_tx.send(buffer).map_err(|e| anyhow::anyhow!("Failed to send buffer: {}", e))?;
        }

        let capabilities = DeviceCapabilities {
            can_input: true,
            can_output: false,
            supported_formats: vec![SampleFormat::F32, SampleFormat::I16],
            supported_sample_rates: vec![44100, 48000, 96000, 192000],
            max_channels: 32,
        };

        Ok(Self {
            device_name,
            filled_rx,
            empty_tx,
            is_streaming: Arc::new(AtomicBool::new(false)),
            capabilities,
        })
    }
}

#[async_trait]
impl Device for AudioDevice {
    async fn start(&mut self) -> Result<()> {
        self.is_streaming.store(true, Ordering::Relaxed);
        // CPAL stream start will be implemented in next task
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.is_streaming.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn get_channels(&mut self) -> DeviceChannels {
        DeviceChannels {
            filled_rx: self.filled_rx.clone(),
            empty_tx: self.empty_tx.clone(),
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        self.capabilities.clone()
    }

    fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }
}
```

**Step 2: Update audio driver to create device**

In `src/hal/drivers/audio.rs`, update `create_device`:

```rust
use super::audio_device::AudioDevice;

// In AudioDriver impl:
fn create_device(&self, device_id: &str, config: DeviceConfig) -> Result<Box<dyn Device>> {
    let device = AudioDevice::new(
        config.name,
        config.sample_rate,
        config.format,
        config.buffer_size,
        config.channel_mapping.physical_channels,
    )?;

    Ok(Box::new(device))
}
```

**Step 3: Export audio_device module**

In `src/hal/drivers/mod.rs`:

```rust
pub mod audio;
pub mod audio_device;

pub use audio::AudioDriver;
pub use audio_device::AudioDevice;
```

**Step 4: Test device creation**

Add to `tests/hal_audio_driver_tests.rs`:

```rust
#[tokio::test]
async fn test_audio_device_creation() {
    use audiotab::hal::*;

    let driver = AudioDriver::new();
    let config = DeviceConfig {
        name: "Test Device".to_string(),
        sample_rate: 48000,
        format: SampleFormat::F32,
        buffer_size: 1024,
        channel_mapping: ChannelMapping::default(),
        calibration: Calibration::default(),
    };

    let mut device = driver.create_device("test-id", config).unwrap();

    // Should not be streaming initially
    assert!(!device.is_streaming());

    // Get channels
    let channels = device.get_channels();
    assert!(channels.filled_rx.is_empty());
}
```

**Step 5: Run test**

```bash
cargo test test_audio_device_creation
```

Expected: PASS

**Step 6: Commit**

```bash
git add src/hal/drivers/audio_device.rs src/hal/drivers/audio.rs src/hal/drivers/mod.rs tests/hal_audio_driver_tests.rs
git commit -m "feat(hal): implement audio device with channel ping-pong

- AudioDevice with double-buffered channels
- Pre-allocated packet buffers
- Device creation through AudioDriver"
```

---

### Task 5: Audio Streaming with CPAL

**Files:**
- Modify: `src/hal/drivers/audio_device.rs`
- Create: `tests/hal_audio_streaming_tests.rs`

**Step 1: Write streaming test**

Create `tests/hal_audio_streaming_tests.rs`:

```rust
use audiotab::hal::*;
use std::time::Duration;

#[tokio::test]
async fn test_audio_streaming_basic() {
    let driver = AudioDriver::new();

    // Discover default input device
    let devices = driver.discover_devices().await.unwrap();
    let input_device = devices.iter()
        .find(|d| d.name.contains("Input"))
        .expect("No input device found");

    let config = DeviceConfig {
        name: input_device.name.clone(),
        sample_rate: 48000,
        format: SampleFormat::F32,
        buffer_size: 1024,
        channel_mapping: ChannelMapping {
            physical_channels: 2,
            virtual_channels: 2,
            routing: vec![],
        },
        calibration: Calibration::default(),
    };

    let mut device = driver.create_device(&input_device.id, config).unwrap();
    let mut channels = device.get_channels();

    // Start streaming
    device.start().await.unwrap();
    assert!(device.is_streaming());

    // Wait for a buffer (with timeout)
    tokio::select! {
        buffer = tokio::task::spawn_blocking(move || channels.filled_rx.recv()) => {
            let packet = buffer.unwrap().unwrap();
            assert!(packet.sample_rate > 0);
            println!("Received audio packet: {} samples", match &packet.data {
                SampleData::F32(v) => v.len(),
                _ => 0,
            });
        }
        _ = tokio::time::sleep(Duration::from_secs(2)) => {
            panic!("Timeout waiting for audio packet");
        }
    }

    device.stop().await.unwrap();
}
```

**Step 2: Implement CPAL streaming in AudioDevice**

Replace `src/hal/drivers/audio_device.rs` with:

```rust
use async_trait::async_trait;
use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig, SampleFormat as CpalFormat};
use crate::hal::{Device, DeviceChannels, DeviceCapabilities, PacketBuffer, SampleData, SampleFormat};

pub struct AudioDevice {
    device_name: String,
    sample_rate: u64,
    format: SampleFormat,
    buffer_size: usize,
    num_channels: usize,
    filled_tx: Sender<PacketBuffer>,
    filled_rx: Receiver<PacketBuffer>,
    empty_tx: Sender<PacketBuffer>,
    empty_rx: Receiver<PacketBuffer>,
    is_streaming: Arc<AtomicBool>,
    capabilities: DeviceCapabilities,
    stream: Option<Stream>,
}

impl AudioDevice {
    pub fn new(
        device_name: String,
        sample_rate: u64,
        format: SampleFormat,
        buffer_size: usize,
        num_channels: usize,
    ) -> Result<Self> {
        let (filled_tx, filled_rx) = bounded(2);
        let (empty_tx, empty_rx) = bounded(2);

        // Pre-allocate buffers
        for _ in 0..2 {
            let buffer = PacketBuffer::new(format, buffer_size, num_channels);
            empty_tx.send(buffer)
                .map_err(|e| anyhow::anyhow!("Failed to send buffer: {}", e))?;
        }

        let capabilities = DeviceCapabilities {
            can_input: true,
            can_output: false,
            supported_formats: vec![SampleFormat::F32, SampleFormat::I16],
            supported_sample_rates: vec![44100, 48000, 96000, 192000],
            max_channels: 32,
        };

        Ok(Self {
            device_name,
            sample_rate,
            format,
            buffer_size,
            num_channels,
            filled_tx,
            filled_rx,
            empty_tx,
            empty_rx,
            is_streaming: Arc::new(AtomicBool::new(false)),
            capabilities,
            stream: None,
        })
    }

    fn start_cpal_stream(&mut self) -> Result<()> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device"))?;

        let config = StreamConfig {
            channels: self.num_channels as u16,
            sample_rate: cpal::SampleRate(self.sample_rate as u32),
            buffer_size: cpal::BufferSize::Fixed(self.buffer_size as u32),
        };

        let empty_rx = self.empty_rx.clone();
        let filled_tx = self.filled_tx.clone();
        let buffer_size = self.buffer_size;
        let num_channels = self.num_channels;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Try to get empty buffer
                if let Ok(mut buffer) = empty_rx.try_recv() {
                    // Copy audio data
                    if let SampleData::F32(ref mut samples) = buffer.data {
                        let copy_len = data.len().min(samples.len());
                        samples[..copy_len].copy_from_slice(&data[..copy_len]);
                        buffer.num_channels = num_channels;
                    }

                    // Send filled buffer
                    let _ = filled_tx.try_send(buffer);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);

        Ok(())
    }
}

#[async_trait]
impl Device for AudioDevice {
    async fn start(&mut self) -> Result<()> {
        self.start_cpal_stream()?;
        self.is_streaming.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.stream = None;  // Drops stream, stops playback
        self.is_streaming.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn get_channels(&mut self) -> DeviceChannels {
        DeviceChannels {
            filled_rx: self.filled_rx.clone(),
            empty_tx: self.empty_tx.clone(),
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        self.capabilities.clone()
    }

    fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }
}
```

**Step 3: Run streaming test**

```bash
cargo test test_audio_streaming_basic -- --nocapture
```

Expected: PASS (receives audio packet)

Note: If running headless, test may timeout. That's expected.

**Step 4: Commit**

```bash
git add src/hal/drivers/audio_device.rs tests/hal_audio_streaming_tests.rs
git commit -m "feat(hal): implement CPAL audio streaming

- Build input stream with CPAL
- Channel ping-pong for buffer reuse
- Test verifies packet reception"
```

---

### Task 6: Channel Mapping Logic

**Files:**
- Create: `src/hal/channel_mapper.rs`
- Modify: `src/hal/mod.rs`
- Create: `tests/hal_channel_mapper_tests.rs`

**Step 1: Write channel mapping tests**

Create `tests/hal_channel_mapper_tests.rs`:

```rust
use audiotab::hal::*;

#[test]
fn test_channel_mapping_direct() {
    let mapping = ChannelMapping {
        physical_channels: 3,
        virtual_channels: 3,
        routing: vec![
            ChannelRoute::Direct(0),
            ChannelRoute::Direct(1),
            ChannelRoute::Direct(2),
        ],
    };

    let physical = vec![1.0, 2.0, 3.0];  // Interleaved: [ch0, ch1, ch2]
    let virtual_mapped = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_mapped.len(), 3);
    assert_eq!(virtual_mapped, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_channel_mapping_reorder() {
    let mapping = ChannelMapping {
        physical_channels: 3,
        virtual_channels: 3,
        routing: vec![
            ChannelRoute::Direct(2),  // Virt[0] = Phys[2]
            ChannelRoute::Direct(1),  // Virt[1] = Phys[1]
            ChannelRoute::Direct(0),  // Virt[2] = Phys[0]
        ],
    };

    let physical = vec![1.0, 2.0, 3.0];
    let virtual_mapped = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_mapped, vec![3.0, 2.0, 1.0]);  // Reversed
}

#[test]
fn test_channel_mapping_merge() {
    let mapping = ChannelMapping {
        physical_channels: 3,
        virtual_channels: 1,
        routing: vec![
            ChannelRoute::Merge(vec![0, 1, 2]),  // Virt[0] = avg(Phys[0,1,2])
        ],
    };

    let physical = vec![1.0, 2.0, 3.0];
    let virtual_mapped = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_mapped.len(), 1);
    assert_eq!(virtual_mapped[0], 2.0);  // (1+2+3)/3 = 2.0
}

#[test]
fn test_channel_mapping_duplicate() {
    let mapping = ChannelMapping {
        physical_channels: 1,
        virtual_channels: 3,
        routing: vec![
            ChannelRoute::Direct(0),
            ChannelRoute::Duplicate(0),  // Virt[1] = Phys[0]
            ChannelRoute::Duplicate(0),  // Virt[2] = Phys[0]
        ],
    };

    let physical = vec![5.0];
    let virtual_mapped = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_mapped, vec![5.0, 5.0, 5.0]);
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test hal_channel_mapper
```

Expected: FAIL with "ChannelMapper not found"

**Step 3: Implement ChannelMapper**

Create `src/hal/channel_mapper.rs`:

```rust
use anyhow::Result;
use super::types::{ChannelMapping, ChannelRoute};

pub struct ChannelMapper;

impl ChannelMapper {
    /// Apply channel mapping to physical samples, producing virtual samples
    pub fn apply(mapping: &ChannelMapping, physical: &[f64]) -> Result<Vec<f64>> {
        if physical.len() != mapping.physical_channels {
            anyhow::bail!(
                "Expected {} physical channels, got {}",
                mapping.physical_channels,
                physical.len()
            );
        }

        let mut virtual_samples = Vec::with_capacity(mapping.virtual_channels);

        for route in &mapping.routing {
            let sample = match route {
                ChannelRoute::Direct(ch) => {
                    Self::validate_channel(*ch, physical.len())?;
                    physical[*ch]
                }
                ChannelRoute::Reorder(channels) => {
                    // Same as Direct for single channel
                    if channels.len() != 1 {
                        anyhow::bail!("Reorder expects single channel, got {}", channels.len());
                    }
                    Self::validate_channel(channels[0], physical.len())?;
                    physical[channels[0]]
                }
                ChannelRoute::Merge(channels) => {
                    // Average the channels
                    let sum: f64 = channels.iter()
                        .map(|&ch| {
                            Self::validate_channel(ch, physical.len()).unwrap();
                            physical[ch]
                        })
                        .sum();
                    sum / channels.len() as f64
                }
                ChannelRoute::Duplicate(ch) => {
                    Self::validate_channel(*ch, physical.len())?;
                    physical[*ch]
                }
            };

            virtual_samples.push(sample);
        }

        if virtual_samples.len() != mapping.virtual_channels {
            anyhow::bail!(
                "Mapping produced {} channels, expected {}",
                virtual_samples.len(),
                mapping.virtual_channels
            );
        }

        Ok(virtual_samples)
    }

    fn validate_channel(ch: usize, available: usize) -> Result<()> {
        if ch >= available {
            anyhow::bail!("Channel {} out of range (0..{})", ch, available);
        }
        Ok(())
    }

    /// Create default 1:1 mapping
    pub fn default_mapping(num_channels: usize) -> ChannelMapping {
        ChannelMapping {
            physical_channels: num_channels,
            virtual_channels: num_channels,
            routing: (0..num_channels).map(ChannelRoute::Direct).collect(),
        }
    }
}
```

**Step 4: Export ChannelMapper**

In `src/hal/mod.rs`:

```rust
pub mod channel_mapper;

pub use channel_mapper::ChannelMapper;
```

**Step 5: Run tests**

```bash
cargo test hal_channel_mapper
```

Expected: PASS (4 tests)

**Step 6: Commit**

```bash
git add src/hal/channel_mapper.rs src/hal/mod.rs tests/hal_channel_mapper_tests.rs
git commit -m "feat(hal): implement channel mapping logic

- ChannelMapper for routing physical to virtual channels
- Support direct, reorder, merge, duplicate routing
- Default 1:1 mapping helper"
```

---

## Phase 5.3: Hardware Manager UI (Frontend)

### Task 7: Hardware Manager State (Tauri Backend)

**Files:**
- Create: `src-tauri/src/hardware_manager/mod.rs`
- Create: `src-tauri/src/hardware_manager/state.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create hardware manager state module**

Create `src-tauri/src/hardware_manager/mod.rs`:

```rust
pub mod state;

pub use state::HardwareManagerState;
```

**Step 2: Implement hardware manager state**

Create `src-tauri/src/hardware_manager/state.rs`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use audiotab::hal::*;
use anyhow::Result;

pub struct HardwareManagerState {
    registry: Arc<RwLock<HardwareRegistry>>,
}

impl HardwareManagerState {
    pub fn new() -> Self {
        let mut registry = HardwareRegistry::new();

        // Register built-in drivers
        registry.register(AudioDriver::new());

        Self {
            registry: Arc::new(RwLock::new(registry)),
        }
    }

    pub async fn discover_devices(&self) -> Result<Vec<DeviceInfo>> {
        let registry = self.registry.read().await;
        registry.discover_all().await
    }

    pub async fn create_device(
        &self,
        driver_id: &str,
        device_id: &str,
        config: DeviceConfig,
    ) -> Result<()> {
        let registry = self.registry.read().await;
        let _device = registry.create_device(driver_id, device_id, config)?;
        // TODO: Store device in state
        Ok(())
    }
}

impl Default for HardwareManagerState {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Add to Tauri app state**

In `src-tauri/src/lib.rs`, add:

```rust
mod hardware_manager;

use hardware_manager::HardwareManagerState;

// In the builder, add state:
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .manage(HardwareManagerState::new())  // Add this
        // ... rest of builder
}
```

**Step 4: Commit**

```bash
git add src-tauri/src/hardware_manager/ src-tauri/src/lib.rs
git commit -m "feat(tauri): add hardware manager state

- HardwareManagerState with registry
- Initialize with AudioDriver
- Integrate with Tauri app state"
```

---

### Task 8: Hardware Manager Tauri Commands

**Files:**
- Create: `src-tauri/src/hardware_manager/commands.rs`
- Modify: `src-tauri/src/hardware_manager/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create Tauri commands**

Create `src-tauri/src/hardware_manager/commands.rs`:

```rust
use tauri::State;
use audiotab::hal::{DeviceInfo, DeviceConfig};
use super::state::HardwareManagerState;

#[tauri::command]
pub async fn discover_hardware(
    state: State<'_, HardwareManagerState>,
) -> Result<Vec<DeviceInfo>, String> {
    state.discover_devices()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_hardware_device(
    state: State<'_, HardwareManagerState>,
    driver_id: String,
    device_id: String,
    config: DeviceConfig,
) -> Result<(), String> {
    state.create_device(&driver_id, &device_id, config)
        .await
        .map_err(|e| e.to_string())
}
```

**Step 2: Export commands**

In `src-tauri/src/hardware_manager/mod.rs`:

```rust
pub mod commands;
pub mod state;

pub use commands::*;
pub use state::HardwareManagerState;
```

**Step 3: Register commands in Tauri**

In `src-tauri/src/lib.rs`, update the `invoke_handler`:

```rust
use hardware_manager::{discover_hardware, create_hardware_device};

// In run():
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    discover_hardware,
    create_hardware_device,
])
```

**Step 4: Build to verify**

```bash
cd src-tauri && cargo check
```

Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src-tauri/src/hardware_manager/ src-tauri/src/lib.rs
git commit -m "feat(tauri): add hardware manager commands

- discover_hardware command
- create_hardware_device command
- Register in invoke_handler"
```

---

### Task 9: Hardware Manager UI - Device List

**Files:**
- Create: `src-frontend/src/pages/HardwareManager.tsx`
- Create: `src-frontend/src/components/DeviceList.tsx`
- Modify: `src-frontend/src/App.tsx`

**Step 1: Create device list component**

Create `src-frontend/src/components/DeviceList.tsx`:

```typescript
import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface DeviceInfo {
  id: string;
  name: string;
  hardware_type: 'Acoustic' | 'Special';
  driver_id: string;
}

export function DeviceList() {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [loading, setLoading] = useState(true);

  const discoverDevices = async () => {
    setLoading(true);
    try {
      const discovered = await invoke<DeviceInfo[]>('discover_hardware');
      setDevices(discovered);
    } catch (err) {
      console.error('Failed to discover devices:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    discoverDevices();
  }, []);

  const acousticDevices = devices.filter(d => d.hardware_type === 'Acoustic');
  const specialDevices = devices.filter(d => d.hardware_type === 'Special');

  if (loading) {
    return <div>Discovering devices...</div>;
  }

  return (
    <div style={{ padding: '20px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
        <h2>Hardware Manager</h2>
        <button onClick={discoverDevices}>Refresh</button>
      </div>

      <section>
        <h3>🎤 Acoustic Hardware</h3>
        {acousticDevices.length === 0 ? (
          <p>No acoustic devices found</p>
        ) : (
          <ul>
            {acousticDevices.map(device => (
              <li key={device.id}>
                {device.name}
                <button style={{ marginLeft: '10px' }}>Configure</button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section style={{ marginTop: '30px' }}>
        <h3>⚙️ Special Hardware</h3>
        {specialDevices.length === 0 ? (
          <p>No special devices found</p>
        ) : (
          <ul>
            {specialDevices.map(device => (
              <li key={device.id}>
                {device.name}
                <button style={{ marginLeft: '10px' }}>Configure</button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
```

**Step 2: Create hardware manager page**

Create `src-frontend/src/pages/HardwareManager.tsx`:

```typescript
import React from 'react';
import { DeviceList } from '../components/DeviceList';

export function HardwareManager() {
  return (
    <div style={{ width: '100%', height: '100vh', overflow: 'auto' }}>
      <DeviceList />
    </div>
  );
}
```

**Step 3: Add route to App.tsx**

In `src-frontend/src/App.tsx`, add hardware manager toggle:

```typescript
import { HardwareManager } from './pages/HardwareManager';

// Add state:
const [showHardware, setShowHardware] = useState(false);

// Update toggle button section:
<div style={{ display: 'flex', gap: '10px' }}>
  <button onClick={() => setShowVizDemo(!showVizDemo)}>
    {showVizDemo ? 'Editor' : 'Viz Demo'}
  </button>
  <button onClick={() => setShowHardware(!showHardware)}>
    {showHardware ? 'Editor' : 'Hardware'}
  </button>
</div>

// Update conditional rendering:
{showVizDemo ? (
  <VisualizationDemo />
) : showHardware ? (
  <HardwareManager />
) : (
  // ... existing editor view
)}
```

**Step 4: Test in browser**

```bash
cargo tauri dev
```

Click "Hardware" button, should see device list with discovered audio devices.

**Step 5: Commit**

```bash
git add src-frontend/src/components/DeviceList.tsx src-frontend/src/pages/HardwareManager.tsx src-frontend/src/App.tsx
git commit -m "feat(frontend): add hardware manager UI

- DeviceList component with two-tier display
- Hardware manager page
- Toggle button in App"
```

---

## Summary

**Phase 5.1 Complete:**
- HAL traits and types
- Hardware registry
- Channel ping-pong infrastructure

**Phase 5.2 Complete:**
- CPAL audio driver
- Audio device with streaming
- Channel mapping logic

**Phase 5.3 Partial:**
- Tauri backend state and commands
- Basic device list UI

**Remaining Work:**
- Device configuration panel
- Channel mapping UI
- Flow graph integration
- Developer documentation

**Next Steps:**

Continue with Phase 5.3 (device configuration UI), then Phase 5.4 (flow graph integration), and Phase 5.5 (documentation).
