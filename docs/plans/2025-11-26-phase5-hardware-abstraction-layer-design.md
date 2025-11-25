# Phase 5: Hardware Abstraction Layer (HAL) Design

**Date**: 2025-11-26
**Status**: Approved for Implementation
**Previous Phase**: Phase 4 - Streaming & Visualization

## Overview

Phase 5 introduces a Hardware Abstraction Layer that enables the streaming analysis platform to work with diverse hardware sources (audio, vibration, TCP streams, etc.) through a unified interface. The system provides first-class support for acoustic hardware while allowing developers to easily add custom hardware types.

## Design Goals

1. **Well-defined hardware architecture** supporting multiple streaming hardware types
2. **Efficient streaming** with zero-copy and buffer reuse where possible
3. **Developer convenience** - simple trait to implement, clear conversion model
4. **Complexity isolation** - hardware management separate from flow graph
5. **Real-time discovery** for audio hardware with manual refresh fallback

## Key Design Tension

**Abstraction cost vs framework simplicity**: Abstracting all hardware to streaming packets has performance overhead, but it simplifies the analysis framework by isolating hardware complexity to the HAL module.

**Solution**: Use native hardware formats (i16, f32, etc.) and channel ping-pong for efficient zero-copy streaming while maintaining clean abstractions.

## Architecture

### Three-Layer Structure

```
┌─────────────────────────────────────────────────────────────┐
│              Hardware Driver Trait Layer                    │
│  (Trait: HardwareDriver - compile-time plugin system)       │
├─────────────────────┬───────────────────────────────────────┤
│  Built-in Drivers   │   Developer-Added Drivers             │
│  • AudioDriver      │   • VibrationSensorDriver             │
│    (CPAL-based)     │   • TCPStreamDriver                   │
│                     │   • SerialProtocolDriver              │
├─────────────────────┴───────────────────────────────────────┤
│           Hardware Registry & Management                    │
│  • Device discovery (hot-plug + manual refresh)             │
│  • Configuration storage (name, channel map, calibration)   │
│  • Two-tier classification (Acoustic / Special)             │
├─────────────────────────────────────────────────────────────┤
│           Virtual Hardware (Unified Abstraction)            │
│  • Timestamped packet streams                               │
│  • Channel-routed data                                      │
│  • Calibration applied                                      │
└─────────────────────────────────────────────────────────────┘
```

## Hardware Classification

### Acoustic Hardware (First-Class Support)

**Definition**: Hardware that produces time-series sample streams at a known sample rate.

**Includes**:
- Native audio devices (microphones, audio interfaces) via CPAL
- Developer-converted devices (vibration sensors, accelerometers) that output sample streams

**Features**:
- Real-time hot-plug discovery
- Channel mapping (reorder, merge, duplicate)
- Simple calibration (gain/offset)
- Automatic integration with visualization
- Flow graph integration

### Special Hardware (Limited Support)

**Definition**: Hardware with custom protocols or unique data formats.

**Includes**:
- TCP streams with custom protocols
- Serial devices with proprietary formats
- Hardware that doesn't fit sample-stream model

**Features**:
- Basic device listing
- Developer-defined usage patterns
- No automatic framework integration

## Core Trait Interfaces

### HardwareDriver Trait

```rust
use async_trait::async_trait;

#[async_trait]
pub trait HardwareDriver: Send + Sync {
    /// Unique driver identifier (e.g., "cpal-audio", "tcp-stream")
    fn driver_id(&self) -> &str;

    /// Hardware classification
    fn hardware_type(&self) -> HardwareType {
        HardwareType::Special  // Default to special
    }

    /// Discover available devices (async for network discovery)
    async fn discover_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Create a device instance with configuration
    fn create_device(
        &self,
        device_id: &str,
        config: DeviceConfig
    ) -> Result<Box<dyn Device>>;
}

pub enum HardwareType {
    Acoustic,  // Full framework support - time-series samples
    Special,   // Developer-defined usage
}
```

### Device Trait

```rust
#[async_trait]
pub trait Device: Send {
    /// Start streaming data
    async fn start(&mut self) -> Result<()>;

    /// Stop streaming
    async fn stop(&mut self) -> Result<()>;

    /// Get channels for buffer ping-pong
    fn get_channels(&mut self) -> DeviceChannels;

    /// Device capabilities and info
    fn capabilities(&self) -> DeviceCapabilities;
}

pub struct DeviceChannels {
    /// Receive filled buffers from hardware
    pub filled_rx: Receiver<PacketBuffer>,

    /// Send empty buffers back to hardware for reuse
    pub empty_tx: Sender<PacketBuffer>,
}

// For output devices:
pub struct OutputChannels {
    /// Send filled buffers to hardware
    pub filled_tx: Sender<PacketBuffer>,

    /// Receive empty buffers from hardware
    pub empty_rx: Receiver<PacketBuffer>,
}
```

## Data Types

### Sample Formats

Hardware produces data in native formats. No forced conversion to f64.

```rust
pub enum SampleFormat {
    I16,    // 16-bit PCM (most common mic/audio)
    I24,    // 24-bit (pro audio)
    I32,    // 32-bit integer
    F32,    // 32-bit float
    F64,    // 64-bit float (analysis/processing)
    U8,     // 8-bit unsigned
}
```

### Packet Buffer

```rust
pub struct PacketBuffer {
    pub data: SampleData,
    pub sample_rate: u64,
    pub num_channels: usize,
    pub timestamp: Option<u64>,  // Nanoseconds, optional
}

pub enum SampleData {
    I16(Vec<i16>),
    I24(Vec<u8>),    // 3 bytes per sample
    I32(Vec<i32>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    U8(Vec<u8>),
    Bytes(Vec<u8>),  // For special hardware
}
```

### Device Configuration

```rust
pub struct DeviceConfig {
    pub name: String,              // User-assigned name
    pub sample_rate: u64,
    pub format: SampleFormat,
    pub buffer_size: usize,        // Samples per buffer
    pub channel_mapping: ChannelMapping,
    pub calibration: Calibration,
}

pub struct ChannelMapping {
    pub physical_channels: usize,
    pub virtual_channels: usize,
    pub routing: Vec<ChannelRoute>,  // Default: 1:1 sequential
}

pub enum ChannelRoute {
    Direct(usize),           // Phys[i] -> Virt[i]
    Reorder(Vec<usize>),     // Phys[1,2,3] -> Virt[3,2,1]
    Merge(Vec<usize>),       // Phys[1,2,3] -> Virt[1] (sum/avg)
    Duplicate(usize),        // Phys[1] -> Virt[1,2,3]
}

pub struct Calibration {
    pub gain: f64,      // Multiply (* float) for voltage
    pub offset: f64,    // Add (+ float) for SPL
}
```

## Channel Ping-Pong Pattern

Efficient buffer reuse through channel-based producer-consumer pattern.

### Workflow

```
Hardware Thread          Channel           Application Thread
───────────────         ────────          ──────────────────

Fill buffer A    ───────────►             Receive buffer A
                 filled_rx                Process data
Fill buffer B
                                          Send buffer A back
                 ◄───────────
                 empty_tx                 Receive buffer B
Fill buffer A    ───────────►             Process data

                                          Send buffer B back
                 ◄───────────

(Cycle continues with 2+ buffers in rotation)
```

### Implementation Example

```rust
impl MyAudioDevice {
    fn new(config: DeviceConfig) -> Self {
        // Create bounded channels (depth 2 for double-buffering)
        let (filled_tx, filled_rx) = channel::bounded(2);
        let (empty_tx, empty_rx) = channel::bounded(2);

        // Pre-allocate ping-pong buffers
        for _ in 0..2 {
            let buffer = PacketBuffer::new(
                config.format,
                config.buffer_size
            );
            empty_tx.send(buffer).unwrap();
        }

        // Spawn hardware thread
        let hw_thread = spawn(move || {
            loop {
                // Get empty buffer from pool
                let mut buffer = empty_rx.recv().unwrap();

                // Fill with hardware data
                hardware_read(&mut buffer);

                // Send filled buffer to application
                filled_tx.send(buffer).unwrap();
            }
        });

        Self {
            filled_rx,
            empty_tx,
            hw_thread,
            /* ... */
        }
    }
}
```

## Device Discovery

### Hot-Plug Events (Primary)

Listen to OS events for automatic discovery:
- **macOS**: IOKit notifications for USB/Thunderbolt devices
- **Windows**: Device notification messages (WM_DEVICECHANGE)
- **Linux**: udev monitoring

CPAL library handles this for audio devices automatically.

### Manual Refresh (Fallback)

Hardware manager provides "Refresh" button:
- Triggers `discover_devices()` on all registered drivers
- Useful for custom discovery mechanisms
- Gives users confidence/control

Developers can override refresh behavior for custom protocols.

## Hardware Manager UI

### Two-Tier Device List

```
┌─────────────────────────────────────────────────────────────┐
│              🎤 Acoustic Hardware                           │
├─────────────────────────────────────────────────────────────┤
│  ☑ MacBook Pro Microphone                   [Configure]    │
│  ☑ Focusrite Scarlett 2i2                   [Configure]    │
│  ☐ USB Accelerometer (Ch 1-3 → Acoustic)    [Configure]    │
├─────────────────────────────────────────────────────────────┤
│              ⚙️  Special Hardware                           │
├─────────────────────────────────────────────────────────────┤
│  ☐ TCP Sensor (192.168.1.50:8080)          [Configure]    │
│  ☐ Serial Device (COM3)                     [Configure]    │
└─────────────────────────────────────────────────────────────┘
```

### Device Configuration Panel

Clicking [Configure] opens device-specific settings:

**For Acoustic Hardware**:
- **Name**: "Lab Microphone 1"
- **Channels**: Physical (8) → Virtual (6)
- **Channel Mapping**:
  - Visual matrix or list showing routing
  - Examples: [1→1, 2→2, 3,4,5→3 (merge), 6→4,5,6 (duplicate)]
- **Calibration**:
  - Gain: 1.0× (voltage scaling)
  - Offset: +0.0 dB (SPL adjustment)
- **Sample Rate**: 48000 Hz (if configurable)
- **Buffer Size**: 1024 samples

**For Special Hardware**:
- Developer-defined configuration UI
- Framework provides basic name/enable fields

## Flow Graph Integration

Hardware manager is **separate** from flow graph. Flow graph only references devices by ID.

### Example Flow

```
[Input Node: "Lab Mic 1"] ──► [Filter] ──► [Gain] ──► [Output: "Speakers"]
       ↓                                                        ↓
  (References                                            (References
   device ID)                                             device ID)
```

User selects hardware from dropdown:
- Input nodes: Choose from registered input devices
- Output nodes: Choose from registered output devices

**No hardware configuration in flow graph** - that complexity lives in hardware manager.

## Developer Extension Workflow

### Adding New Hardware Type

1. **Implement HardwareDriver trait**:

```rust
pub struct MyVibrationDriver;

#[async_trait]
impl HardwareDriver for MyVibrationDriver {
    fn driver_id(&self) -> &str { "vibration-sensor" }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::Acoustic  // Outputs time-series samples
    }

    async fn discover_devices(&self) -> Result<Vec<DeviceInfo>> {
        // Scan USB or network for sensors
    }

    fn create_device(&self, id: &str, config: DeviceConfig)
        -> Result<Box<dyn Device>> {
        Ok(Box::new(MyVibrationDevice::new(id, config)?))
    }
}
```

2. **Implement Device trait**:

```rust
pub struct MyVibrationDevice {
    filled_tx: Sender<PacketBuffer>,
    empty_rx: Receiver<PacketBuffer>,
    // ... hardware state
}

#[async_trait]
impl Device for MyVibrationDevice {
    async fn start(&mut self) -> Result<()> {
        // Start reading from sensor
        // Fill buffers from empty_rx
        // Send to filled_tx
    }

    async fn stop(&mut self) -> Result<()> { /* ... */ }

    fn get_channels(&mut self) -> DeviceChannels {
        DeviceChannels {
            filled_rx: self.filled_rx.clone(),
            empty_tx: self.empty_tx.clone(),
        }
    }

    fn capabilities(&self) -> DeviceCapabilities { /* ... */ }
}
```

3. **Register driver in `src/hal/mod.rs`**:

```rust
pub fn init_hardware_registry() -> HardwareRegistry {
    let mut registry = HardwareRegistry::new();

    // Built-in drivers
    registry.register(AudioDriver::new());

    // Developer-added drivers
    registry.register(MyVibrationDriver::new());

    registry
}
```

4. **Done!** Hardware appears in hardware manager, gets full acoustic hardware support.

## Module Structure

```
src/
├── hal/
│   ├── mod.rs                  # Registry, public API
│   ├── traits.rs               # HardwareDriver, Device traits
│   ├── types.rs                # PacketBuffer, SampleFormat, etc.
│   ├── discovery.rs            # Hot-plug monitoring
│   ├── drivers/
│   │   ├── mod.rs
│   │   ├── audio.rs            # CPAL-based audio driver
│   │   └── tcp.rs              # Example TCP stream driver
│   └── channels.rs             # Buffer ping-pong helpers
│
├── hardware_manager/           # UI for device management
│   ├── mod.rs
│   ├── device_list.tsx         # Two-tier device list
│   ├── config_panel.tsx        # Device configuration UI
│   └── channel_mapper.tsx      # Visual channel routing
```

## Data Flow

### From Hardware to Visualization

```
Hardware                HAL                  Application
────────               ─────                ─────────────

Audio Interface  ──►  AudioDriver     ──►  HardwareRegistry
                      .read_packet()            ↓
                           │              ConfiguredDevice
                           │                    │
                      PacketBuffer              │
                      (i16 samples)        Apply channel
                           │              mapping, calibration
                           │                    │
                      Channel ping-pong    PacketBuffer
                      filled_rx ──────►   (virtual channels)
                           │                    │
                      empty_tx ◄──────    Process/visualize
```

### Timestamp Derivation

```rust
impl PacketBuffer {
    pub fn derive_timestamp(&self, packet_index: u64) -> u64 {
        // If hardware provides timestamp, use it
        if let Some(ts) = self.timestamp {
            return ts;
        }

        // Otherwise derive from sample count
        let samples_elapsed = packet_index * self.data.len();
        let time_ns = (samples_elapsed * 1_000_000_000) / self.sample_rate;
        time_ns
    }
}
```

## Implementation Phases

### Phase 5.1: Core HAL Infrastructure
- Define traits (HardwareDriver, Device)
- Implement channel ping-pong helpers
- Create hardware registry
- Basic device configuration storage

### Phase 5.2: Audio Driver (CPAL)
- Implement AudioDriver with CPAL
- Hot-plug discovery for audio devices
- Channel mapping logic
- Simple calibration

### Phase 5.3: Hardware Manager UI
- Device list (two-tier: acoustic/special)
- Configuration panel
- Channel mapping UI
- Integration with Tauri backend

### Phase 5.4: Flow Graph Integration
- Input/Output nodes reference devices by ID
- Data flow from hardware to visualization
- Update existing AudioSourceNode to use HAL

### Phase 5.5: Developer Documentation
- Guide for adding custom hardware
- Example TCP stream driver
- API documentation

## Success Criteria

1. ✅ Audio devices discovered automatically (hot-plug)
2. ✅ User can configure device name, channels, calibration
3. ✅ Channel mapping works (reorder, merge, duplicate)
4. ✅ Data flows from hardware → visualization with <10ms latency
5. ✅ Developer can add new hardware type in <100 lines of code
6. ✅ Zero-copy streaming where possible
7. ✅ No audio dropouts/glitches during streaming

## Performance Targets

- **Latency**: <10ms from hardware capture to visualization
- **Throughput**: 8 channels @ 192kHz with <1% CPU overhead
- **Memory**: <50MB for 30s ring buffer per device
- **Discovery**: <500ms to detect new USB device

## Next Steps

After Phase 5 completion:
- **Phase 6**: Advanced visualization (zoom, pan, markers)
- **Phase 7**: Real-time DSP nodes (filters, dynamics, EQ)
- **Phase 8**: Multi-device sync and routing
