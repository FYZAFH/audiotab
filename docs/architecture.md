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

## Phase 2 Limitations (Addressed in Phase 3)

- ~~No built-in error recovery (node errors propagate and stop pipeline)~~ → **ResilientNode with ErrorPolicy**
- ~~No zero-copy buffer management (frames are cloned for fanout)~~ → **Arc<Vec<f64>> for reference-counted sharing**
- No dynamic pipeline reconfiguration (must stop and rebuild)
- No real-time scheduling guarantees (relies on tokio scheduler)
- Fixed topology (connections set at creation time)

## Phase 3: Production Readiness

### Observability Architecture

```
NodeMetrics (AtomicU64)
    ↓
MetricsCollector (HashMap<String, Arc<NodeMetrics>>)
    ↓
PipelineMonitor (snapshot & format)
```

**NodeMetrics:**
- frames_processed: AtomicU64
- errors_count: AtomicU64
- total_latency_us: AtomicU64
- Thread-safe, lock-free updates

**MetricsCollector:**
- Registers Arc<NodeMetrics> per node
- Provides snapshot() for current state
- Shared across all pipeline components

**PipelineMonitor:**
- Wraps MetricsCollector
- Formats human-readable reports
- Per-node statistics

### Error Recovery Architecture

```
ProcessingNode
    ↓
ResilientNode (wrapper)
    ├─ ErrorPolicy (how to handle errors)
    ├─ RestartStrategy (when to retry)
    └─ NodeMetrics (track failures)
```

**ErrorPolicy:**
- Propagate: Stop pipeline (default Phase 2 behavior)
- SkipFrame: Continue with next frame
- UseDefault: Substitute default DataFrame

**RestartStrategy:**
- Never: One-shot execution
- Immediate: Restart on failure
- Exponential: Backoff retry (base_ms, max_ms, max_attempts)
- CircuitBreaker: Threshold-based failure detection

**ResilientNode:**
- Wraps any ProcessingNode
- Catches errors during run()
- Applies ErrorPolicy
- Records metrics (errors, latency)
- Composition pattern - no changes to existing nodes

### Resource Management Architecture

```
BufferPool<Vec<f64>>
    ↓
PooledBuffer (RAII wrapper)
    ↓
DataFrame (Arc<PooledBuffer>)
```

**BufferPool:**
- Mutex<Vec<Vec<f64>>> for thread-safe storage
- get() pops from pool or allocates new
- PooledBuffer returns on drop
- Reduces allocation overhead

**Zero-Copy DataFrame:**
- payload: HashMap<String, Arc<Vec<f64>>>
- clone() shares data via Arc (no copy)
- Fanout duplicates Arc pointer, not data
- Reference counting manages lifetime

## Phase 3 Improvements

**Observability:**
- Real-time pipeline health monitoring
- Per-node performance metrics
- Debugging support for production issues

**Resilience:**
- Graceful error handling (no crashes)
- Configurable failure recovery
- Circuit breaker prevents cascading failures

**Efficiency:**
- Buffer reuse reduces GC pressure
- Zero-copy sharing for fanout nodes
- Lower memory footprint at scale

## Phase 3 Limitations (Future Work)

- No automatic restart strategy implementation yet (policies defined, not enforced)
- No integration with external monitoring systems (Prometheus, Grafana)
- BufferPool not yet integrated with DataFrame (manual management required)
- No real-time scheduling guarantees
- No dynamic reconfiguration
