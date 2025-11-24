# Implementation Plan: Complete StreamLab Core Project Proposal

**Date:** 2025-11-24
**Objective:** Complete the project.md proposal by adding Section 7 (Kernel Architecture & Advanced Improvements) and Section 8 (Interface Design Specifications)

## Overview

This plan details the steps to enhance the existing project.md with comprehensive kernel architecture documentation and detailed interface design specifications. The additions will provide both high-level conceptual designs and technical implementation details.

## Prerequisites

- Existing project.md file at `/Users/fh/Code/audiotab/project.md`
- Understanding of the current project structure (Rust core, React frontend, Tauri bridge)
- Access to edit markdown files

---

## Task 1: Add Section 7 - Kernel Architecture & Advanced Improvements

**File:** `/Users/fh/Code/audiotab/project.md`
**Location:** After Section 6 (Definition of Done), before any appendices

### Step 1.1: Add Section 7 Header and Introduction

Insert the following after line 223 (end of Section 6):

```markdown

---

## 7. Kernel Architecture & Advanced Improvements

This section provides an in-depth look at the core engine architecture and advanced features needed to achieve production-grade performance, reliability, and scalability.

### 7.1 Enhanced Pipeline Execution Model

The pipeline executor is the heart of the streaming analysis system. Beyond the basic async execution described in Phase 1, the production system requires sophisticated scheduling and lifecycle management.

#### 7.1.1 Pipeline State Machine

Each Pipeline instance follows a strict state machine:

```
Idle → Initializing → Running → [Paused] → Completed
                ↓                    ↓
              Error ← ─ ─ ─ ─ ─ ─ ← ─┘
```

**State Definitions:**
- **Idle**: Pipeline definition exists but no resources allocated
- **Initializing**: Nodes are being created, hardware devices are being opened, buffers allocated
- **Running**: Active data processing, frames flowing through nodes
- **Paused**: Execution suspended, state preserved, can resume
- **Completed**: All data processed, resources being cleaned up
- **Error**: Unrecoverable error occurred, requires manual intervention

**Implementation Requirements:**
- File: `src/engine/pipeline_state.rs`
- Implement `PipelineState` enum and `StateMachine` trait
- Each state transition must emit events to the frontend via Tauri
- State transitions must be atomic (use `tokio::sync::RwLock`)

```rust
pub enum PipelineState {
    Idle,
    Initializing { progress: f32 },
    Running { start_time: Instant, frames_processed: u64 },
    Paused { pause_time: Instant },
    Completed { duration: Duration, total_frames: u64 },
    Error { error_msg: String, recoverable: bool },
}

#[async_trait]
pub trait StateMachine {
    async fn transition_to(&mut self, new_state: PipelineState) -> Result<()>;
    fn can_transition(&self, target: &PipelineState) -> bool;
    fn current_state(&self) -> &PipelineState;
}
```

#### 7.1.2 Priority-Based Task Scheduling

Not all pipelines are equal. Interactive previews need low latency, while batch exports can tolerate delays.

**Priority Levels:**
1. **Critical** (0-10ms target latency): Real-time monitoring, safety-critical analysis
2. **High** (10-50ms): User-triggered interactive analysis
3. **Normal** (50-200ms): Background automated testing
4. **Low** (>200ms): Batch processing, exports, calibration

**Implementation:**
- File: `src/engine/scheduler.rs`
- Use `tokio::task::JoinSet` with priority queues
- Implement work-stealing for load balancing
- Add deadline scheduling (abort if taking too long)

```rust
pub struct PipelineScheduler {
    priority_queues: HashMap<Priority, VecDeque<PipelineTask>>,
    active_tasks: JoinSet<Result<DataFrame>>,
    max_concurrent: usize,
}

impl PipelineScheduler {
    pub async fn schedule(&mut self, task: PipelineTask, priority: Priority) {
        // Higher priority tasks preempt lower priority ones
        if self.active_tasks.len() >= self.max_concurrent {
            self.maybe_preempt(priority).await;
        }
        self.spawn_task(task, priority).await;
    }
}
```

#### 7.1.3 Resource Pooling & Reuse

Creating and destroying pipelines for every trigger is expensive. Implement object pooling.

**Pool Types:**
1. **Pipeline Instance Pool**: Reuse entire pipeline instances (for identical graphs)
2. **Node Pool**: Reuse expensive nodes (e.g., FFT with pre-allocated buffers)
3. **Buffer Pool**: Reuse memory allocations for DataFrames

**Implementation:**
- File: `src/engine/pools.rs`
- Use `crossbeam::queue::ArrayQueue` for lock-free pooling
- Implement `Poolable` trait with `reset()` method
- Set pool size limits based on memory constraints

```rust
pub struct PipelinePool {
    pools: HashMap<GraphId, ArrayQueue<Box<dyn Pipeline>>>,
    max_pool_size: usize,
}

impl PipelinePool {
    pub fn acquire(&self, graph_id: &GraphId) -> Option<Box<dyn Pipeline>> {
        self.pools.get(graph_id)?.pop()
    }

    pub fn release(&self, graph_id: GraphId, mut pipeline: Box<dyn Pipeline>) {
        pipeline.reset(); // Clear internal state
        if let Some(pool) = self.pools.get(&graph_id) {
            let _ = pool.push(pipeline); // Ignore if pool full
        }
    }
}
```

#### 7.1.4 Checkpoint & Recovery System

For long-running analyses (e.g., 24-hour vibration monitoring), support checkpointing.

**Features:**
- Periodic state snapshots to disk
- Resume from last checkpoint on crash
- Configurable checkpoint interval (e.g., every 1000 frames or 60 seconds)

**Implementation:**
- File: `src/engine/checkpoint.rs`
- Serialize pipeline state + node states to MessagePack/Bincode
- Use WAL (Write-Ahead Log) pattern for crash consistency
- Store checkpoints in `~/.streamlab/checkpoints/{pipeline_id}/`

### 7.2 Zero-Copy Data Architecture

Copying large multi-channel arrays is a performance killer. Aggressive use of Arc, memory mapping, and SIMD.

#### 7.2.1 Advanced DataFrame Design

The DataFrame from Section 2.1 needs enhancement for zero-copy semantics.

**Enhanced DataFrame:**
```rust
use arrow::array::{Array, Float64Array};
use arrow::datatypes::Schema;

pub struct DataFrame {
    pub timestamp: u64,
    pub sequence_id: u64,

    // Use Apache Arrow for columnar, zero-copy data
    pub data: Arc<arrow::record_batch::RecordBatch>,

    // Metadata (small, can be cloned cheaply)
    pub metadata: Arc<HashMap<String, String>>,

    // Optional memory-mapped backing for huge datasets
    pub mmap_region: Option<Arc<Mmap>>,
}

impl DataFrame {
    // Zero-copy column extraction
    pub fn get_channel(&self, name: &str) -> Option<&Float64Array> {
        let column = self.data.column_by_name(name)?;
        column.as_any().downcast_ref::<Float64Array>()
    }

    // Create a view (no copy) with different metadata
    pub fn with_metadata(&self, key: String, value: String) -> Self {
        let mut new_meta = (*self.metadata).clone();
        new_meta.insert(key, value);
        DataFrame {
            timestamp: self.timestamp,
            sequence_id: self.sequence_id,
            data: Arc::clone(&self.data),
            metadata: Arc::new(new_meta),
            mmap_region: self.mmap_region.clone(),
        }
    }
}
```

**Why Apache Arrow?**
- Columnar layout: Better CPU cache utilization
- Zero-copy inter-process communication
- Native SIMD support
- Language-agnostic (can pass to Python without conversion)

#### 7.2.2 Memory Mapping for Large Datasets

When loading multi-gigabyte historical data for offline analysis:

**Implementation:**
- File: `src/core/mmap_source.rs`
- Use `memmap2` crate
- Map files as `Float64Array` directly (ensure alignment)
- OS handles paging (no manual buffer management)

```rust
use memmap2::Mmap;

pub struct MmapDataSource {
    mmap: Arc<Mmap>,
    num_channels: usize,
    samples_per_channel: usize,
}

impl MmapDataSource {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        // Parse header to get dimensions
        Ok(Self { mmap: Arc::new(mmap), ... })
    }

    pub fn get_channel_slice(&self, channel: usize) -> &[f64] {
        let offset = channel * self.samples_per_channel * 8; // 8 bytes per f64
        let ptr = unsafe { self.mmap.as_ptr().add(offset) as *const f64 };
        unsafe { std::slice::from_raw_parts(ptr, self.samples_per_channel) }
    }
}
```

#### 7.2.3 SIMD Optimization Patterns

All DSP kernels (FFT, filtering, correlation) must use SIMD when available.

**Strategy:**
1. Use `std::simd` (portable SIMD, Rust nightly as of 2024)
2. Fallback to scalar for unsupported targets
3. Auto-vectorization via slice operations where possible

**Example: Vectorized Gain Application**
```rust
use std::simd::f64x4;

pub fn apply_gain_simd(samples: &mut [f64], gain: f64) {
    let gain_vec = f64x4::splat(gain);
    let chunks = samples.chunks_exact_mut(4);

    for chunk in chunks {
        let mut vec = f64x4::from_slice(chunk);
        vec *= gain_vec;
        vec.copy_to_slice(chunk);
    }

    // Handle remainder
    let remainder = samples.chunks_exact_mut(4).into_remainder();
    for sample in remainder {
        *sample *= gain;
    }
}
```

### 7.3 Advanced Scheduling & Orchestration

#### 7.3.1 CPU Affinity for Real-Time Nodes

Pin real-time critical nodes to specific CPU cores to avoid context switches.

**Implementation:**
- File: `src/engine/affinity.rs`
- Use `core_affinity` crate
- Configuration: User marks nodes as "real-time" in graph JSON
- Automatically pin to isolated cores (if available)

```rust
pub fn pin_pipeline_to_core(pipeline_id: &str, core_id: usize) -> Result<()> {
    let core_ids = core_affinity::get_core_ids()
        .ok_or_else(|| anyhow!("Could not get core IDs"))?;

    if core_id >= core_ids.len() {
        bail!("Core {} not available", core_id);
    }

    core_affinity::set_for_current(core_ids[core_id]);
    Ok(())
}
```

#### 7.3.2 Dynamic Pipeline Recompilation

When user edits a graph while it's running, support hot-swapping without full restart.

**Approach:**
1. Compute graph diff (added/removed/modified nodes)
2. For unchanged subgraphs, reuse existing node instances
3. For changed parts, create new nodes and splice them in
4. Use double-buffering: Old pipeline keeps running until new one is ready

**Implementation:**
- File: `src/engine/hot_reload.rs`
- Compute graph hash for each subgraph
- Maintain version numbers for each node
- Use `Arc::make_mut` for copy-on-write node updates

#### 7.3.3 Distributed Execution (Future-Proofing)

For very large installations (e.g., 1000-channel microphone arrays), support splitting computation across machines.

**Design (Not Phase 1, but architecture must support it):**
- Each machine runs a `WorkerNode` daemon
- Central `Orchestrator` partitions graph
- Use gRPC streams for inter-machine DataFrame transport
- Automatic failover if a worker dies

**Architecture Requirement:**
- All nodes must be **stateless** or explicitly serialize state
- DataFrame must be `Serialize + Deserialize`
- Network topology aware scheduling

### 7.4 Error Handling & Observability

#### 7.4.1 Hierarchical Error Propagation

Errors can occur at multiple levels:
1. **Hardware Layer**: Device disconnected, buffer overrun
2. **Node Layer**: Algorithm failure (e.g., FFT on invalid data)
3. **Pipeline Layer**: Deadlock, timeout
4. **Orchestrator Layer**: Out of memory, configuration error

**Error Types:**
```rust
#[derive(Error, Debug)]
pub enum StreamLabError {
    #[error("Hardware error: {source}")]
    Hardware {
        device_id: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Node '{node_id}' failed: {reason}")]
    NodeProcessing {
        node_id: String,
        reason: String,
        recoverable: bool,
    },

    #[error("Pipeline timeout after {timeout:?}")]
    PipelineTimeout {
        pipeline_id: String,
        timeout: Duration,
    },

    #[error("Orchestrator error: {0}")]
    Orchestrator(String),
}
```

**Propagation Strategy:**
- Node errors: Try to isolate (use fallback data if available)
- Pipeline errors: Stop pipeline, notify user, preserve state
- Hardware errors: Attempt reconnect (3 retries), then fail pipeline
- Orchestrator errors: Global alert, may require restart

#### 7.4.2 Circuit Breaker Pattern

If a node fails repeatedly, stop trying to prevent cascading failures.

**Implementation:**
- File: `src/core/circuit_breaker.rs`
- Track failure rate per node type
- States: Closed (normal) → Open (failing) → Half-Open (testing recovery)
- After N consecutive failures, open circuit for T seconds

```rust
pub struct CircuitBreaker {
    state: Arc<RwLock<BreakerState>>,
    failure_threshold: usize,
    timeout: Duration,
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        match *self.state.read().await {
            BreakerState::Open => bail!("Circuit breaker open"),
            BreakerState::HalfOpen | BreakerState::Closed => {
                match f() {
                    Ok(result) => {
                        self.on_success().await;
                        Ok(result)
                    }
                    Err(e) => {
                        self.on_failure().await;
                        Err(e)
                    }
                }
            }
        }
    }
}
```

#### 7.4.3 Observability & Metrics

Expose runtime metrics for monitoring and debugging.

**Metrics to Track:**
- Pipeline throughput (frames/sec)
- Node processing latency (p50, p95, p99)
- Memory usage per pipeline
- Queue depths (backpressure indicator)
- Error rates

**Implementation:**
- Use `metrics` crate with Prometheus exporter
- Expose metrics endpoint at `http://localhost:9090/metrics`
- Frontend can query and display graphs

```rust
use metrics::{counter, histogram, gauge};

impl Pipeline {
    async fn process_frame(&self, frame: DataFrame) -> Result<DataFrame> {
        let start = Instant::now();

        let result = self.inner_process(frame).await;

        histogram!("pipeline.frame.latency", start.elapsed().as_secs_f64());

        if result.is_ok() {
            counter!("pipeline.frames.success").increment(1);
        } else {
            counter!("pipeline.frames.error").increment(1);
        }

        gauge!("pipeline.queue.depth").set(self.queue.len() as f64);

        result
    }
}
```

**Logging Strategy:**
- Use `tracing` crate with structured logging
- Log levels: ERROR (unrecoverable), WARN (degraded), INFO (lifecycle events), DEBUG (detailed flow), TRACE (per-frame)
- Write logs to `~/.streamlab/logs/` with rotation
- Expose log viewer in UI (bottom panel)

---
```

**Verification:**
- Confirm the section is added after Section 6
- Check markdown formatting is correct
- Ensure code blocks use proper syntax highlighting

### Step 1.2: Add Remaining Kernel Subsections

Continue adding after Section 7.4:

```markdown

### 7.5 Memory Management & Leak Prevention

Long-running streaming processes are prone to memory leaks. Aggressive strategies required.

#### 7.5.1 Frame Lifecycle Tracking

Every DataFrame must have a clear lifecycle: Created → In-Flight → Consumed → Dropped

**Debugging Tool:**
- In debug builds, maintain a global registry of all live DataFrames
- Track allocation stack traces using `backtrace` crate
- Periodically dump report of DataFrames that haven't been dropped

```rust
#[cfg(debug_assertions)]
pub struct FrameRegistry {
    frames: Arc<RwLock<HashMap<u64, FrameDebugInfo>>>,
}

struct FrameDebugInfo {
    sequence_id: u64,
    created_at: Instant,
    allocation_trace: Backtrace,
}

impl DataFrame {
    pub fn new(...) -> Self {
        let frame = DataFrame { ... };
        #[cfg(debug_assertions)]
        FrameRegistry::global().register(&frame);
        frame
    }
}

impl Drop for DataFrame {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        FrameRegistry::global().unregister(self.sequence_id);
    }
}
```

#### 7.5.2 Bounded Queues & Backpressure

Unbounded channels can grow infinitely. All inter-node channels must be bounded.

**Configuration:**
- Default queue size: 16 frames (configurable per edge in graph)
- Behavior on full queue:
  - **Drop Oldest** (for real-time): Discard oldest frame, insert new
  - **Drop Newest** (for batch): Reject new frame, return error
  - **Block** (for critical data): Wait until space available (may deadlock!)

```rust
pub enum BackpressureStrategy {
    DropOldest,
    DropNewest,
    Block { timeout: Duration },
}

pub fn create_channel(capacity: usize, strategy: BackpressureStrategy)
    -> (Sender, Receiver)
{
    match strategy {
        BackpressureStrategy::DropOldest => {
            // Custom channel implementation
            ring_channel::bounded(capacity)
        },
        BackpressureStrategy::DropNewest => {
            let (tx, rx) = tokio::sync::mpsc::channel(capacity);
            (tx.into(), rx.into())
        },
        BackpressureStrategy::Block { timeout } => {
            // Wrapper around tokio channel with timeout
            blocking_channel::bounded(capacity, timeout)
        }
    }
}
```

#### 7.5.3 Memory Pool Pre-Allocation

Pre-allocate memory at pipeline initialization to avoid runtime allocations.

**Strategy:**
- At pipeline init, allocate `N` buffers of max expected frame size
- Use `crossbeam::queue::SegQueue` for lock-free pool
- Nodes borrow from pool, return when done

```rust
pub struct BufferPool {
    pool: SegQueue<Vec<f64>>,
    buffer_size: usize,
}

impl BufferPool {
    pub fn new(num_buffers: usize, buffer_size: usize) -> Self {
        let pool = SegQueue::new();
        for _ in 0..num_buffers {
            pool.push(vec![0.0; buffer_size]);
        }
        Self { pool, buffer_size }
    }

    pub fn acquire(&self) -> Option<Vec<f64>> {
        self.pool.pop()
    }

    pub fn release(&self, mut buffer: Vec<f64>) {
        buffer.clear();
        buffer.resize(self.buffer_size, 0.0);
        self.pool.push(buffer);
    }
}
```

### 7.6 Testing & Validation Infrastructure

The kernel must be rigorously tested. Beyond unit tests, need specialized testing harnesses.

#### 7.6.1 Synthetic Data Generators

**File:** `src/testing/generators.rs`

Implement deterministic signal generators for testing:
- Sine/Square/Triangle wave generators
- White/Pink noise generators
- Chirp (swept sine) generator
- Impulse response generator

```rust
pub struct SineGenerator {
    sample_rate: f64,
    frequency: f64,
    amplitude: f64,
    phase: f64,
}

impl SineGenerator {
    pub fn generate(&mut self, num_samples: usize) -> Vec<f64> {
        let mut samples = Vec::with_capacity(num_samples);
        let delta_phase = 2.0 * PI * self.frequency / self.sample_rate;

        for _ in 0..num_samples {
            samples.push(self.amplitude * self.phase.sin());
            self.phase += delta_phase;
            if self.phase > 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
        }
        samples
    }
}
```

#### 7.6.2 Pipeline Integration Tests

**File:** `tests/pipeline_integration_test.rs`

Test complete pipelines end-to-end:

```rust
#[tokio::test]
async fn test_basic_pipeline() {
    // 1. Build a test graph: Sine Gen -> Gain -> Assert
    let graph_json = json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "params": {"freq": 440.0}},
            {"id": "gain", "type": "Gain", "params": {"gain": 2.0}},
            {"id": "assert", "type": "AssertRMS", "params": {"expected_rms": 1.414}}
        ],
        "edges": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "assert"}
        ]
    });

    // 2. Execute pipeline
    let pipeline = PipelineBuilder::from_json(&graph_json).await.unwrap();
    let result = pipeline.run_once().await;

    // 3. Verify result
    assert!(result.is_ok(), "Pipeline failed: {:?}", result);
}
```

#### 7.6.3 Stress & Soak Testing

**File:** `tests/stress_test.rs`

- **Stress Test**: Spawn 100 concurrent pipelines, each with 50 nodes, run for 60 seconds
- **Soak Test**: Single pipeline, run for 24 hours, monitor memory (must not grow)
- **Chaos Test**: Randomly kill/restart nodes, inject errors, verify recovery

```rust
#[tokio::test]
#[ignore] // Only run with `cargo test --ignored`
async fn soak_test_24_hours() {
    let start = Instant::now();
    let duration = Duration::from_secs(24 * 3600);

    let pipeline = create_test_pipeline().await;

    while start.elapsed() < duration {
        pipeline.process_frame(generate_test_frame()).await.unwrap();

        // Check memory every 5 minutes
        if start.elapsed().as_secs() % 300 == 0 {
            let mem_usage = get_process_memory();
            assert!(mem_usage < MEMORY_LIMIT, "Memory leak detected!");
        }
    }
}
```

---
```

**Verification:**
- Ensure subsections 7.5 and 7.6 are complete
- Check all code examples compile (syntax check)
- Verify section numbering is sequential

---

## Task 2: Add Section 8 - Interface Design Specifications

**File:** `/Users/fh/Code/audiotab/project.md`
**Location:** After Section 7

### Step 2.1: Add Section 8 Header and Main Interface Design

Insert after Section 7:

```markdown

---

## 8. Interface Design Specifications

This section provides detailed UI/UX specifications for all major interfaces in the application. Designs balance power-user efficiency with approachability for new users.

### 8.1 Main Interface Layout (Workbench View)

The primary user interface follows a classic IDE-style layout optimized for flow-based programming.

#### 8.1.1 Overall Layout Structure

```
┌─────────────────────────────────────────────────────────────────┐
│ Menu Bar: File | Edit | View | Pipeline | Hardware | Help       │
├──────┬──────────────────────────────────────────────┬───────────┤
│      │                                              │           │
│ Node │                                              │ Property  │
│ Pal- │          Infinite Canvas                     │ Inspector │
│ ette │        (React Flow Editor)                   │           │
│      │                                              │  - Node   │
│ - Src│                                              │    Config │
│ - DSP│                                              │  - Params │
│ - AI │                                              │  - Info   │
│ - Out│                                              │           │
│      │                                              │           │
├──────┴──────────────────────────────────────────────┴───────────┤
│ Console / Logs / Metrics                         │ Status Bar  │
└──────────────────────────────────────────────────┴─────────────┘
```

**Dimensions (Default):**
- Window: 1920x1080 minimum recommended
- Left Sidebar (Node Palette): 240px fixed width, collapsible
- Right Sidebar (Property Inspector): 320px, resizable from 280-600px
- Bottom Panel (Console): 200px height, resizable, collapsible
- Status Bar: 24px fixed height

#### 8.1.2 Menu Bar Specification

**File Menu:**
- New Workflow (Ctrl+N)
- Open Workflow... (Ctrl+O)
- Save Workflow (Ctrl+S)
- Save As... (Ctrl+Shift+S)
- Recent Workflows (submenu, last 10)
- Import
  - Import Graph JSON
  - Import Python Node
- Export
  - Export Graph as JSON
  - Export Results as CSV/HDF5
- Exit (Ctrl+Q)

**Edit Menu:**
- Undo (Ctrl+Z)
- Redo (Ctrl+Y)
- Cut (Ctrl+X)
- Copy (Ctrl+C)
- Paste (Ctrl+V)
- Delete (Del)
- Select All (Ctrl+A)
- Find Node... (Ctrl+F) - Quick search palette

**View Menu:**
- Zoom In (Ctrl++)
- Zoom Out (Ctrl+-)
- Zoom to Fit (Ctrl+0)
- Toggle Node Palette (Ctrl+1)
- Toggle Property Inspector (Ctrl+2)
- Toggle Console (Ctrl+3)
- Full Screen (F11)

**Pipeline Menu:**
- Validate Graph (Ctrl+K) - Check for errors before run
- Run (Ctrl+R) - Start execution
- Pause (Ctrl+P)
- Stop (Ctrl+.)
- Step Through (F10) - Debug mode, step frame-by-frame
- Clear Results

**Hardware Menu:**
- Device Manager... - Open hardware config dialog
- Refresh Devices
- Calibrate Selected Device...
- Device Health Monitor

**Help Menu:**
- Documentation (F1)
- Example Workflows (submenu)
- Keyboard Shortcuts
- Check for Updates
- About StreamLab

#### 8.1.3 Node Palette (Left Sidebar)

**Structure:**
- Search box at top: "Search nodes..." (instant filter)
- Collapsible categories (can expand/collapse multiple simultaneously)
- Drag-and-drop nodes onto canvas

**Categories & Representative Nodes:**

**📥 Sources (Inputs)**
- Microphone Input
- Audio File Player
- Function Generator (Sine/Square/Triangle)
- CSV Data Reader
- TCP Stream Receiver
- USB Trigger Input

**🎛️ Signal Processing (DSP)**
- Gain/Attenuation
- High-Pass Filter
- Low-Pass Filter
- Band-Pass Filter
- FFT (Fast Fourier Transform)
- STFT (Short-Time FT)
- Spectrogram
- Correlation
- Convolution
- Resample

**🤖 AI & Machine Learning**
- PyTorch Model Inference
- TensorFlow Model Inference
- Anomaly Detector (One-Class SVM)
- Feature Extractor (MFCC, Chroma)
- Python Script (Custom)

**📊 Analysis & Metrics**
- RMS Calculator
- Peak Detector
- THD (Total Harmonic Distortion)
- SNR Calculator
- Statistical Summary (Mean/Std/Min/Max)

**🔀 Logic & Control**
- If/Else (Conditional Router)
- Switch (Multi-way router)
- Loop Controller
- Delay Buffer
- Frame Combiner (Merge channels)
- Frame Splitter

**📤 Outputs (Sinks)**
- Speaker Output
- File Writer (WAV/CSV/HDF5)
- TCP Stream Sender
- MQTT Publisher
- Visual Display (Waveform/Spectrum)
- Relay Control (GPIO)

**🔧 Utilities**
- Debug Print (to console)
- Assert (for testing)
- Benchmark (measure latency)
- Comment/Annotation

**Visual Design:**
- Each node has an icon (emoji or Lucide icon)
- Node name below icon
- On hover: Tooltip with description
- Drag behavior: Create ghost preview on canvas

#### 8.1.4 Infinite Canvas (Center Panel)

**Framework:** React Flow (https://reactflow.dev/)

**Features:**
- Infinite panning (middle-mouse drag or space+drag)
- Zoom: Mouse wheel or trackpad pinch
- Mini-map in bottom-right corner (toggleable)
- Grid background (optional, toggle in View menu)
- Snap-to-grid (optional, 20px grid)

**Node Appearance:**

```
┌────────────────────────┐
│ 🎤 Microphone Input    │  ← Title bar (draggable)
├────────────────────────┤
│ Device: [Dropdown ▾]   │  ← Parameters (inline editing)
│ Channels: 2            │
│ Sample Rate: 48000 Hz  │
├────────────────────────┤
│         ○ Output       │  ← Output port (right side)
└────────────────────────┘

For nodes with inputs:
        Input ○
┌────────────────────────┐
│ 🎚️ Gain               │
├────────────────────────┤
│ Gain: [2.0    ] dB     │  ← Slider or input
├────────────────────────┤
│         ○ Output       │
└────────────────────────┘
```

**Node States (Visual Feedback):**
- **Idle**: Default gray border
- **Running**: Pulsing green border
- **Processing**: Animated blue glow
- **Error**: Red border + error icon (🔴) in top-right
- **Warning**: Yellow border (e.g., parameter out of recommended range)

**Edge (Connection) Appearance:**
- Default: Gray bezier curve, 2px width
- Active (data flowing): Animated dashes, 3px, blue color
- Hovered: Highlighted, 4px
- Invalid connection attempt: Red, dashed

**Context Menu (Right-Click on Node):**
- Edit Parameters (or double-click node)
- Duplicate
- Enable/Disable (gray out, skip during execution)
- Delete
- View Logs
- Benchmark This Node

**Context Menu (Right-Click on Canvas):**
- Add Node (opens quick-add palette)
- Paste
- Organize
  - Auto-Layout (vertical)
  - Auto-Layout (horizontal)
  - Align Selected Nodes
- Background
  - Show/Hide Grid
  - Snap to Grid

#### 8.1.5 Property Inspector (Right Sidebar)

**Sections (Collapsible Accordions):**

**1. Node Information** (always visible when node selected)
- Node Type: `Gain`
- Node ID: `gain_1` (editable, must be unique)
- Description: (multi-line text area, user can annotate)

**2. Parameters** (dynamic based on node type)
- Each parameter rendered based on schema
  - **Number**: Slider + text input (shows units)
  - **Enum**: Dropdown
  - **Boolean**: Toggle switch
  - **String**: Text input
  - **File**: File picker button
  - **Color**: Color picker
- Real-time validation (show error message below parameter if invalid)
- Presets dropdown at top: "Load Preset..." (common configurations)

**3. Inputs/Outputs**
- List of input ports (name, connected from)
- List of output ports (name, connected to)
- Shows data types (e.g., "Audio Float64[1024]")

**4. Performance Metrics** (if pipeline is running)
- Frames Processed: 1,234
- Avg Latency: 2.3 ms
- Last Error: (if any)

**5. Advanced**
- Priority: [Normal ▾] (Critical/High/Normal/Low)
- Max Queue Size: [16]
- Backpressure Strategy: [Drop Oldest ▾]

**Example (Gain Node):**

```
┌─────────────────────────────┐
│ Node Information            │
│  Type: Gain                 │
│  ID: [gain_main_____]       │
│  Description:               │
│  [Applies gain to signal]   │
├─────────────────────────────┤
│ Parameters                  │
│  Preset: [Default ▾]        │
│                             │
│  Gain (dB)                  │
│  ├─────●─────┤ [6.0  ] dB   │
│  -20        +20              │
│                             │
│  Clip Protection            │
│  [✓] Enable                 │
├─────────────────────────────┤
│ Inputs                      │
│  ○ Input                    │
│    ← microphone_1.output    │
│    Type: Float64[1024]      │
├─────────────────────────────┤
│ Outputs                     │
│  ○ Output                   │
│    → fft_1.input            │
├─────────────────────────────┤
│ Performance ▾               │
│  Frames: 5,432              │
│  Latency: 0.8 ms (avg)      │
└─────────────────────────────┘
```

#### 8.1.6 Bottom Panel (Console/Logs/Metrics)

**Tabs:**

**1. Console**
- Shows log output from nodes (especially Debug Print nodes)
- Supports filtering by log level (ERROR/WARN/INFO/DEBUG)
- Search box
- Auto-scroll toggle
- Clear button

**2. System Logs**
- Structured logs from Rust backend
- Filterable by module (engine, HAL, nodes)
- Timestamp, level, message
- Click on error to jump to problematic node

**3. Metrics Dashboard**
- Real-time charts (last 60 seconds)
- Metrics:
  - Pipeline throughput (frames/sec)
  - System CPU usage (%)
  - System memory usage (MB)
  - Active pipelines count
- Powered by uPlot or Recharts

**4. Network Monitor** (if using TCP/MQTT nodes)
- List of active connections
- Bytes sent/received
- Connection status

**Visual Design:**
- Dark theme (consistent with VSCode aesthetic)
- Monospace font for logs
- Color-coded log levels:
  - ERROR: Red
  - WARN: Yellow
  - INFO: Blue
  - DEBUG: Gray

#### 8.1.7 Status Bar (Bottom)

**Left Side:**
- Pipeline State: "⚫ Idle" | "🟢 Running" | "🟡 Paused" | "🔴 Error"
- Active Pipelines: "3 active"

**Center:**
- Current file: "my_workflow.json" (clickable, opens file location)
- Modified indicator: "●" (if unsaved changes)

**Right Side:**
- Hardware Status: "🔌 3 devices connected"
- Backend Connection: "Connected" (or "Disconnected" if Tauri bridge lost)
- FPS counter (for visualization performance): "60 fps"

---
```

**Verification:**
- Confirm Section 8.1 is complete
- Check ASCII diagrams render correctly
- Verify all UI components are described

### Step 2.2: Add Analysis Configuration Interface

Continue after Section 8.1:

```markdown

### 8.2 Analysis Configuration Interface

When users need to configure complex analysis parameters or create reusable configurations.

#### 8.2.1 Quick Parameter Panel (Inline Editing)

For simple adjustments, users can edit parameters directly on the canvas without opening dialogs.

**Implementation:**
- Double-click node → Parameters appear in overlay below node
- Edit value → Press Enter to confirm, Esc to cancel
- Supports keyboard navigation (Tab to next parameter)

**Visual:**
```
      ┌────────────────────────┐
      │ 🎚️ Gain               │
      └────────────────────────┘
           ↓ (double-click)
      ┌────────────────────────┐
      │ 🎚️ Gain               │
      ├────────────────────────┤
      │ Gain (dB): [▓6.0▓] dB  │ ← Focused input
      │ [Apply] [Cancel]       │
      └────────────────────────┘
```

#### 8.2.2 Parameter Presets System

Users can save and load parameter configurations for nodes.

**Preset Manager UI (Dropdown in Property Inspector):**
- Built-in presets (read-only, shipped with app):
  - "Factory Default"
  - "Conservative" (safe parameters)
  - "Aggressive" (max performance)
- User presets (saved to `~/.streamlab/presets/{node_type}/`)
  - "My Config 1"
  - "Production Setup"
- Actions:
  - "Save Current as Preset..."
  - "Manage Presets..." (opens preset library dialog)

**Preset Library Dialog:**
```
┌─────────────────────────────────────────────────────┐
│ Preset Manager - Gain Node                         │
├──────────────────┬──────────────────────────────────┤
│ Available Presets│ Preview                          │
│                  │                                  │
│ Built-in         │ Name: Conservative               │
│  ▸ Factory       │ Description:                     │
│  ▾ Conservative  │ "Safe gain settings for prod..."│
│  ▸ Aggressive    │                                  │
│                  │ Parameters:                      │
│ User             │  - Gain: 3.0 dB                  │
│  ▸ My Config 1   │  - Clip Protection: Enabled      │
│  ▸ Prod Setup    │                                  │
│                  │ [Apply] [Export JSON]            │
│                  │                                  │
│ [New] [Delete]   │                                  │
└──────────────────┴──────────────────────────────────┘
```

#### 8.2.3 Graph Templates

Users can save entire workflows as templates for reuse.

**Template Gallery (File → New from Template):**
```
┌───────────────────────────────────────────────────────────┐
│ Workflow Templates                                        │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ 🎤          │  │ 📊          │  │ 🔊          │      │
│  │ Basic Audio │  │ Vibration   │  │ Speaker     │      │
│  │ Capture     │  │ Analysis    │  │ Test        │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│                                                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ 🤖          │  │ 📈          │  │ ➕          │      │
│  │ AI Anomaly  │  │ FFT         │  │ Blank       │      │
│  │ Detection   │  │ Waterfall   │  │ Workflow    │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│                                                           │
│  [Import Template...]  [Manage Templates...]             │
└───────────────────────────────────────────────────────────┘
```

**Template Card Details (on hover):**
- Thumbnail preview (mini graph visualization)
- Description (1-2 sentences)
- Number of nodes
- Required hardware (if any)
- Tags (e.g., "Audio", "Real-time", "Advanced")

#### 8.2.4 Bulk Parameter Editing

When multiple nodes of the same type are selected, allow editing parameters in batch.

**UI Flow:**
1. Select multiple nodes (Shift+Click or drag selection box)
2. Property Inspector shows: "3 Gain nodes selected"
3. Common parameters shown with:
   - If all values identical: Show value
   - If values differ: Show "(Mixed)"
4. Editing a parameter applies to all selected nodes

**Example:**
```
┌─────────────────────────────┐
│ 3 Gain Nodes Selected       │
├─────────────────────────────┤
│ Parameters                  │
│                             │
│  Gain (dB)                  │
│  ├─────●─────┤ [(Mixed)]    │ ← Different values
│                             │
│  Clip Protection            │
│  [✓] Enable                 │ ← Same value
│                             │
│  [Apply to All 3]           │
└─────────────────────────────┘
```

#### 8.2.5 Parameter Expressions & Linking

Advanced users can link parameters or use expressions.

**Use Case:** Make multiple filter cutoff frequencies track a single "Master Frequency" parameter.

**Implementation:**
- Right-click parameter → "Link to Variable..."
- Define global variables in "Graph Settings" dialog
- Parameter field shows: `=${master_freq} * 2` (expression)

**Variable Manager (Graph Settings):**
```
┌─────────────────────────────────────────┐
│ Graph Settings                          │
├─────────────────────────────────────────┤
│ Global Variables                        │
│                                         │
│ Name            Value      Type         │
│ master_freq     1000       Number (Hz)  │
│ gain_boost      6.0        Number (dB)  │
│ enable_debug    true       Boolean      │
│                                         │
│ [Add Variable] [Remove]                 │
├─────────────────────────────────────────┤
│ Metadata                                │
│ Title: [Production Test Workflow]      │
│ Author: [John Doe___________]           │
│ Version: [1.2.0_____]                   │
│ Description:                            │
│ [Multi-line text area........]          │
└─────────────────────────────────────────┘
```

---
```

**Verification:**
- Confirm Section 8.2 is complete
- Check UI mockups are clear

### Step 2.3: Add Hardware Configuration Interface

Continue after Section 8.2:

```markdown

### 8.3 Hardware Configuration Interface (Device Manager)

Accessed via Hardware → Device Manager, this dialog manages all physical I/O devices.

#### 8.3.1 Device Manager Dialog Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Device Manager                                                  │
├──────────────────┬──────────────────────────────────────────────┤
│ Detected Devices │ Device Configuration                         │
│                  │                                              │
│ 🎤 Audio         │ Selected: Focusrite Scarlett 18i20          │
│  ✓ Focusrite ... │                                              │
│  ○ Realtek HD    │ Status: ✓ Connected                          │
│                  │ Driver: ASIO v2.15                           │
│ 📊 DAQ Cards     │                                              │
│  ✓ NI USB-6001   │ Capabilities:                                │
│                  │  - Input Channels: 18 (analog)               │
│ 🔌 Serial        │  - Output Channels: 20 (analog)              │
│  ○ COM3          │  - Sample Rate: 44.1k - 192k Hz             │
│  ○ COM5          │  - Bit Depth: 24-bit                         │
│                  │                                              │
│ ⚡ Triggers      │ Channel Configuration:                       │
│  (none)          │  Input 1: [Mic Front Left_____] Gain: [+0dB]│
│                  │  Input 2: [Mic Front Right____] Gain: [+0dB]│
│ [Refresh]        │  ...                                         │
│                  │                                              │
│                  │ [Test Device] [Calibrate] [Advanced...]      │
└──────────────────┴──────────────────────────────────────────────┘
                   [Close]
```

**Left Panel: Device List**
- Groups devices by type (Audio, DAQ, Serial, Triggers, Custom)
- Checkmark (✓) = Enabled and ready
- Circle (○) = Detected but not enabled
- Red X (✗) = Error state (e.g., driver issue)
- [Refresh] button: Re-scan for devices

**Right Panel: Device Configuration**
- Shows detailed info for selected device
- Configuration options depend on device type

#### 8.3.2 Audio Device Configuration

For ASIO/CoreAudio/ALSA devices:

**Fields:**
- **Sample Rate**: Dropdown (44100, 48000, 96000, 192000 Hz)
- **Buffer Size**: Dropdown (64, 128, 256, 512, 1024, 2048 samples)
  - Shows latency estimate: "256 samples = 5.3ms @ 48kHz"
- **Clock Source**: (for devices with multiple clocks)
  - Internal
  - External (e.g., Word Clock, ADAT)
- **Channel Mapping**: Table view

**Channel Mapping Table:**
```
┌──────┬─────────────────────────┬──────┬─────────┐
│ Ch # │ Label                   │ Gain │ Enabled │
├──────┼─────────────────────────┼──────┼─────────┤
│ 1    │ [Mic Front Left_____]   │ [+0] │ [✓]     │
│ 2    │ [Mic Front Right____]   │ [+0] │ [✓]     │
│ 3    │ [Mic Rear__________]    │ [+0] │ [✓]     │
│ 4    │ [Unused____________]    │ [+0] │ [ ]     │
│ ...  │                         │      │         │
└──────┴─────────────────────────┴──────┴─────────┘
```

- Users can rename channels (labels saved to config)
- Enable/disable individual channels
- Adjust per-channel gain (if hardware supports)

**Buttons:**
- **[Test Device]**: Plays a test tone (sine wave) through outputs, captures on inputs, shows waveform
- **[Calibrate]**: Launches calibration wizard (see 8.3.4)
- **[Advanced...]**: Opens vendor-specific control panel (if available)

#### 8.3.3 DAQ Card Configuration

For National Instruments, Measurement Computing, or similar devices:

**Additional Fields:**
- **Voltage Range**: ±10V, ±5V, ±1V (affects resolution vs. range trade-off)
- **Input Coupling**: AC or DC
- **Terminal Configuration**: RSE (Referenced Single-Ended), NRSE, Differential
- **Trigger Source**:
  - Software (default)
  - External Digital (specify pin)
  - Analog Level (specify threshold)

**Visual Pinout Diagram:**
- Show device schematic with pin assignments
- Highlight connected/active pins in green
- Clickable pins to assign functions

#### 8.3.4 Calibration Wizard

Accessed via [Calibrate] button. Multi-step wizard for device calibration.

**Purpose:**
- Measure actual gain of signal chain (mic + preamp + ADC)
- Store calibration factors for accurate dB SPL measurements

**Steps:**

**Step 1: Setup**
```
┌─────────────────────────────────────────────┐
│ Calibration Wizard - Step 1 of 4           │
├─────────────────────────────────────────────┤
│ Select channels to calibrate:              │
│                                             │
│ [✓] Channel 1: Mic Front Left              │
│ [✓] Channel 2: Mic Front Right             │
│ [ ] Channel 3: Mic Rear                    │
│                                             │
│ Reference signal:                           │
│ ● 94 dB SPL @ 1kHz (Type 1 calibrator)     │
│ ○ 114 dB SPL @ 1kHz (Type 2 calibrator)    │
│ ○ Custom: [___] dB SPL @ [1000] Hz         │
│                                             │
│         [Cancel]  [Next >]                  │
└─────────────────────────────────────────────┘
```

**Step 2: Measurement**
```
┌─────────────────────────────────────────────┐
│ Calibration Wizard - Step 2 of 4           │
├─────────────────────────────────────────────┤
│ Instructions:                               │
│ 1. Place calibrator on microphone          │
│ 2. Turn on calibrator (94 dB @ 1kHz)       │
│ 3. Click "Measure" when ready              │
│                                             │
│ Channel 1: Mic Front Left                  │
│                                             │
│ ┌─────────────────────────────────────┐    │
│ │  Live Waveform (1 sec)              │    │
│ │  ~~~∿∿∿~~~∿∿∿~~~∿∿∿~~~∿∿∿~~~         │    │
│ └─────────────────────────────────────┘    │
│                                             │
│ Measured RMS: 0.0042 V                      │
│ Expected: 94 dB SPL                         │
│ Computed Gain: +42.3 dB                     │
│                                             │
│ Status: ✓ Calibration successful            │
│                                             │
│         [< Back]  [Next >]                  │
└─────────────────────────────────────────────┘
```

**Step 3: Verification**
- Repeat measurement, compare with stored calibration
- Show error percentage (should be <1%)

**Step 4: Save**
- Save calibration data to `~/.streamlab/calibrations/{device_id}.json`
- Associate with device serial number (survives unplugging)

#### 8.3.5 Trigger Source Configuration

For GPIO, Serial, or USB trigger devices:

**Fields:**
- **Trigger Type**:
  - Rising Edge (0→1 transition)
  - Falling Edge (1→0)
  - Level High (while signal is 1)
  - Serial Command (e.g., "TRIG\n")
- **Debounce Time**: [50] ms (ignore rapid re-triggers)
- **Pre-Trigger Buffering**: Capture [0.5] seconds before trigger

**Serial Trigger Config:**
```
Port: [COM3 ▾]
Baud Rate: [9600 ▾]
Data Bits: [8 ▾]
Parity: [None ▾]
Stop Bits: [1 ▾]
Trigger Command: [TRIG\n____________]

[Test Trigger] ← Sends command, shows if received
```

#### 8.3.6 Device Health Monitor

Accessible via Hardware → Device Health Monitor.

**Real-time Dashboard:**
```
┌─────────────────────────────────────────────────────────┐
│ Device Health Monitor                                   │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ 🎤 Focusrite Scarlett 18i20                            │
│   Status: ✓ Healthy                                     │
│   Uptime: 2h 34m                                        │
│   Buffer Underruns: 0                                   │
│   Last Error: None                                      │
│                                                         │
│   Input Levels (RMS, last 1 sec):                      │
│   Ch1: ▓▓▓▓▓▓▓░░░░░░░░░ -18 dB                          │
│   Ch2: ▓▓▓▓▓▓░░░░░░░░░░ -20 dB                          │
│   Ch3: ░░░░░░░░░░░░░░░░ -∞ dB (no signal)              │
│                                                         │
│ 📊 NI USB-6001                                          │
│   Status: ⚠️ Warning (high temperature)                 │
│   Temperature: 68°C                                     │
│   Recommended Max: 60°C                                 │
│   Action: Improve ventilation                           │
│                                                         │
│   [Export Report]  [Acknowledge Warning]                │
└─────────────────────────────────────────────────────────┘
```

**Alerts:**
- Buffer overruns/underruns (performance issue)
- Device disconnected (USB unplugged)
- Sample rate mismatch (config error)
- Clipping detected (signal too hot)

---
```

**Verification:**
- Confirm Section 8.3 is complete
- Check hardware configuration UI is detailed

### Step 2.4: Add Data Visualization & Remaining UI Sections

Continue after Section 8.3:

```markdown

### 8.4 Data Visualization Panels

Real-time and post-analysis visualization is critical. All visualizations must run at 60fps minimum.

#### 8.4.1 Visualization Node Types

Unlike processing nodes, visualization nodes don't pass data through. They consume data and display it.

**Visual Display Node (on canvas):**
```
Input ○┌────────────────────────┐
       │ 📊 Waveform Viewer     │
       ├────────────────────────┤
       │ [Embedded plot preview]│ ← Mini preview (100x60px)
       │ ≈≈∿∿∿≈≈∿∿∿≈≈           │
       ├────────────────────────┤
       │ [🗗 Open Full View]     │ ← Button to open detailed panel
       └────────────────────────┘
```

**Full Visualization Panel (Separate Window or Docked):**
- Detachable (can drag to second monitor)
- Resizable
- Multiple panels can be open simultaneously
- Synchronized playback (all plots show same time range)

#### 8.4.2 Waveform Viewer (Time Domain)

**Features:**
- X-axis: Time (seconds or samples)
- Y-axis: Amplitude (Volts, dB, normalized)
- Supports up to 32 channels simultaneously
- Auto-scaling or manual Y-axis limits
- Cursors for measurement (drag to measure time/amplitude between points)

**Implementation:**
- Library: uPlot (ultra-fast canvas-based plotting)
- Data decimation: For long recordings, show max/min envelope at zoomed-out views
- Streaming mode: Rolling buffer, shows last N seconds

**UI Controls:**
```
┌───────────────────────────────────────────────────────────┐
│ Waveform Viewer                                           │
├───────────────────────────────────────────────────────────┤
│ Channels: [☑ Ch1] [☑ Ch2] [☐ Ch3] ...                    │
│ Y-Axis: [Auto-Scale ▾] Range: [-1.0] to [+1.0]           │
│ X-Axis: [Last 10s ▾]   |◄ ► ∎| (Play/Pause/Stop)        │
├───────────────────────────────────────────────────────────┤
│                                                           │
│   +1.0├─────────────∿∿∿───────────∿∿∿─────────────┤      │
│       │          ∿∿∿   ∿∿∿     ∿∿∿   ∿∿∿           │      │
│   0.0 ├──────∿∿∿───────────∿∿∿───────────∿∿∿──────┤      │
│       │  ∿∿∿                                   ∿∿∿ │      │
│  -1.0 ├──────────────────────────────────────────┤       │
│       0s        2s        4s        6s        8s   10s   │
│                                                           │
│ Cursor A: 2.341s, +0.456V   Δ to B: 1.234s              │
└───────────────────────────────────────────────────────────┘
```

**Interaction:**
- Click and drag: Pan
- Scroll: Zoom in/out (X-axis)
- Shift+Scroll: Zoom Y-axis
- Click on plot: Place cursor A
- Shift+Click: Place cursor B (measure delta)
- Right-click: Export visible region as PNG/CSV

#### 8.4.3 Spectrum Analyzer (Frequency Domain)

Displays FFT magnitude spectrum.

**Features:**
- X-axis: Frequency (Hz, linear or logarithmic)
- Y-axis: Magnitude (dB or linear)
- Peak hold (dotted line showing historical max)
- Smoothing options (none, 1/3 octave, 1/6 octave)
- Reference line (e.g., -20 dB)

**UI:**
```
┌───────────────────────────────────────────────────────────┐
│ Spectrum Analyzer                                         │
├───────────────────────────────────────────────────────────┤
│ FFT Size: [2048 ▾]  Window: [Hanning ▾]  Overlap: [50%]  │
│ X-Axis: [☑ Log Scale]  Y-Axis: [dB ▾]                    │
├───────────────────────────────────────────────────────────┤
│                                                           │
│   0 dB├───┬───────────────────────────────────────┬───┤  │
│       │   │╱╲                                     │   │  │
│ -20 dB│   │  ╲╱╲                                  │   │  │
│       │   │     ╲╱╲                               │   │  │
│ -40 dB│   │        ╲╱╲                            │   │  │
│       │   │           ╲                           │   │  │
│ -60 dB│   │            ╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱│   │  │
│      20Hz 50  100  200  500  1k  2k  5k  10k  20kHz     │
│                                                           │
│ Peak: 1.234 kHz @ -12.3 dB                               │
└───────────────────────────────────────────────────────────┘
```

**Advanced Features:**
- Harmonic markers (if fundamental detected, show 2f, 3f, ...)
- THD calculation and display
- Save reference spectrum (for A/B comparison)

#### 8.4.4 Spectrogram / Waterfall Display

2D time-frequency representation.

**Features:**
- X-axis: Time
- Y-axis: Frequency
- Color: Magnitude (heat map: blue=low, red=high)
- Scroll mode: Waterfall (time flows down)

**UI:**
```
┌───────────────────────────────────────────────────────────┐
│ Spectrogram                                               │
├───────────────────────────────────────────────────────────┤
│ Color Map: [Viridis ▾]  Range: [-60] to [0] dB           │
├───────────────────────────────────────────────────────────┤
│ 20kHz├──────────────────────────────────────────────┤     │
│      │▓░░░░░▓▓▓░░░░░░░▓▓▓░░░░░░░░▓░░░░░░░░░░░░░░░░│     │
│      │▓▓░░░▓▓▓▓▓░░░░░▓▓▓▓▓░░░░░░▓▓░░░░░░░░░░░░░░░│     │
│ 10kHz│▓▓▓░▓▓▓▓▓▓░░░░▓▓▓▓▓▓░░░░░▓▓▓░░░░░░░░░░░░░░░│     │
│      │▓▓▓▓▓▓▓▓▓▓▓░░▓▓▓▓▓▓▓░░░░▓▓▓▓░░░░░░░░░░░░░░░│     │
│  5kHz│▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░▓▓▓▓▓░░░░░░░░░░░░░░░│     │
│      │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░▓▓▓▓▓▓░░░░░░░░░░░░░░░│     │
│  1kHz│▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░│     │
│      │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░│     │
│   0Hz└──────────────────────────────────────────────┘     │
│      0s                                           10s     │
│                                                           │
│ Color Scale: [-60 dB] ░▒▓█ [0 dB]                        │
└───────────────────────────────────────────────────────────┘
```

**Use Cases:**
- Voice activity detection (see speech formants)
- Rotating machinery analysis (see harmonics evolve)
- Music visualization

#### 8.4.5 Multi-Channel Synchronized View

When analyzing microphone arrays or multi-point vibration, need synchronized multi-channel display.

**Layout:**
```
┌───────────────────────────────────────────────────────────┐
│ Multi-Channel View (Synchronized)                         │
├───────────────────────────────────────────────────────────┤
│ Channel 1: Front Left                                     │
│ ∿∿∿───────────∿∿∿───────────∿∿∿─────────                  │
├───────────────────────────────────────────────────────────┤
│ Channel 2: Front Right                                    │
│ ──∿∿∿───────────∿∿∿───────────∿∿∿───────                  │
├───────────────────────────────────────────────────────────┤
│ Channel 3: Rear                                           │
│ ─────∿∿∿───────────∿∿∿───────────∿∿∿─────                 │
├───────────────────────────────────────────────────────────┤
│ ◄──────────  Shared Time Axis  ──────────►               │
│ 0s        2s        4s        6s        8s        10s     │
└───────────────────────────────────────────────────────────┘
```

**Features:**
- Shared time axis (zooming/panning affects all)
- Individual Y-axis scaling (each channel can have different amplitude range)
- Phase difference visualization (shift one channel to measure delay)
- Cross-correlation display

#### 8.4.6 Export & Annotation Tools

All visualization panels have:

**Export Options (Right-click or toolbar button):**
- Export as PNG (raster image, current view)
- Export as SVG (vector, for publications)
- Export data as CSV (underlying data points)
- Export as PDF report (includes plot + metadata)

**Annotation Tools:**
- Text labels (click to add, drag to position)
- Arrows (point to features of interest)
- Regions (highlight time/frequency ranges)
- Saved with workflow, displayed on plot

**Example:**
```
User adds annotation: "Anomaly detected at 3.2s, -15dB spike"
→ Saved to graph JSON under annotations array
→ Displayed on plot with draggable text box
```

### 8.5 State Management & Data Flow

The frontend manages complex state. Clean architecture is essential.

#### 8.5.1 State Management Architecture

**Technology:** Recoil (Facebook's state management library)

**Atom Structure:**
```typescript
// Graph Definition (the workflow)
export const graphAtom = atom<Graph>({
  key: 'graph',
  default: { nodes: [], edges: [] },
});

// Selected nodes (for property inspector)
export const selectedNodesAtom = atom<string[]>({
  key: 'selectedNodes',
  default: [],
});

// Pipeline runtime state
export const pipelineStateAtom = atom<PipelineState>({
  key: 'pipelineState',
  default: 'Idle',
});

// Real-time visualization data
export const vizDataAtom = atomFamily<VizData, string>({
  key: 'vizData',
  default: null,
});

// UI preferences (sidebar widths, theme, etc.)
export const uiPreferencesAtom = atom<UIPreferences>({
  key: 'uiPreferences',
  default: defaultPreferences,
  effects_UNSTABLE: [localStorageEffect('ui-prefs')], // Persist to localStorage
});
```

**Why Recoil?**
- Minimal boilerplate
- Atom-based (fine-grained reactivity, less re-rendering)
- Built-in async support (for Tauri calls)
- DevTools for debugging

#### 8.5.2 Frontend-Backend Communication

**Tauri Commands (Frontend → Backend):**
```typescript
// Deploy a graph (start execution)
await invoke('deploy_graph', { graphJson: JSON.stringify(graph) });

// Pause/Resume pipeline
await invoke('pause_pipeline', { pipelineId: 'main' });
await invoke('resume_pipeline', { pipelineId: 'main' });

// Query node registry (get available node types)
const nodeTypes = await invoke('get_node_types');

// Hardware operations
const devices = await invoke('list_devices');
await invoke('calibrate_device', { deviceId: 'focusrite_001' });
```

**Tauri Events (Backend → Frontend):**
```typescript
// Listen for pipeline state changes
listen('pipeline-state-changed', (event) => {
  const { pipelineId, newState } = event.payload;
  setRecoilState(pipelineStateAtom, newState);
});

// Listen for real-time data (for visualization)
listen('viz-data', (event) => {
  const { nodeId, data } = event.payload;
  setRecoilState(vizDataAtom(nodeId), data);
});

// Listen for errors
listen('error', (event) => {
  const { message, severity } = event.payload;
  showNotification(message, severity);
});
```

**High-Volume Data (Shared Memory Approach):**
- For data too large for Tauri events (e.g., 192kHz 64-channel audio)
- Backend writes to memory-mapped file: `/tmp/streamlab_shm_viz_data`
- Frontend WebAssembly module reads from same file
- Polling: Frontend checks every 16ms (60fps)

```typescript
// frontend/src/utils/sharedMemoryReader.ts
export class SharedMemoryReader {
  private sab: SharedArrayBuffer;

  async init() {
    // Tauri provides path to shared memory region
    const shmPath = await invoke('get_shm_path');

    // Use WebAssembly to map file (not directly possible in JS)
    this.sab = await wasmModule.mapSharedMemory(shmPath);
  }

  readLatestFrame(): Float64Array {
    // Read from shared buffer (lock-free ring buffer protocol)
    const header = new Uint32Array(this.sab, 0, 4);
    const writeIndex = Atomics.load(header, 0);
    const frameSize = Atomics.load(header, 1);

    const dataOffset = 16 + (writeIndex % RING_SIZE) * frameSize;
    return new Float64Array(this.sab, dataOffset, frameSize);
  }
}
```

#### 8.5.3 Optimistic Updates & Conflict Resolution

When user edits a parameter while pipeline is running:

**Flow:**
1. User changes Gain from 6dB to 12dB
2. Frontend immediately updates local state (optimistic)
3. Frontend sends `update_node_param` command to backend
4. Backend applies change
5. Backend sends confirmation event
6. If backend rejects (e.g., invalid value), frontend reverts

**Implementation:**
```typescript
const updateNodeParam = async (nodeId: string, paramName: string, value: any) => {
  const oldValue = getNodeParam(nodeId, paramName);

  // Optimistic update
  setNodeParam(nodeId, paramName, value);

  try {
    await invoke('update_node_param', { nodeId, paramName, value });
  } catch (error) {
    // Revert on failure
    setNodeParam(nodeId, paramName, oldValue);
    showError(`Failed to update ${paramName}: ${error}`);
  }
};
```

### 8.6 Accessibility & Keyboard Navigation

Ensure power users can work efficiently with keyboard only.

#### 8.6.1 Global Keyboard Shortcuts

| Shortcut         | Action                          |
|------------------|---------------------------------|
| Ctrl+N           | New Workflow                    |
| Ctrl+O           | Open Workflow                   |
| Ctrl+S           | Save Workflow                   |
| Ctrl+Shift+S     | Save As                         |
| Ctrl+Z           | Undo                            |
| Ctrl+Y           | Redo                            |
| Ctrl+X           | Cut selected nodes              |
| Ctrl+C           | Copy selected nodes             |
| Ctrl+V           | Paste nodes                     |
| Del              | Delete selected nodes           |
| Ctrl+A           | Select all nodes                |
| Ctrl+F           | Quick node search palette       |
| Ctrl+R           | Run pipeline                    |
| Ctrl+P           | Pause pipeline                  |
| Ctrl+.           | Stop pipeline                   |
| F5               | Refresh device list             |
| F11              | Toggle fullscreen               |

#### 8.6.2 Canvas Keyboard Navigation

- **Arrow Keys**: Pan canvas (hold Shift for faster)
- **+/-**: Zoom in/out
- **Tab**: Cycle through nodes (shows focus ring)
- **Enter** (on focused node): Open parameter editor
- **Space + Drag**: Pan canvas (alternative to middle-mouse)

#### 8.6.3 Accessibility Features

- **ARIA Labels**: All interactive elements have `aria-label` for screen readers
- **Focus Indicators**: Clear blue outline (3px) on focused elements
- **High Contrast Mode**: Option in View menu (white on black, no gradients)
- **Keyboard-Only Mode**: Disable all hover effects (for keyboard users)

---
```

**Verification:**
- Confirm Section 8 is complete with subsections 8.1-8.6
- Check all UI specifications are detailed
- Verify code examples are present

### Step 2.5: Add Final Review and Metadata

Add a brief conclusion to Section 8:

```markdown

### 8.7 Design System & UI Consistency

To ensure a cohesive look and feel across all interfaces:

**Color Palette (Dark Theme - Default):**
- Background: `#1e1e1e` (VSCode dark)
- Surface: `#252526`
- Border: `#3c3c3c`
- Text Primary: `#cccccc`
- Text Secondary: `#808080`
- Accent Blue: `#0078d4` (buttons, links)
- Success Green: `#4ec9b0`
- Warning Yellow: `#dcdcaa`
- Error Red: `#f48771`

**Typography:**
- UI Font: Inter (sans-serif, system fallback)
- Code Font: Fira Code / JetBrains Mono (monospace)
- Sizes:
  - Body: 13px
  - Small: 11px
  - Large: 15px
  - Title: 18px

**Component Library:**
- Use Radix UI primitives (headless, accessible)
- Custom styling with Tailwind CSS
- Shadcn/ui component patterns

**Icons:**
- Lucide React (consistent icon set)
- 20px standard size
- Stroke width: 2px

**Spacing Scale:**
- Base unit: 4px
- Spacing: 4px, 8px, 12px, 16px, 24px, 32px, 48px

**Animation:**
- Transitions: 150ms ease-in-out (standard)
- Micro-interactions: 100ms (button hover)
- Page transitions: 300ms
- Reduced motion: Respect `prefers-reduced-motion` media query

**Responsive Design:**
- Minimum resolution: 1280x720
- Recommended: 1920x1080
- Sidebars collapse on smaller screens (<1600px width)

---

**Implementation Notes:**
- All interface designs should be implemented using React 19 features (Server Components where applicable, though Tauri is client-side)
- Use TypeScript for type safety
- Follow React best practices (hooks, functional components, memoization)
- Ensure all components are unit tested (Vitest + React Testing Library)

---
```

**Verification:**
- Section 8 is complete
- Design system guidelines provided

---

## Task 3: Final Review and Validation

### Step 3.1: Read the Updated File

After adding Sections 7 and 8, read the entire project.md file to ensure:
- Sections are numbered correctly (1-8)
- No formatting errors
- Code blocks are properly closed
- Consistency in terminology
- All cross-references are valid

### Step 3.2: Create a Summary of Additions

Create a summary document listing what was added:

**File:** `/Users/fh/Code/audiotab/docs/project-completion-summary.md`

```markdown
# Project.md Completion Summary

**Date:** 2025-11-24

## Additions Made

### Section 7: Kernel Architecture & Advanced Improvements

Added comprehensive kernel-level documentation covering:
- Enhanced pipeline execution model (state machines, priority scheduling, resource pooling, checkpointing)
- Zero-copy data architecture (Apache Arrow integration, memory mapping, SIMD optimization)
- Advanced scheduling and orchestration (CPU affinity, dynamic recompilation, distributed execution prep)
- Error handling and observability (hierarchical error propagation, circuit breaker pattern, metrics)
- Memory management and leak prevention (frame lifecycle tracking, backpressure, memory pooling)
- Testing and validation infrastructure (synthetic generators, integration tests, stress/soak testing)

**Total Lines Added:** ~400 lines
**Code Examples:** 15+ Rust implementations

### Section 8: Interface Design Specifications

Added detailed UI/UX specifications covering:
- Main interface layout (workbench view with menu bar, node palette, canvas, property inspector, console, status bar)
- Analysis configuration interface (quick parameters, presets, templates, bulk editing, parameter expressions)
- Hardware configuration interface (device manager, audio config, DAQ config, calibration wizard, trigger sources, health monitor)
- Data visualization panels (waveform viewer, spectrum analyzer, spectrogram, multi-channel sync view, export tools)
- State management architecture (Recoil atoms, Tauri communication, shared memory for high-volume data)
- Accessibility and keyboard navigation (shortcuts, focus management, screen reader support)
- Design system guidelines (color palette, typography, components, spacing, animations)

**Total Lines Added:** ~600 lines
**UI Mockups:** 20+ ASCII diagrams
**Code Examples:** 10+ TypeScript/React implementations

## Impact

The proposal is now complete with production-grade specifications for both backend kernel improvements and comprehensive frontend interface designs. Developers can use these sections to:
1. Understand advanced kernel features needed for production deployment
2. Implement consistent, accessible, and efficient user interfaces
3. Build a cohesive system that balances power-user functionality with approachability

## Next Steps

1. Review the updated project.md with stakeholders
2. Use these specifications to create detailed implementation plans for Phases 1-5
3. Begin development following the roadmap
```

### Step 3.3: Verification Checklist

Before marking the task complete, verify:

- [ ] Section 7 is added after Section 6
- [ ] Section 8 is added after Section 7
- [ ] All subsections are numbered correctly (7.1-7.6, 8.1-8.7)
- [ ] Code blocks use proper markdown syntax (triple backticks with language)
- [ ] ASCII diagrams are properly formatted
- [ ] No broken internal references
- [ ] File compiles as valid markdown (no syntax errors)
- [ ] Terminology is consistent with existing sections (DataFrame, Pipeline, Node, etc.)
- [ ] Summary document is created

---

## Success Criteria

The plan is complete when:
1. `/Users/fh/Code/audiotab/project.md` contains Sections 7 and 8 with all specified content
2. No markdown formatting errors exist
3. All code examples are syntactically valid (at minimum, structurally correct)
4. Summary document is created documenting changes
5. Total document is cohesive and ready for developer consumption

---

## Estimated Time

- Task 1 (Section 7): 45 minutes (writing + code examples)
- Task 2 (Section 8): 60 minutes (UI specifications + diagrams)
- Task 3 (Review): 15 minutes

**Total: ~2 hours**

---

## Notes

- Maintain the same writing style and technical depth as existing sections
- Use concrete code examples wherever possible (Rust for kernel, TypeScript/React for UI)
- Ensure all UI mockups are clear even in ASCII format (use box-drawing characters)
- Cross-reference between sections where appropriate (e.g., Section 8.2 refers to Section 7.1 pipeline states)
