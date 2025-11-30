# StreamLab Architectural Framework
## A Conceptual Design for Multi-Physics Streaming Analysis

**Document Purpose:** This framework defines WHAT software modules exist and WHAT each module should be responsible for, without prescribing HOW they are implemented. This provides architectural freedom while maintaining structural clarity.

---

## Executive Summary

StreamLab is a **three-pillar architecture** designed for streaming multi-physics analysis:

1. **Visual Orchestration** - Users compose analysis workflows like drawing flowcharts
2. **Unified Processing Kernel** - A streaming engine that executes workflows with real-time performance
3. **Hardware Virtualization** - Any physical device appears as a standardized virtual device

**Target Audiences:**
- **End Users:** Scientists, engineers, technicians who need to analyze data without programming
- **Algorithm Developers:** DSP experts who want to contribute signal processing modules
- **Hardware Developers:** Engineers integrating custom acquisition hardware
- **System Integrators:** Teams deploying this on production lines or in labs

---

## Part I: Conceptual Architecture

### 1. The Four-Layer Model

```
┌─────────────────────────────────────────────────────────────┐
│  PRESENTATION LAYER                                         │
│  • User Interface                                           │
│  • Real-time Visualization                                  │
│  • Configuration Editors                                    │
└─────────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────────┐
│  APPLICATION LAYER                                          │
│  • Graph Definition & Validation                            │
│  • Workflow Orchestration                                   │
│  • State Management                                         │
└─────────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────────┐
│  DOMAIN LAYER (The Kernel)                                  │
│  • Processing Node Registry                                 │
│  • Pipeline Execution Engine                                │
│  • Data Transformation                                      │
└─────────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────────┐
│  INFRASTRUCTURE LAYER                                       │
│  • Hardware Abstraction (HAL)                               │
│  • Data Transport                                           │
│  • Storage & Persistence                                    │
└─────────────────────────────────────────────────────────────┘
```

**Layer Responsibilities:**

- **Presentation:** What users see and interact with
- **Application:** Business rules, workflow logic, user intent
- **Domain:** Core computation, algorithm execution
- **Infrastructure:** Hardware, file systems, network, OS

**Key Principle:** Each layer only knows about the layer directly below it. The Domain layer does NOT know about the UI. This enables:
- Testing the kernel without a GUI
- Multiple UIs for the same kernel (desktop, web, CLI)
- Hardware changes without affecting algorithms

---

### 2. The Central Abstraction: The Processing Graph

**What it IS:**
A **Processing Graph** is a directed acyclic graph (DAG) where:
- **Nodes** represent operations (data source, transformation, sink)
- **Edges** represent data flow between operations
- **Parameters** configure each node's behavior

**What it is NOT:**
- Not a specific file format (could be JSON, YAML, binary, or database)
- Not tied to a particular programming language
- Not limited to audio (could be vibration, temperature, voltage, etc.)

**Critical Design Decision:**
The graph is a **first-class citizen** - it can be:
- Created visually or programmatically
- Serialized and versioned
- Validated before execution
- Executed multiple times concurrently
- Shared between users
- Embedded in larger systems

**Graph States:**
```
[Definition] → [Validation] → [Instantiation] → [Execution] → [Completion]
                     ↓              ↓                ↓              ↓
                   Invalid       Failed           Error       Terminated
```

---

### 3. The Processing Node: Universal Building Block

**Conceptual Interface:**
Every processing node, regardless of what it does, must answer these questions:

1. **What am I?**
   - Identity: Unique type identifier (e.g., "FFT", "HighPassFilter")
   - Category: Organizational grouping (Source, Filter, Analysis, Sink)
   - Metadata: Human-readable name, description, author, version

2. **What do I need?**
   - Input Ports: Named channels expecting specific data types
   - Output Ports: Named channels producing specific data types
   - Parameters: Configuration values with types, ranges, defaults

3. **What do I do?**
   - Processing Logic: Transform input data → output data
   - State Management: Internal state between invocations (if any)
   - Resource Requirements: Memory, CPU, special hardware

4. **How do I behave?**
   - Lifecycle Hooks: Setup, teardown, pause, resume
   - Error Handling: What errors can I produce? How do I recover?
   - Concurrency Model: Can I be called in parallel? Do I need locks?

**Node Taxonomy:**

**Source Nodes** (Data Producers)
- No inputs, one or more outputs
- Examples: Microphone, file reader, function generator, trigger listener
- Special behavior: May run continuously (streaming) or once (batch)

**Transform Nodes** (Data Processors)
- One or more inputs, one or more outputs
- Examples: Filters, FFT, gain, resample, correlation
- Special behavior: Must be stateless OR explicitly manage state

**Sink Nodes** (Data Consumers)
- One or more inputs, no outputs
- Examples: File writer, speaker output, network sender, display
- Special behavior: May have side effects (I/O, hardware control)

**Control Nodes** (Logic Flow)
- Route data based on conditions
- Examples: If/else, switch, loop, delay, merge, split
- Special behavior: Can change graph topology dynamically

**Hybrid Nodes** (Special Cases)
- Combine multiple roles
- Examples: Audio loopback (source + sink), echo canceller (needs reference input)

---

### 4. Data Flow: The DataFrame Concept

**What flows through the graph?**

Not raw bytes, not typed arrays, but **DataFrames** - a structured container with:

**Essential Components:**
1. **Timestamp** - When was this data captured/created?
2. **Sequence ID** - What is the order of this frame in the stream?
3. **Payload** - The actual data (multi-channel, multi-modal)
4. **Metadata** - Side-channel information (sample rate, units, calibration, etc.)

**Why not just arrays?**
- Arrays don't carry semantic information
- Multi-channel data needs channel names
- Mixed data types (audio + vibration + temperature) in one frame
- Metadata must travel with data (sample rate changes, gain settings)

**DataFrame Properties:**
- **Immutable Preferred:** Once created, should not be modified (create new frames instead)
- **Reference Countable:** Multiple nodes can read the same frame without copying
- **Type Flexible:** Payload can contain different data types per channel
- **Extensible:** New metadata fields can be added without breaking old nodes

**Data Flow Patterns:**

**Stream Processing (Real-time):**
```
Source → Frame 1 → Transform → Frame 1' → Sink
      → Frame 2 → Transform → Frame 2' → Sink
      → Frame 3 → Transform → Frame 3' → Sink
```

**Batch Processing (Offline):**
```
Source → All Frames → Buffer → Process All → Results → Sink
```

**Triggered Processing:**
```
Trigger Signal → Source → Frames → Process → Sink → Wait for Next Trigger
```

---

## Part II: Module Specifications

### Module 1: Visual Flowchart Builder (Presentation Layer)

**Purpose:** Enable non-programmers to construct complex analysis workflows

**Core Capabilities:**
1. **Infinite Canvas Workspace**
   - Pan, zoom, navigate large workflows
   - Organize nodes spatially for clarity
   - Group related nodes (visual organization, not execution)

2. **Node Palette**
   - Searchable library of all available node types
   - Categorized by function (Sources, Filters, Analysis, etc.)
   - Drag-and-drop onto canvas

3. **Connection System**
   - Connect output ports to input ports with visual edges
   - Type checking: Only compatible ports can connect
   - Visual feedback for valid/invalid connections
   - Support for branching (one output → multiple inputs)
   - Support for merging (multiple outputs → one node with multiple inputs)

4. **Parameter Editor**
   - Inline editing of node parameters
   - Context panel for detailed configuration
   - Real-time validation (show errors before execution)
   - Support for different input types: numbers, enums, files, colors, text

5. **Workflow Validation**
   - Static analysis before execution
   - Check for: disconnected nodes, circular dependencies, type mismatches
   - Visual highlighting of errors

6. **Templates & Presets**
   - Save common workflows as templates
   - Share templates between users
   - Import/export as portable files

**User Experience Goals:**
- Approachable: Someone with no programming experience can build a basic workflow in 5 minutes
- Powerful: Expert users can build complex multi-stage processing pipelines
- Discoverable: Users can explore capabilities through the node palette

**What This Module Does NOT Do:**
- Does not execute graphs (delegates to the kernel)
- Does not manage hardware (delegates to hardware manager)
- Does not store data (delegates to storage layer)

---

### Module 2: Hardware Abstraction Layer (HAL)

**Purpose:** Make any physical device appear as a standardized virtual device

**The Core Problem:**
- Audio interfaces use ASIO/CoreAudio/ALSA with different APIs
- DAQ cards use vendor-specific SDKs (NI-DAQmx, Measurement Computing)
- Custom hardware may use Serial, USB HID, network protocols
- Each device has different: channel counts, sample rates, data formats, triggering

**The Solution:**
A two-tier abstraction:

**Tier 1: Device Categories**
Define abstract categories of devices:

1. **Streaming Sources** (Audio, Vibration, Voltage)
   - Continuous data acquisition at fixed sample rate
   - Multi-channel support
   - Hardware-timed sampling
   - Example: Microphones, accelerometers

2. **Function Generators** (Signal output)
   - Generate waveforms or patterns
   - Synchronized multi-channel output
   - Example: Audio playback, vibration shakers

3. **Trigger Inputs** (Event detection)
   - Digital/analog threshold detection
   - External sync signals
   - Example: GPIO, encoder pulses

4. **Control Outputs** (Actuators)
   - Binary outputs (relay, LED)
   - Analog control (voltage/current)
   - Example: Pass/fail indicators, valve control

**Tier 2: Virtual Device Interface**
Each category has a standard interface:

**Streaming Source Interface:**
- `start()` - Begin acquisition
- `stop()` - End acquisition
- `configure(sample_rate, channels, format)` - Set parameters
- `get_data()` - Retrieve next buffer of samples
- `get_capabilities()` - Query supported sample rates, channels, formats

**Why This Matters:**
A "Microphone Input" node in the graph doesn't know if it's reading from:
- A USB microphone
- A professional audio interface
- A simulated sine wave generator (for testing)
- A network stream from another machine

**Hardware Driver Responsibilities:**
1. **Discovery:** Enumerate available devices on the system
2. **Configuration:** Translate virtual parameters to hardware-specific commands
3. **Streaming:** Deliver data in standardized DataFrame format
4. **Timing:** Provide accurate timestamps for all data
5. **Error Handling:** Detect and report hardware issues (disconnection, buffer overruns)

**Device Registration:**
- Plug-in architecture: Drivers register themselves at startup
- Metadata: Each driver declares what devices it supports
- Priority: Multiple drivers for same device? User chooses.

**Calibration & Mapping:**
- **Channel Mapping:** Physical channel 3 → Virtual channel "Front Left"
- **Gain/Offset:** Raw ADC counts → Engineering units (Pascals, g's, Volts)
- **Calibration Workflows:** Semi-automated calibration processes
  - Example: Play reference tone → measure response → calculate gain

---

### Module 3: Processing Node Registry & Plugin System

**Purpose:** Make it trivial for developers to add new analysis capabilities

**The Developer's Perspective:**

**Option A: Rust Native Node**
Developer writes a Rust module that implements the Node interface:
- Define metadata (name, category, description)
- Define inputs/outputs with types
- Define parameters with validation rules
- Implement processing function

**Benefit:** Maximum performance, type safety, compiled into the binary

**Option B: Python Script Node**
Developer writes a Python function:
- Receives input data as NumPy arrays
- Returns output data as NumPy arrays
- No compilation needed, can be edited and reloaded

**Benefit:** Rapid prototyping, access to SciPy/PyTorch/TensorFlow

**Option C: External Process**
Developer writes a standalone program:
- Communicates via stdin/stdout or network
- Any language (Julia, MATLAB, C++)

**Benefit:** Integrate existing tools without rewriting

**Registry Responsibilities:**
1. **Discovery:** Scan for available nodes at startup
   - Built-in nodes (compiled into binary)
   - Python scripts in designated folders
   - External executables with manifest files

2. **Metadata Extraction:**
   - Parse each node's declaration
   - Extract: name, category, inputs, outputs, parameters, version

3. **Type System:**
   - Maintain catalog of data types (Float64Array, Complex128, Spectrogram, etc.)
   - Validate that connected nodes have compatible types
   - Allow developers to define custom types

4. **Version Management:**
   - Handle multiple versions of same node
   - Workflows specify which version they used
   - Backward compatibility warnings

**Developer Experience Goals:**
- **Minimal Boilerplate:** Declaring a new node should be ~20 lines of definition
- **Automatic UI Generation:** Parameters automatically appear in UI
- **Hot Reload:** Change a Python node, reload without restarting app
- **Testing Framework:** Easy to unit test nodes in isolation

---

### Module 4: Pipeline Execution Engine (The Kernel Core)

**Purpose:** Execute processing graphs with real-time performance and concurrency

**Key Concepts:**

**Graph vs. Pipeline:**
- **Graph:** Static definition (blueprint)
- **Pipeline:** Runtime instance (executing blueprint)
- One graph can have multiple concurrent pipeline instances

**Execution Models:**

**Model 1: Streaming (Real-time)**
```
Flow: Source → Transform1 → Transform2 → Sink
Behavior: Continuous, process frames as they arrive
Timing: Nanosecond-level latency critical
```

**Model 2: Batch (Offline)**
```
Flow: Load All → Process → Save Results
Behavior: Process complete dataset at once
Timing: Throughput important, latency less so
```

**Model 3: Triggered (On-demand)**
```
Flow: Wait for Trigger → Run Pipeline → Return Result → Wait
Behavior: Execute graph in response to external event
Example: Test station (trigger = DUT detected)
```

**Concurrency Architecture:**

**Problem:** If a pipeline takes 2 seconds to complete, and triggers arrive every 0.5 seconds, we need to run multiple pipelines concurrently.

**Solution:** Pipeline Pool
- Maintain a pool of pipeline instances
- When trigger arrives:
  1. Acquire available pipeline from pool (or create new if under limit)
  2. Execute asynchronously
  3. Return to pool when complete
- Prevents resource exhaustion (maximum concurrent limit)

**Data Flow Mechanics:**

**Channel-Based Architecture:**
Each edge in the graph becomes a message queue:
```
[Node A] → [Queue] → [Node B]
```

**Queue Properties:**
- **Bounded:** Fixed maximum size (prevents memory explosion)
- **Backpressure Strategy:** What to do when queue is full?
  - Drop oldest (real-time: prefer fresh data)
  - Drop newest (batch: preserve all data)
  - Block sender (critical: cannot lose data)

**Scheduler:**
- **Task per Node:** Each node runs as independent async task
- **Work Stealing:** Idle threads can help busy threads
- **Priority Levels:** Critical nodes get more CPU time
- **Deadline Scheduling:** Abort if taking too long

**State Management:**

**Stateless Nodes (Preferred):**
- No memory between frames
- Can be parallelized easily
- Example: Gain, filters (if designed properly)

**Stateful Nodes (Careful):**
- Maintain internal state across frames
- Requires careful synchronization
- Example: FFT with overlapping windows, accumulators

**Error Handling Philosophy:**

**Node-Level Errors:**
- **Recoverable:** Log warning, use default/last-known-good value, continue
- **Unrecoverable:** Stop node, propagate error, mark pipeline as failed

**Pipeline-Level Errors:**
- Preserve state (checkpoint)
- Notify user
- Offer: retry, skip, abort

**System-Level Errors:**
- Hardware disconnected
- Out of memory
- Thread panic
- Action: Graceful degradation, save what you can, alert user

---

### Module 5: Real-time Visualization Subsystem

**Purpose:** Display streaming data at 60+ fps without blocking the kernel

**The Fundamental Problem:**
- Kernel may produce data at 192 kHz (192,000 samples/sec)
- Display can only refresh at 60 Hz (60 times/sec)
- Sending all data to UI would saturate communication channel

**Solution Architecture:**

**Tier 1: Data Decimation (Kernel Side)**
- Downsample high-rate data to display rate
- Use intelligent downsampling (min/max pairs to preserve peaks)
- Example: 48000 Hz → 4800 Hz (every 10th sample, but keep min/max of each 10)

**Tier 2: Ring Buffer (Shared Memory)**
- Kernel writes to fixed-size circular buffer
- UI reads from buffer at its own pace
- No blocking: If UI is slow, it sees older data but kernel continues

**Tier 3: Render Optimization (UI Side)**
- Use high-performance plotting libraries (WebGL, Canvas)
- Batch updates: Don't redraw on every sample
- Cull off-screen data
- Level-of-detail: Fewer points when zoomed out

**Visualization Types:**

**Waveform (Time Domain):**
- X-axis: Time
- Y-axis: Amplitude
- Multi-channel: Stacked or overlaid
- Features: Cursors for measurement, zoom, pan

**Spectrum (Frequency Domain):**
- X-axis: Frequency (Hz)
- Y-axis: Magnitude (dB)
- Features: Peak hold, averaging, log/linear scales

**Spectrogram (Time-Frequency):**
- X-axis: Time
- Y-axis: Frequency
- Color: Magnitude
- Features: Color map selection, zoom, playback cursor

**Data Transport Options:**

**Option A: Event-Based (Low bandwidth)**
- Kernel emits events with downsampled data
- Frontend receives events, updates plots
- Good for: Slow refresh rates, low channel counts

**Option B: Shared Memory (High bandwidth)**
- Kernel writes to shared memory region
- Frontend reads directly from memory
- Good for: High refresh rates, many channels
- Challenge: Requires platform-specific implementation

**Option C: WebSocket Binary (Medium bandwidth)**
- Kernel streams binary data over WebSocket
- Frontend deserializes and plots
- Good for: Remote access, web-based UI

---

### Module 6: State & Configuration Management

**Purpose:** Persist and synchronize state between UI, kernel, and disk

**State Categories:**

**1. Graph State (Workflow Definition)**
- Node positions, connections, parameters
- Format: JSON (human-readable, version-controllable)
- Storage: User's documents folder, project directories
- Versioning: Git-friendly format

**2. Runtime State (Execution Status)**
- Which pipelines are running?
- Current frame count, throughput, errors
- Format: Ephemeral (in-memory), optionally logged
- Storage: Not persisted (recreated on restart)

**3. Application State (User Preferences)**
- UI layout, theme, keyboard shortcuts
- Last opened files, recent workflows
- Format: Key-value store or structured config
- Storage: Application data directory

**4. Hardware State (Device Configuration)**
- Discovered devices, selected devices, calibrations
- Channel mappings, gain settings
- Format: Structured config (JSON/TOML)
- Storage: Application data directory, per-project overrides

**Synchronization Strategy:**

**Unidirectional Data Flow:**
```
User Action → UI → Command → Kernel → Update → Event → UI Update
```

**Key Principle:** UI does not directly modify shared state. It sends commands to kernel, kernel updates state, kernel emits event, UI reacts.

**Why:** Prevents race conditions, makes debugging easier, enables undo/redo

**Conflict Resolution:**
- **Optimistic Updates:** UI updates immediately (perceived performance)
- **Rollback on Error:** If kernel rejects command, UI reverts
- **Version Vectors:** Detect concurrent modifications

---

## Part III: Developer & User Journeys

### User Journey 1: Scientist Analyzing Recorded Data

**Goal:** Load a WAV file, apply filters, compute FFT, export spectrum as CSV

**Steps:**
1. **Open Application** → See empty canvas
2. **Drag "Audio File Reader" node** → Prompts to select file
3. **Drag "High-Pass Filter" node** → Connect to reader output
4. **Drag "FFT" node** → Connect to filter output
5. **Drag "CSV Exporter" node** → Connect to FFT output
6. **Configure Parameters:**
   - Filter cutoff: 100 Hz
   - FFT size: 2048
   - Export path: Desktop/results.csv
7. **Click "Validate"** → Graph checks for errors (all clear)
8. **Click "Run"** → Pipeline executes
9. **View Log** → "Exported 50000 rows to results.csv"
10. **Open CSV** → Spreadsheet with frequency, magnitude columns

**Key Takeaway:** Entire workflow without writing code. Drag → Connect → Configure → Run.

---

### User Journey 2: Production Line Real-time Testing

**Goal:** When DUT (Device Under Test) is detected, play test tone, measure response, pass/fail based on THD

**Steps:**
1. **Build Graph:**
   ```
   USB Trigger Input → Function Generator → Speaker Output
                                         ↓
                                   Microphone Input → THD Analyzer → Pass/Fail Logic → Relay Output
   ```

2. **Configure Trigger:** USB trigger from fixture (DUT detected)

3. **Configure Test:**
   - Play 1 kHz sine, 1 second duration
   - Measure THD on microphone
   - Pass if THD < 1%
   - Green light if pass, red if fail

4. **Deploy:**
   - Save graph as "speaker_test.json"
   - Set to auto-start on boot
   - Place on production line PC

5. **Operation:**
   - Operator places DUT in fixture
   - Trigger fires automatically
   - Graph executes (takes 1.5 seconds)
   - Light indicates pass/fail
   - Repeat for next DUT

**Key Takeaway:** Same visual tool used for lab work now runs production tests. No separate "deployment" process needed.

---

### Developer Journey 1: Adding a Rust Analysis Node

**Goal:** Create a "Cepstrum" node (advanced signal processing)

**Steps:**
1. **Create Module File:** `src/nodes/cepstrum.rs`

2. **Define Node Struct:**
   - Name: "Cepstrum"
   - Category: "Advanced Analysis"
   - Inputs: One audio channel
   - Outputs: Cepstrum coefficients
   - Parameters: Number of coefficients (default 13)

3. **Implement Processing Logic:**
   - Take input frame
   - Apply FFT
   - Take logarithm of magnitude
   - Apply inverse FFT
   - Return cepstrum coefficients

4. **Register Node:**
   - Add annotation/macro that auto-registers at compile time

5. **Test:**
   - Write unit test with synthetic sine wave
   - Verify expected output

6. **Build:**
   - Recompile application
   - Node appears in palette automatically

7. **Document:**
   - Add description, parameter explanations
   - Appears as tooltip in UI

**Key Takeaway:** Developer focuses on algorithm. Framework handles UI generation, serialization, type checking.

---

### Developer Journey 2: Adding a Python Script Node

**Goal:** Create anomaly detector using scikit-learn

**Steps:**
1. **Create Python File:** `~/.streamlab/nodes/anomaly_detector.py`

2. **Define Function:**
   ```python
   # (Pseudo-code representation)
   name: "Anomaly Detector"
   category: "AI"
   inputs: ["features": Float64Array]
   outputs: ["is_anomaly": Boolean, "score": Float64]
   parameters: ["threshold": Float64 = 3.0]

   def process(inputs, params):
       features = inputs["features"]
       # Use One-Class SVM
       clf = load_model("anomaly_model.pkl")
       score = clf.score_samples([features])[0]
       is_anomaly = score < -params["threshold"]
       return {"is_anomaly": is_anomaly, "score": score}
   ```

3. **Train Model Separately:**
   - Use normal data to train One-Class SVM
   - Save as `anomaly_model.pkl`

4. **Reload Nodes:**
   - In app: "Tools" → "Reload Python Nodes"
   - Node appears in palette

5. **Use in Graph:**
   - Drag onto canvas
   - Connect to feature extraction node
   - Set threshold parameter
   - Run

**Key Takeaway:** No compilation, no restart. Write function, reload, use.

---

### Developer Journey 3: Integrating Custom Hardware

**Goal:** Support a proprietary vibration sensor with USB interface

**Steps:**
1. **Understand Hardware:**
   - USB vendor/product ID
   - Data format (16-bit signed, 10 kHz sample rate, 3 axes)
   - Communication protocol (bulk transfers)

2. **Create Driver Module:**
   - Implement HAL "Streaming Source" interface
   - Discovery: Enumerate USB devices, match vendor ID
   - Configure: Set sample rate (fixed at 10 kHz)
   - Stream: Read bulk endpoint, parse into DataFrame

3. **Map Channels:**
   - Physical: Raw bytes
   - Logical: ["accel_x", "accel_y", "accel_z"]
   - Units: g's (gravity units)

4. **Calibration:**
   - Read calibration coefficients from device EEPROM
   - Apply gain/offset to convert raw → g's

5. **Register Driver:**
   - Add to HAL registry
   - Declare device category: "Vibration Sensor"

6. **Test:**
   - Connect physical device
   - Open Device Manager in app
   - Device appears in list
   - Select device
   - Create simple graph: Sensor → Waveform Display
   - Tap sensor, see vibration on screen

**Key Takeaway:** Once driver is written, device works with all existing nodes. No changes to UI or kernel needed.

---

## Part IV: Non-Functional Requirements & Design Principles

### Performance Targets

**Latency (Time from input to output):**
- Real-time Audio: < 10 ms (perceptual threshold)
- Real-time Vibration: < 50 ms
- Triggered Testing: < 200 ms (human perception of responsiveness)
- Batch Processing: N/A (throughput matters, not latency)

**Throughput (Data processing rate):**
- Audio: 192 kHz × 64 channels = 12.3 million samples/sec
- Vibration: 50 kHz × 256 channels = 12.8 million samples/sec
- Target: Process at 2× real-time (safety margin)

**Memory:**
- Baseline (Idle): < 200 MB
- Per Pipeline: < 50 MB (varies by graph complexity)
- Ring Buffers: Configurable (default 10 seconds of data)
- Total: Aim for < 2 GB on a machine with 16 GB RAM

**CPU:**
- Utilize all available cores
- Real-time pipelines: Pin to dedicated cores if possible
- Batch pipelines: Use all remaining cores

---

### Scalability Considerations

**Horizontal Scaling (Future):**
- Design allows splitting graph across machines
- Example: 1000-channel microphone array across 10 PCs
- Requires network-transparent DataFrame transport
- Orchestrator coordinates distributed execution

**Vertical Scaling (Current):**
- Efficient use of single machine resources
- SIMD vectorization for DSP operations
- GPU acceleration for AI inference (optional)
- Zero-copy data passing where possible

---

### Security & Safety

**Sandboxing:**
- Python nodes run in restricted interpreter
- Cannot access filesystem outside designated folders
- Network access requires explicit permission

**Input Validation:**
- All user parameters validated before execution
- Hardware commands validated (prevent damage)
- Graph structure validated (no cycles, type safety)

**Error Isolation:**
- Node failure doesn't crash kernel
- Pipeline failure doesn't crash application
- Hardware error doesn't freeze UI

**Data Safety:**
- Automatic saves (workflow every 30 seconds)
- Crash recovery (restore last state)
- Export results incrementally (don't lose hours of computation)

---

### Extensibility & Future-Proofing

**API Versioning:**
- Node interface versioned independently
- Graphs declare which API version they target
- Old graphs can run on new kernels (compatibility layer)

**Plugin Discovery:**
- No central registry required
- Drop file in folder → discovered at startup
- Distributed via package manager (future)

**Cross-Platform:**
- Windows, macOS, Linux support from day one
- Hardware drivers are platform-specific, but HAL abstracts this
- UI should adapt to OS conventions (native file dialogs, etc.)

---

## Part V: What This Framework Does NOT Specify

**Implementation Details Left Open:**

1. **Programming Language:**
   - Rust suggested for performance, but core concepts apply to any language
   - Could be C++, Go, Zig, etc.

2. **UI Framework:**
   - React suggested, but could be Vue, Svelte, Qt, Electron, Tauri
   - As long as it supports: Canvas, drag-and-drop, dynamic forms

3. **Serialization Format:**
   - JSON suggested for graphs, but could be YAML, Protocol Buffers, MessagePack
   - As long as it's: versionable, human-readable (for git), cross-platform

4. **Data Format:**
   - Apache Arrow suggested for DataFrames, but could be custom binary, HDF5, FlatBuffers
   - As long as it supports: zero-copy, multi-channel, metadata

5. **Communication Protocol:**
   - Tauri Events, gRPC, WebSockets, Shared Memory
   - Choose based on performance needs and deployment model

6. **Node Implementation Language:**
   - Nodes can be Rust, Python, Julia, MATLAB, C++, WebAssembly
   - As long as they implement the node interface

---

## Part VI: Success Criteria

**For End Users:**
- ✅ Can build a useful workflow in under 10 minutes (first time)
- ✅ Can build a complex workflow in under 1 hour (experienced user)
- ✅ Can run real-time analysis without stuttering or dropped frames
- ✅ Can see clear error messages when something goes wrong
- ✅ Can share workflows with colleagues (platform-independent files)

**For Algorithm Developers:**
- ✅ Can write a new node in under 1 hour (simple transform)
- ✅ Can test a node in isolation without the full application
- ✅ Can hot-reload Python nodes without restarting
- ✅ Can access comprehensive documentation and examples

**For Hardware Developers:**
- ✅ Can integrate a new device in under 1 day (with SDK provided by manufacturer)
- ✅ Device immediately works with all existing analysis nodes
- ✅ Can define custom calibration workflows

**For System Integrators:**
- ✅ Can deploy to production line without code changes
- ✅ Can configure via files (no hardcoded paths)
- ✅ Can monitor health remotely (metrics, logs)
- ✅ Can update workflows without redeploying software

---

## Implementation Progress

**Phase 7: Hardware Device Management** ✅ Complete (2025-12-01)
- DeviceProfile data model with configuration and metadata
- Device persistence layer (JSON storage)
- DeviceManager for lifecycle management
- Channel mapping (Identity, Reordering, Selection, Merging, Duplication)
- Tauri command layer for device CRUD operations
- Device Manager UI with discovery and configuration
- Integration structure with AudioSourceNode pipeline deployment

See: `docs/plans/2025-12-01-hardware-device-management.md` for details.

**Phase 6: Graph-to-Pipeline Integration** ✅ Complete (2025-11-27)
- Graph translator (frontend → backend format)
- Pipeline deployment via Tauri commands
- Error handling and status propagation
- End-to-end testing from UI to backend

See: `docs/implementation/phase6-completion.md` for details.

**Next Phase: Complete Device-to-Pipeline Integration** ⏳ Planned
- Finalize async device creation in pipeline deployment
- Integrate NodePropertiesPanel into UI
- Real-time channel mapping application
- Device calibration implementation

---

## Conclusion

This framework defines a **flexible architecture** for streaming multi-physics analysis. It is:

- **Layered** - Clear separation of concerns
- **Modular** - Each module has a single responsibility
- **Extensible** - Add nodes, devices, visualizations without modifying core
- **User-Centric** - Non-programmers can use, programmers can extend
- **Performance-Oriented** - Designed for real-time processing
- **Cross-Platform** - Not tied to specific OS or hardware

**The Three Pillars:**
1. **Visual Orchestration** - Drag-and-drop flowcharts
2. **Unified Kernel** - Streaming execution engine
3. **Hardware Abstraction** - Any device → Standard interface

**Next Steps:**
1. Prototype a minimal kernel (3 nodes: Source, Gain, Sink)
2. Build basic UI (canvas, palette, run button)
3. Integrate one real hardware device
4. Validate performance with real-world data
5. Iterate based on user feedback

This framework is not code - it's a **mental model** for how the system should be organized. Actual implementation will discover details not specified here. That's expected. The framework provides structure, not constraints.
