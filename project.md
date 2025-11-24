This is a detailed implementation plan regarding the construction of a **Next-Generation Streaming Multi-Physics Analysis & Test Framework**.

---

# Project Plan: StreamLab Core (Code Name)

## 1. Project Overview & Goal Definition

### 1.1 Project Background & Usage
This project aims to develop a high-performance, highly scalable desktop software framework for the acquisition, streaming analysis, and automated testing of acoustics, vibration, voltage, temperature, and other time-series data. The framework addresses the inflexibility caused by "hard-coding" in traditional test software, empowering end-users to freely construct complex analysis workflows via **Visual Flow-Based Programming**.

**Core Value Propositions:**
*   **For Users**: Define complex test tasks like drawing a flowchart by dragging and connecting nodes such as "Acquisition," "Filter," "FFT," "AI Inference," and "Judgment."
*   **For Developers**: Provide high-performance Operator development interfaces in Rust and rapid prototyping interfaces in Python. Develop once, and automatically register to the frontend component library.
*   **For Scenarios**: Supports not only offline analysis in laboratories but focuses significantly on real-time (Streaming) automated testing, equipment calibration, and fault monitoring on production lines.

### 1.2 Core Feature Specs
1.  **Visual Orchestration System**: An infinite canvas based on React Flow, supporting drag-and-drop, connection, and parameter configuration for 100+ operator nodes. Supports Branching and Looping/Retrying.
2.  **Multi-modal Triggering & Concurrent Kernel**:
    *   Supports various trigger sources (UI Buttons, TCP Commands, USB Signals, GPIO Levels).
    *   **Pipeline Concurrency Mechanism**: When a previous signal processing is not yet complete (e.g., slow AI inference), a newly arrived trigger signal should immediately start a new Pipeline instance (based on Rust's Async/Tokio Task mechanism) without blocking each other.
3.  **Hardware Abstraction Layer (HAL)**: Shields underlying hardware differences (ASIO sound cards, NI DAQ cards, PLCs, Serial devices), abstracting them uniformly as `SourceNode` (Streaming Input Source) and `SinkNode` (Excitation Output Source). Supports multi-channel (e.g., 64-channel microphone arrays) and multi-modal (simultaneous voltage and vibration collection) synchronization.
4.  **Hybrid Runtime**:
    *   **Rust Core**: Ensures nanosecond-level latency for intensive calculations like FFT, STFT, and filtering.
    *   **Python Sandbox**: Allows users or developers to write Python scripts to run directly as nodes, achieving zero-copy data transmission via shared memory, supporting PyTorch/TensorFlow model calls.
5.  **Implicit Analysis Workflows**: In addition to the visible analysis on the main interface, supports "background implicit workflows." For example, an "Equipment Calibration" function is essentially invoking a pre-defined "Play-Acquire-Calculate Gain" analysis graph, where the user simply clicks the "Calibrate" button.
6.  **Real-time Visualization**: Supports high frame rate (60fps+) display of streaming waveforms, spectrograms, and waterfall plots.

### 1.3 Tech Stack Standards
*   **Core Language**: Rust (2024 Edition) - Emphasizing async runtime (`tokio`), zero-copy (`bytes`, `arrow-rs`), and dynamic dispatch.
*   **Scripting**: Python 3.13 - Utilizing the latest JIT features and `PyO3` bindings.
*   **Frontend**: React 19 + React Flow + Recoil/Zustand (State Management) + WebGL Plotting (uPlot/PixiJS).
*   **App Shell**: Tauri v2 - Ensuring a small binary size and native system capability access.
*   **Communication**: gRPC (Internal) or Shared Memory (System-wide) + Tauri Events.

---

## 2. Architecture Breakdown

This section is for reference by architects and senior developers to define the system skeleton.

### 2.1 Domain Model & Data Flow
*   **The Graph**: Represents a complete analysis logic.
*   **The Pipeline**: A **runtime instantiation** of the Graph.
*   **Frame (Data Frame)**: The basic unit passed within the system.

```rust
struct DataFrame {
    timestamp: u64,
    sequence_id: u64,
    payload: HashMap<String, Arc<DataTensor>>, // Supports reference counting sharing for multi-channel data
    metadata: HashMap<String, String>, // Passing side-channel information (e.g., Gain settings)
}
```

### 2.2 Module Division
1.  **Front-End (GUI)**: Responsible for describing the Graph via JSON and rendering real-time data.
2.  **Orchestrator (Rust)**: The coordinator. Parses JSON Graphs and manages the lifecycle of Pipelines (creation, destruction, concurrency pool management).
3.  **Node Registry (Rust)**: The plugin system. Scans and registers all available operators at program startup.
4.  **HAL (Hardware Abstraction Layer)**: The driver adaptation layer.

---

## 3. Implementation Roadmap

This section can be directly assigned to developers for execution.

### Phase 1: Core Engine - Estimated Time: 4 Weeks

**Goal**: Implement the Rust backend capable of parsing JSON configuration and running a simple flow: "Sine Wave Gen -> Gain -> Print".

#### Task 1.1: Define Core Trait System (Rust)
*   **File**: `src/core/node.rs`
*   **Description**: Define the base class for all nodes.
*   **Code Standard**:
    ```rust
    #[async_trait]
    pub trait ProcessingNode: Send + Sync {
        async fn on_create(&mut self, config: serde_json::Value) -> Result<()>;
        async fn process(&self, input: DataFrame) -> Result<DataFrame>; 
        // Note: This handles not only data flow but also control flow signals
    }
    ```

#### Task 1.2: Implement Actor Scheduling Model (Rust)
*   **File**: `src/engine/executor.rs`
*   **Description**:
    1.  Implement a `PipelineBuilder` that takes a JSON graph structure as input and outputs a series of `Task`s connected via `tokio::mpsc::channel`.
    2.  Implement **Backpressure** mechanism: When downstream processing is slow, upstream should be blocked or packet-dropped (configurable).
*   **Concurrency Requirement**: Must support running multiple Pipeline instances simultaneously. Design a `PipelinePool`; when a trigger signal arrives, `tokio::spawn` a brand new execution instance of the graph.

#### Task 1.3: Implement Basic HAL Interfaces
*   **File**: `src/hal/mod.rs`
*   **Description**: Define `DeviceSource` and `DeviceSink` traits.
*   **Mock Implementation**: Write a "Simulated Audio Source" that generates a 1024-point sine wave data packet every 10ms.

---

### Phase 2: Frontend & Builder - Estimated Time: 3 Weeks

**Goal**: Users can drag and drop to generate JSON on the interface and control the startup of the backend engine.

#### Task 2.1: React Flow Integration & Custom Nodes
*   **Path**: `src-frontend/components/FlowEditor`
*   **Description**:
    1.  Encapsulate a `BaseNode` component, including input/output anchors.
    2.  Implement dynamic node loading: The frontend does not hard-code what an "FFT Node" looks like. Instead, it requests `GET /api/nodes` from the backend to obtain metadata (input port count, parameter Schema) and renders the node UI dynamically.

#### Task 2.2: State Synchronization Mechanism (Tauri Bridge)
*   **Tech**: Tauri Commands + Events
*   **Description**:
    1.  Frontend -> Backend: `invoke('deploy_graph', { json })`
    2.  Backend -> Frontend: High-volume data is not transmitted here; only status is transmitted (e.g., Pipeline 1 started, Pipeline 1 running, Pipeline 1 error).

---

### Phase 3: Hybrid Runtime & Plugin System (Python Integration) - Estimated Time: 3 Weeks

**Goal**: Support Python scripts as nodes and support complex mathematical analysis.

#### Task 3.1: PyO3 Bridge
*   **File**: `src/nodes/python_bridge.rs`
*   **Description**:
    1.  Initialize a global Python interpreter instance (or a sub-interpreter per Pipeline, depending on isolation needs; Global + Lock recommended for 3.13 no-GIL experiments).
    2.  Implement `PythonNode`, which in the `process` method:
        *   Gets the memory pointer of the Rust `DataFrame`.
        *   Creates a Python `memoryview` / `numpy.array` via unsafe means (avoiding copy).
        *   Calls the user-specified `.py` function.
        *   Converts the return result back to a Rust structure.

#### Task 3.2: Operator Registry Auto-Discovery
*   **Description**: Use Rust macros `#[derive(Node)]` and the `inventory` crate (or similar mechanism). Developers still write Rust structs, but they are automatically registered to the system at compile time, making them visible when queried by the frontend.

---

### Phase 4: Streaming & Visualization (Streaming & Viz) - Estimated Time: 3 Weeks

**Goal**: Make waveforms animate and support high-performance display.

#### Task 4.1: Shared Memory Ring Buffer
*   **Pain Point**: No matter how fast Tauri events are, sending large `f64` arrays via IPC will cause stuttering.
*   **Solution**:
    1.  The Rust backend maintains a fixed-size Ring Buffer (storing the latest N seconds of data).
    2.  The frontend uses WebAssembly (WASM) or a simple timer to fetch **downsampled** data for plotting.
    3.  *Advanced Solution*: If on the same machine, explore Rust writing directly to `SharedArrayBuffer` and frontend JS reading directly (requires Tauri security policy permission). *Fallback Solution*: Binary WebSocket transmission.

#### Task 4.2: WebGL Plotting Component
*   **Tech**: Integrate `uPlot` or write a simple WebGL renderer.
*   **Requirement**: Support updating 4096 points per frame without dropping frames.

---

### Phase 5: Logic Control & Advanced Features (Logic & HAL) - Estimated Time: 3 Weeks

#### Task 5.1: Implement Logic Control Nodes
*   **If/Else Node**: Routes data packets to Output A or Output B based on data characteristics (e.g., RMS > Threshold). This requires the `executor` to support dynamic routing.
*   **Loop Control**: Implement "Run N times" or "Stop when condition met" global controllers.

#### Task 5.2: Real Hardware Integration
*   **Task**: Implement the Source Node for `CPAL` (Cross-Platform Audio Library).
*   **Task**: Implement a Trigger Node for Serial/VISA protocols.

---

## 4. Developer API Reference

### 4.1 How to add a new Rust Analysis Operator?
Developers simply need to create a new file under `src/nodes/` and implement the trait; macros handle UI registration automatically.

```rust
// src/nodes/my_filter.rs

#[derive(StreamNode, Serialize, Deserialize)] 
#[node_meta(name = "My HighPass", category = "Filters", flexible_input = false)]
pub struct HighPassNode {
    // Define parameters configurable on the frontend
    #[param(default = 100.0, min = 20.0, max = 20000.0)]
    cutoff_freq: f64,

    // Internal state (not exposed to configuration)
    #[serde(skip)]
    internal_state: Vec<f64>, 
}

#[async_trait]
impl ProcessingNode for HighPassNode {
    async fn process(&mut self, mut frame: DataFrame) -> Result<DataFrame> {
        // 1. Get input data
        let data = frame.payload.get("main_channel").unwrap();
    
        // 2. Algorithm processing
        let result = my_dsp_algo(data, self.cutoff_freq);
    
        // 3. Modify Frame and pass downstream
        frame.payload.insert("main_channel".to_string(), Arc::new(result));
        Ok(frame)
    }
}
```

### 4.2 How to define triggers?
A trigger is essentially a special `SourceNode` that does not produce Data Frames, but only produces `SignalFrame` (Empty Payload, containing only Trigger ID and Timestamp).
The system should maintain built-in `TcpTriggerNode`, `UsbTriggerNode`.

---

## 5. Risk Management

1.  **Python Global Interpreter Lock (GIL)**:
    *   *Risk*: Computation in Python nodes freezing the entire Rust thread pool.
    *   *Mitigation*: Use Rust 2024 and Python 3.13 (No-GIL build); or run Python in a separate thread/process, communicating via IPC. Initially, mandate that Python nodes should strictly act as lightweight glue logic or one-off inference, avoiding intensive loops.
2.  **Memory Leak**:
    *   *Risk*: Long-running streaming analysis where `Arc` reference counts are not released.
    *   *Mitigation*: Strict Frame lifecycle management. Force `Drop` of all resources at the end of a Pipeline. Write stress test scripts to run continuously for 24 hours to monitor memory.
3.  **Tauri Frontend Performance**:
    *   *Risk*: Too many React Flow nodes causing rendering lag.
    *   *Mitigation*: When nodes exceed 100, enable React Flow's `onlyRenderVisible` option. Separate the real-time plotting layer from the UI layer.

## 6. Definition of Done
1.  **Functional Acceptance**: User drags "Mic Input" -> "STFT" -> "Heatmap Display", clicks run, speaks into the microphone, and a voiceprint appears on the screen in real-time.
2.  **Concurrency Acceptance**: Set an analysis workflow taking $1$ second. Trigger it continuously $10$ times with $0.5$ second intervals; the system should automatically spawn $10$ processing tasks in parallel, returning correct results without blocking backlog.
3.  **Extensibility Acceptance**: Successfully load an external Python script as a new node and run it without recompiling the main program.

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

### 8.2 Analysis Configuration, Hardware Management & Visualization

Due to the extensive scope of the remaining interface specifications, the following subsections are summarized. Full detailed specifications for implementation should reference the complete design plan document at `/Users/fh/Code/audiotab/docs/plans/2025-11-24-complete-project-proposal-plan.md`.

#### 8.2.1 Analysis Configuration Interface

**Key Features:**
- Quick parameter panel with inline editing (double-click nodes)
- Parameter preset system (built-in + user-defined)
- Graph templates gallery for reusable workflows
- Bulk parameter editing for multiple selected nodes
- Parameter expressions & linking with global variables
- Variable manager in Graph Settings dialog

#### 8.2.2 Hardware Configuration Interface (Device Manager)

**Components:**
- Device manager dialog with detected devices list
- Audio device configuration (ASIO/CoreAudio/ALSA)
  - Sample rate, buffer size, clock source
  - Channel mapping table with labels and gain
- DAQ card configuration (NI, Measurement Computing)
  - Voltage range, input coupling, terminal configuration
  - Visual pinout diagram
- Multi-step calibration wizard (setup, measurement, verification, save)
- Trigger source configuration (GPIO, Serial, USB)
- Device health monitor dashboard with real-time status

#### 8.2.3 Data Visualization Panels

**Visualization Types:**
- Waveform Viewer (time domain)
  - Multi-channel support, auto-scaling, cursors for measurement
  - Library: uPlot for 60fps+ performance
- Spectrum Analyzer (frequency domain)
  - FFT magnitude spectrum, peak hold, smoothing options
  - Harmonic markers and THD calculation
- Spectrogram / Waterfall Display
  - 2D time-frequency heat maps
  - Configurable color maps (Viridis, etc.)
- Multi-Channel Synchronized View
  - Shared time axis, individual Y-axis scaling
  - Phase difference visualization
- Export & Annotation Tools
  - Export as PNG/SVG/CSV/PDF
  - Text labels, arrows, region highlighting

#### 8.2.4 State Management Architecture

**Technology Stack:**
- Recoil for state management (atoms, selectors)
- Tauri Commands for frontend→backend communication
- Tauri Events for backend→frontend updates
- Shared Memory approach for high-volume data (192kHz 64-channel)
- Optimistic updates with conflict resolution

#### 8.2.5 Accessibility & Keyboard Navigation

**Features:**
- Comprehensive keyboard shortcuts (Ctrl+N, Ctrl+R, etc.)
- Canvas keyboard navigation (arrows for panning, Tab to cycle nodes)
- ARIA labels for screen readers
- Focus indicators (3px blue outline)
- High contrast mode option
- Keyboard-only mode

#### 8.2.6 Design System & UI Consistency

**Color Palette (Dark Theme):**
- Background: #1e1e1e, Surface: #252526, Border: #3c3c3c
- Text Primary: #cccccc, Text Secondary: #808080
- Accent Blue: #0078d4, Success: #4ec9b0, Warning: #dcdcaa, Error: #f48771

**Typography:**
- UI Font: Inter (13px body, 11px small, 18px title)
- Code Font: Fira Code / JetBrains Mono

**Component Library:**
- Radix UI primitives with Tailwind CSS
- Lucide React icons (20px, 2px stroke)
- 4px base spacing unit

**Animation:**
- Standard transitions: 150ms ease-in-out
- Respects `prefers-reduced-motion` media query

**Responsive Design:**
- Minimum: 1280x720, Recommended: 1920x1080
- Sidebars collapse on screens <1600px

---

**Implementation Notes:**
- Use React 19 with TypeScript for type safety
- Follow React best practices (hooks, functional components, memoization)
- All components unit tested (Vitest + React Testing Library)
- Refer to detailed plan document for complete UI mockups and specifications

---

## Conclusion

This project proposal now contains comprehensive specifications for both the kernel architecture (Section 7) and interface design (Section 8), completing the original Phases 1-6 outlined in the roadmap. Developers can use these specifications to build a production-grade streaming multi-physics analysis framework with:

1. **Advanced Kernel Features**: Zero-copy data architecture, priority scheduling, resource pooling, circuit breakers, observability
2. **Comprehensive UI/UX**: Flow-based visual programming, hardware management, real-time visualization, accessibility
3. **Production Readiness**: Memory leak prevention, stress testing infrastructure, calibration workflows, health monitoring

The system balances power-user functionality with approachability, supporting both laboratory offline analysis and real-time production-line automated testing.
