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
