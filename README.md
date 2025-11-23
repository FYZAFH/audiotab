# StreamLab Core

Next-generation streaming multi-physics analysis & test framework.

## Phase 1: Core Engine ✓

Basic Rust backend with JSON-driven pipeline execution.

### Features

- **DataFrame**: Universal data container for multi-channel time-series
- **ProcessingNode**: Async trait for all pipeline operators
- **Pipeline**: JSON configuration to executable graph converter
- **Built-in Nodes**:
  - SineGenerator: Test signal source
  - Gain: Signal amplification/attenuation
  - Print: Console output for debugging

## Phase 2: Async Streaming ✓

Concurrent node execution with tokio tasks and MPSC channels.

### Features

- **Streaming ProcessingNode**: New `run()` method for continuous data processing
  - Receives frames via `mpsc::Receiver<DataFrame>`
  - Sends processed frames via `mpsc::Sender<DataFrame>`
  - Each node runs in its own tokio task
- **AsyncPipeline**: Concurrent pipeline executor
  - Spawns each node as independent tokio task
  - MPSC channels for inter-node communication
  - Configurable channel capacity for backpressure control
  - Fanout pattern for multiple downstream connections
- **PipelinePool**: Execute multiple pipeline instances concurrently
  - Semaphore-based admission control
  - Configurable max concurrent instances
  - Automatic queuing when at capacity
- **Phase Continuity**: SineGenerator maintains phase across streaming frames

### Quick Start

```bash
# Run Phase 1 demo (linear pipeline)
cargo run

# Run Phase 2 demo (async streaming)
cargo run --bin async_demo

# Run all tests
cargo test
```

### Example Async Pipeline Config

```json
{
  "pipeline_config": {
    "channel_capacity": 10
  },
  "nodes": [
    {
      "id": "sine_gen",
      "type": "SineGenerator",
      "config": {
        "frequency": 440.0,
        "sample_rate": 48000.0,
        "frame_size": 1024
      }
    },
    {
      "id": "amplifier",
      "type": "Gain",
      "config": {"gain": 2.5}
    },
    {
      "id": "console_out",
      "type": "Print",
      "config": {"label": "Async Output"}
    }
  ],
  "connections": [
    {"from": "sine_gen", "to": "amplifier"},
    {"from": "amplifier", "to": "console_out"}
  ]
}
```

### Architecture Improvements

**Phase 1 → Phase 2:**
- Linear sequential execution → Concurrent node execution
- Synchronous blocking → Async non-blocking with tokio
- No backpressure → Bounded channels with configurable capacity
- Single pipeline instance → PipelinePool with concurrent instances
- One-shot processing → Continuous streaming with phase continuity

## Phase 3: Production Readiness ✓

Error recovery, observability, and resource management for production deployment.

### Features

- **Observability System**:
  - NodeMetrics: Lock-free atomic counters (frames, errors, latency)
  - MetricsCollector: Central registry for all node metrics
  - PipelineMonitor: Human-readable reporting
- **Error Recovery**:
  - ErrorPolicy: Propagate, SkipFrame, UseDefault strategies
  - RestartStrategy: Never, Immediate, Exponential, CircuitBreaker
  - ResilientNode: Wrapper that applies policies automatically
- **Resource Management**:
  - BufferPool: Reusable buffer allocation with drop-based return
  - Zero-copy DataFrame: Arc<Vec<f64>> for reference-counted sharing
  - Reduced GC pressure in high-throughput scenarios

### Quick Start

```bash
# Run Phase 3 demo (production features)
cargo run --bin production_demo

# Run all tests (including production features)
cargo test
```

### Example with Monitoring

```rust
let mut pipeline = AsyncPipeline::from_json(config).await?;
pipeline.start().await?;

// Process frames
for i in 0..100 {
    pipeline.trigger(DataFrame::new(i * 1000, i)).await?;
}

// Get metrics report
if let Some(monitor) = pipeline.get_monitor() {
    println!("{}", monitor.generate_report());
}
```

### Next Steps

- [ ] Real-time scheduling with dedicated thread pools
- [ ] Dynamic pipeline reconfiguration without restart
- [ ] HAL interfaces for hardware integration
- [ ] Advanced buffer management (ring buffers, NUMA awareness)
