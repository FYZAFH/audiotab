# Device Registration & Streaming Design

**Date**: 2025-11-26
**Status**: Approved
**Phase**: Phase 6 - Device Registration & Real-Device Streaming

## Overview

This design enables registration of hardware devices to the backend and streaming audio from real devices for analysis and presentation. The system follows a clear separation: the kernel handles processing only, while a separate management layer handles device configuration and persistence.

## Goals

- Register hardware devices with user-defined configurations
- Persist device registrations across app sessions
- Start/stop kernel with all configured devices
- Stream audio from real input devices through the processing pipeline
- Display streaming results in visualization panels
- Support simultaneous input/output for measurement use cases

## Core Principles

1. **Kernel does processing, not configuration management**
2. **Real-time where possible, accurate where necessary**
3. **Configuration separate from execution**

---

## Architecture

### Three-Layer Architecture

```
┌─────────────────────────────────────────┐
│  Frontend (React + Tauri)               │
│  - Hardware Manager UI (menubar)        │
│  - Process Configuration (menubar)      │
│  - Home Page (control center)           │
│  - Visualization Panels                 │
│  - Local Storage (mirrors backend)      │
└──────────────┬──────────────────────────┘
               ↓ Tauri Commands
┌──────────────┴──────────────────────────┐
│  Backend Management (Rust + Tauri)      │
│  - HardwareConfigManager                │
│  - Config file persistence              │
│  - Kernel lifecycle management          │
└──────────────┬──────────────────────────┘
               ↓ Provides activated devices
┌──────────────┴──────────────────────────┐
│  Kernel/Engine (Pure Processing)        │
│  - HAL device streaming                 │
│  - Signal processing pipeline           │
│  - Does NOT manage configuration        │
└─────────────────────────────────────────┘
```

**Key Separation**:
- Frontend: User interaction and display
- Backend Management: Configuration persistence and kernel control
- Kernel: Stateless processing - receives "what to run" and runs it

---

## Data Model

### RegisteredHardware Structure

```typescript
interface RegisteredHardware {
  // Identity
  registration_id: string;        // Unique registration ID
  device_id: string;              // Hardware device ID (from discovery)
  hardware_name: string;          // System device name (read-only)
  driver_id: string;              // Which driver manages this device
  hardware_type: 'Acoustic' | 'Special';
  direction: 'Input' | 'Output';

  // User Configuration
  user_name: string;              // User-defined name (must be unique)
  enabled: boolean;               // Include in kernel when started

  // Audio Configuration
  protocol?: 'ASIO' | 'CoreAudio' | 'ALSA' | 'WASAPI' | 'Jack';
  sample_rate: number;
  channels: number;
  channel_mapping: ChannelMapping;
  calibration: Calibration;
  max_voltage: number;
  notes: string;
}
```

### Persistence Strategy

**Backend Config File**: `~/.audiotab/hardware_config.json`

```json
{
  "version": "1.0",
  "registered_devices": [
    { /* RegisteredHardware objects */ }
  ]
}
```

**Dual Storage Approach**:
- **Backend file** = Source of truth (persists across sessions)
- **Frontend localStorage** = Fast access for UI rendering (no backend calls needed)

**Sync Flow**:
1. Frontend modifies device → calls Tauri command `update_device(device)`
2. Backend updates config file + in-memory state
3. Frontend localStorage mirrors backend (for offline display)
4. On app startup: Frontend loads from backend via `get_registered_devices()`

---

## Kernel Lifecycle

### Start Kernel Flow

**User clicks "Start Kernel"** → Triggers the following sequence:

1. **Validate Configuration**
   - Check flowchart is valid (no disconnected nodes, cycles, etc.)
   - Ensure all nodes with `device_id` reference existing registered devices
   - Verify enabled devices are still available on system

2. **Load Configuration into Kernel**
   - Read flowchart from frontend
   - Get all enabled `RegisteredHardware` from backend config
   - Pass both to kernel via `kernel.initialize(flowchart, devices)`

3. **Activate Devices**
   - For each device referenced in flowchart nodes:
     - Create `DeviceConfig` from `RegisteredHardware`
     - Call `HardwareRegistry::create_device(device_id, config)`
     - Call `device.start()` → begins streaming
   - All device streams now flowing into ring buffers

4. **Initialize Pipeline**
   - Create all nodes from flowchart
   - Connect inputs/outputs based on flowchart edges
   - Allocate buffers for processing
   - Pipeline is **ready but not processing yet** (waits for trigger)

5. **Update UI State**
   - Home page shows "Kernel: Running"
   - Enable trigger buttons (Record, Play, etc.)

### Stop Kernel Flow

- Call `device.stop()` on all active devices
- Deallocate pipeline buffers
- Reset kernel state
- UI shows "Kernel: Stopped"

### Trigger Flow (e.g., "Start Record")

- Kernel is already running (devices streaming)
- Trigger activates processing nodes that were waiting
- Example: Recording node starts writing to file
- Stop trigger pauses processing but keeps kernel running

---

## Device-Node Integration

### Hardware-Requiring Nodes

**New Node Types**:
- `AudioInputNode`: Captures audio from a device
- `AudioOutputNode`: Plays audio to a device
- (Future: `DAQInputNode`, `SerialInputNode`, etc.)

### Node Configuration Flow

**When user adds AudioInputNode to flowchart**:

1. Node appears with default name "Audio Input"
2. Properties panel opens showing:
   ```
   Node: Audio Input
   ├─ Device: [Dropdown of registered input devices]
   ├─ Channels: [Auto-populated from device]
   └─ Sample Rate: [Auto-populated from device]
   ```
3. User selects device from dropdown (filtered to show only enabled inputs)
4. Node stores `device_id` in its configuration

### Data Flow: Device → Node → Processing

**During Kernel Execution**:

```
Hardware Device (CPAL stream)
    ↓ (fills buffer)
PacketBuffer in RingBuffer
    ↓ (kernel reads)
AudioInputNode.process()
    ↓ (outputs AudioData)
Connected Processing Nodes (FFT, Filter, etc.)
    ↓
AudioOutputNode.process()
    ↓ (writes to buffer)
PacketBuffer → Hardware Output Device
```

**Key Mechanism**:
- AudioInputNode has `device_id` property
- On kernel start, kernel maps `device_id` → `Device` instance
- Node's `process()` reads from device's ring buffer
- Node outputs standard `AudioData` for other nodes to consume

### Simultaneous Input/Output

**Use Case**: Speaker performance measurement

```
[Signal Generator Node]
    ↓
[AudioOutputNode: "Studio Speakers"]  → plays test signal
    ↓ (through air)
[AudioInputNode: "Measurement Mic"]   → captures response
    ↓
[Analysis Node: Transfer Function]
    ↓
[Display: Frequency Response]
```

**Capabilities**:
- ✅ Multiple AudioInputNodes and AudioOutputNodes can coexist in same flowchart
- ✅ Each references a different device (e.g., speakers + microphone)
- ✅ All devices start simultaneously when kernel starts
- ✅ Streams run concurrently

**Synchronization (Phase 1)**:
- Timestamp-based alignment (PacketBuffer has timestamps)
- Devices start at approximately the same time
- **Future**: Explicit sync groups for sample-accurate alignment
- **Future**: Post-processing alignment (NCC) for critical measurements

---

## Visualization System

### Hybrid Approach: Nodes + Panels

**Visualization Nodes in Flowchart**:
- `WaveformViewNode`: Displays time-domain audio
- `SpectrogramViewNode`: Displays frequency-time analysis
- (Future: `FrequencyResponseNode`, `WaterfallNode`, etc.)

**Node Configuration**:
```
Node: Waveform View
├─ Enabled: [✓] (draw or not)
├─ Window Size: 1024 samples
├─ Update Rate: 60 FPS
└─ [Open Display Panel] button
```

### Display Panels (Separate from Flowchart)

**Three Ways to Show Visualizations**:

1. **Pop-out from Flowchart**: Click node's "Open Display" → panel appears
2. **Home Page Widget**: Drag visualization from flowchart to home page dock
3. **Standalone Window**: Double-click visualization node → opens in new window

**Data Flow**:
```
Processing Pipeline → VisualizationNode.process()
    ↓ (via Tauri event)
Frontend Display Panel ← receives streaming data
    ↓
Canvas/WebGL rendering
```

**Why Hybrid?**
- **Flowchart shows data flow** (where visualization happens in pipeline)
- **Configuration in node properties** (window size, update rate, etc.)
- **Rendering in dedicated UI** (better performance and UX)
- **Flexible display** (pop-out panels, home page widgets, standalone windows)

---

## UI Restructuring

### Current State (Problematic)
- Home page is the flowchart editor (process configuration)
- Hardware manager is separate page
- No clear control center

### New Structure

#### Home Page (Control Center)

```
┌─────────────────────────────────────┐
│ AudioTab                    [Menu]  │
├─────────────────────────────────────┤
│                                     │
│  Kernel Status: ● Stopped           │
│                                     │
│  ┌──────────┐  ┌──────────┐       │
│  │  Start   │  │   Stop   │       │
│  │  Kernel  │  │  Kernel  │       │
│  └──────────┘  └──────────┘       │
│                                     │
│  ┌──────────┐  ┌──────────┐       │
│  │  Start   │  │ Playback │       │
│  │  Record  │  │          │       │
│  └──────────┘  └──────────┘       │
│                                     │
│  Visualization Panels:              │
│  ┌─────────────────────────────┐  │
│  │  [Waveform] [Spectrogram]   │  │
│  │  (docked panels from flow)  │  │
│  └─────────────────────────────┘  │
└─────────────────────────────────────┘
```

#### Menubar Features

```
File | Edit | View | Configure | Help
              ↓
         Configure menu:
         ├─ Hardware Manager
         ├─ Process Configuration (Flowchart)
         └─ Preferences
```

**Key Changes**:
- **Home = Control panel** (start/stop, triggers, status display)
- **Hardware Manager = Menubar feature** (not home page)
- **Process Configuration (flowchart) = Menubar feature**
- **Visualization panels can dock to home page**

---

## Workflow

### User Workflow (End-to-End)

1. **Configure Devices** (Hardware Manager - menubar)
   - Discover available devices
   - Register devices with user-defined names
   - Configure sample rate, channels, protocol
   - Enable devices for use

2. **Configure Processing** (Process Configuration - menubar)
   - Open flowchart editor
   - Add AudioInputNode, select registered microphone
   - Add processing nodes (FFT, filters, etc.)
   - Add AudioOutputNode, select registered speakers
   - Add WaveformViewNode for visualization
   - Save flowchart

3. **Run Session** (Home page)
   - Click "Start Kernel" → all enabled devices start streaming
   - Kernel status shows "Running"
   - Click "Start Record" → processing begins
   - View real-time waveform in docked panel
   - Click "Stop Record" → processing pauses
   - Click "Stop Kernel" → shutdown

---

## API Design

### Tauri Commands

```rust
// Device Management
#[tauri::command]
async fn get_registered_devices() -> Result<Vec<RegisteredHardware>, String>;

#[tauri::command]
async fn register_device(device: RegisteredHardware) -> Result<(), String>;

#[tauri::command]
async fn update_device(registration_id: String, updates: PartialDevice) -> Result<(), String>;

#[tauri::command]
async fn remove_device(registration_id: String) -> Result<(), String>;

// Kernel Lifecycle
#[tauri::command]
async fn start_kernel(flowchart: FlowchartConfig) -> Result<(), String>;

#[tauri::command]
async fn stop_kernel() -> Result<(), String>;

#[tauri::command]
async fn get_kernel_status() -> Result<KernelStatus, String>;

// Triggers
#[tauri::command]
async fn trigger_record(start: bool) -> Result<(), String>;

#[tauri::command]
async fn trigger_playback(start: bool) -> Result<(), String>;
```

### Backend State Structure

```rust
pub struct HardwareConfigManager {
    config_path: PathBuf,
    registered_devices: Arc<RwLock<Vec<RegisteredHardware>>>,
}

pub struct KernelManager {
    status: Arc<RwLock<KernelStatus>>,
    active_devices: HashMap<String, Box<dyn Device>>,
    pipeline: Option<AsyncPipeline>,
}

pub enum KernelStatus {
    Stopped,
    Running { device_count: usize },
    Error { message: String },
}
```

---

## Implementation Phases

### Phase 1: Backend Management Layer
- [ ] Create `HardwareConfigManager` with config file persistence
- [ ] Implement Tauri commands for device CRUD operations
- [ ] Add config file loading/saving (JSON format)
- [ ] Sync frontend localStorage with backend on startup

### Phase 2: Kernel Lifecycle
- [ ] Create `KernelManager` for kernel state management
- [ ] Implement `start_kernel()` with device activation
- [ ] Implement `stop_kernel()` with cleanup
- [ ] Add kernel status tracking and events

### Phase 3: Device Nodes
- [ ] Create `AudioInputNode` implementation
- [ ] Create `AudioOutputNode` implementation
- [ ] Add device selection dropdown in node properties
- [ ] Wire device streams to node processing

### Phase 4: UI Restructuring
- [ ] Create new Home page with control buttons
- [ ] Move flowchart editor to menubar (Process Configuration)
- [ ] Add kernel status display to Home page
- [ ] Implement visualization panel docking

### Phase 5: Streaming & Visualization
- [ ] Connect device streams to visualization nodes
- [ ] Implement Tauri events for streaming data
- [ ] Add real-time waveform rendering
- [ ] Add real-time spectrogram rendering

---

## Future Enhancements

### Sync Groups (Later Phase)
- Explicit sync groups for sample-accurate alignment
- Shared clock for multiple devices
- Drift correction for long recordings

### Post-Processing Alignment
- NCC (Normalized Cross-Correlation) for offline alignment
- Time-shift correction tools
- Latency compensation

### Additional Hardware Support
- DAQ input nodes (National Instruments, etc.)
- Serial device nodes
- Network stream nodes (TCP/UDP)
- Modbus RTU nodes

---

## Success Criteria

1. ✅ User can register hardware devices with custom names
2. ✅ Devices persist across app restarts
3. ✅ Kernel starts all enabled devices simultaneously
4. ✅ Real audio streams from input devices to visualization
5. ✅ Simultaneous input/output works for measurement use cases
6. ✅ Home page provides clear control center
7. ✅ Visualization panels can be popped out/docked

---

## Non-Goals (Out of Scope for Phase 1)

- Sample-accurate synchronization (timestamp-based is sufficient)
- Hard real-time guarantees (best-effort is acceptable)
- AI/ML real-time processing (offline processing is acceptable)
- Multi-computer distributed processing
- Cloud storage/sync

