# StreamLab Architecture

## Data Flow

### Phase 1: Linear Execution
```
JSON Config → Pipeline Builder → Node Graph → Sequential Execution
```

### Phase 2: Async Streaming
```
JSON Config → AsyncPipeline Builder → Node Graph
     ↓
Tokio Tasks (concurrent) + MPSC Channels (data flow)
     ↓
Multiple Instances via PipelinePool (semaphore-controlled)
```

## Core Abstractions

### DataFrame
- timestamp: μs since epoch
- sequence_id: frame ordering
- payload: HashMap<String, Vec<f64>> - multi-channel data
- metadata: HashMap<String, String> - side-channel info

### ProcessingNode Trait

**Phase 1 (Single-shot):**
```rust
async fn on_create(&mut self, config: Value) -> Result<()>
async fn process(&self, input: DataFrame) -> Result<DataFrame>
```

**Phase 2 (Streaming):**
```rust
async fn on_create(&mut self, config: Value) -> Result<()>
async fn run(
    &self,
    rx: mpsc::Receiver<DataFrame>,
    tx: mpsc::Sender<DataFrame>,
) -> Result<()>
```

Nodes can implement either or both methods. The `run()` method enables continuous streaming processing where the node:
- Receives frames from `rx` in a loop
- Processes each frame
- Sends results to `tx`
- Runs until `rx` is closed (when upstream nodes finish)

### Pipeline (Phase 1)
- Parses JSON graph definition
- Instantiates nodes with configuration
- Executes nodes sequentially in topological order
- Uses `process()` method for single-shot execution

### AsyncPipeline (Phase 2)
- Parses JSON graph definition with optional `pipeline_config.channel_capacity`
- Instantiates nodes with configuration
- Spawns each node as independent tokio task
- Creates MPSC channels for inter-node communication
- Implements fanout pattern for nodes with multiple outputs
- Uses `run()` method for streaming execution
- Supports triggering via source node channel
- Graceful shutdown by dropping channels and awaiting tasks

**Key Implementation Details:**
- Each node runs in `tokio::spawn()` task
- Bounded MPSC channels provide backpressure
- Source node (no incoming connections) receives trigger frames
- Fanout task duplicates frames for multiple downstream nodes
- Channel drops signal shutdown to all nodes

### PipelinePool (Phase 2)
- Manages multiple concurrent pipeline instances
- Uses `Arc<Semaphore>` for admission control
- Each `execute()` call:
  1. Acquires semaphore permit (blocks if at max_concurrent)
  2. Spawns new AsyncPipeline instance
  3. Runs pipeline with trigger frame
  4. Releases permit when done (allows next instance)
- Returns `JoinHandle` for awaiting completion
- Validates config once at pool creation

## Phase 1 Limitations (Addressed in Phase 2)

- ~~Linear execution only (no parallelism)~~ → **Concurrent node execution**
- ~~Synchronous node processing (blocking)~~ → **Async non-blocking with tokio**
- ~~No backpressure~~ → **Bounded channels with configurable capacity**
- ~~No concurrent pipeline instances~~ → **PipelinePool with semaphore control**

## Phase 2 Improvements

**Concurrency:**
- Each node runs in separate tokio task
- Nodes process frames independently and simultaneously
- True pipeline parallelism (not just async coordination)

**Backpressure:**
- Bounded MPSC channels (`channel_capacity` config)
- Slow downstream nodes naturally throttle upstream
- Prevents unbounded memory growth

**Scalability:**
- PipelinePool enables multiple pipeline instances
- Semaphore prevents resource exhaustion
- Configurable concurrency limit

**Phase Continuity:**
- Streaming nodes maintain state across frames
- Example: SineGenerator phase tracks across multiple triggers
- Enables continuous signal generation without discontinuities

## Phase 2 Limitations

- No dynamic pipeline reconfiguration (must stop and rebuild)
- No zero-copy buffer management (frames are cloned for fanout)
- No real-time scheduling guarantees (relies on tokio scheduler)
- No built-in error recovery (node errors propagate and stop pipeline)
- Fixed topology (connections set at creation time)

These will be addressed in future phases.
