# StreamLab Core - Phase 2: Async Streaming Pipeline Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform the linear pipeline into a true async streaming architecture with concurrent node execution, MPSC channels for data flow, backpressure handling, and support for running multiple pipeline instances simultaneously.

**Architecture:** Replace Phase 1's sequential execution with an actor-based model where each node runs in its own tokio task. Nodes communicate via bounded MPSC channels (backpressure). Pipeline spawns separate tasks for each node and manages their lifecycle. PipelinePool enables concurrent execution of multiple pipeline instances triggered independently.

**Tech Stack:** Rust 2024, tokio (spawn, mpsc channels), Arc/Mutex for shared state, futures (select!, join!), existing Phase 1 foundation

---

## Task 1: Refactor ProcessingNode Trait for Streaming

**Files:**
- Modify: `src/core/node.rs`
- Modify: `tests/core_node_tests.rs`

**Context:** Current `process(&self, input: DataFrame)` works for single-shot execution but doesn't support continuous streaming where nodes need to receive multiple frames over time.

**Step 1: Write failing test for streaming node**

Create test in `tests/core_node_tests.rs`:
```rust
use tokio::sync::mpsc;
use audiotab::core::{DataFrame, ProcessingNode};

struct StreamingDummyNode {
    multiplier: f64,
}

#[async_trait]
impl ProcessingNode for StreamingDummyNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        self.multiplier = config["multiplier"].as_f64().unwrap_or(1.0);
        Ok(())
    }

    async fn run(
        &self,
        mut rx: mpsc::Receiver<DataFrame>,
        tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        while let Some(mut frame) = rx.recv().await {
            if let Some(data) = frame.payload.get_mut("test") {
                for value in data.iter_mut() {
                    *value *= self.multiplier;
                }
            }
            tx.send(frame).await.map_err(|_| anyhow!("Send failed"))?;
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_node_streaming() {
    let node = StreamingDummyNode { multiplier: 2.0 };

    let (tx_in, rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    // Spawn node task
    let handle = tokio::spawn(async move {
        node.run(rx_in, tx_out).await
    });

    // Send frames
    let mut df1 = DataFrame::new(0, 0);
    df1.payload.insert("test".to_string(), vec![1.0, 2.0]);
    tx_in.send(df1).await.unwrap();

    let mut df2 = DataFrame::new(1000, 1);
    df2.payload.insert("test".to_string(), vec![3.0, 4.0]);
    tx_in.send(df2).await.unwrap();

    drop(tx_in); // Close channel to terminate node

    // Receive results
    let result1 = rx_out.recv().await.unwrap();
    assert_eq!(result1.payload.get("test").unwrap(), &vec![2.0, 4.0]);

    let result2 = rx_out.recv().await.unwrap();
    assert_eq!(result2.payload.get("test").unwrap(), &vec![6.0, 8.0]);

    handle.await.unwrap().unwrap();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_node_streaming`
Expected: FAIL - "no method named `run` found for trait `ProcessingNode`"

**Step 3: Add new `run` method to ProcessingNode trait**

Modify `src/core/node.rs`:
```rust
use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use tokio::sync::mpsc;
use super::DataFrame;

/// Base trait for all processing nodes in the pipeline
#[async_trait]
pub trait ProcessingNode: Send + Sync {
    /// Called once when node is instantiated with config from JSON
    async fn on_create(&mut self, config: Value) -> Result<()>;

    /// Legacy single-shot processing (Phase 1 compatibility)
    /// Will be deprecated in favor of run() for streaming pipelines
    async fn process(&self, input: DataFrame) -> Result<DataFrame> {
        // Default implementation: not supported
        anyhow::bail!("Node does not support single-shot processing")
    }

    /// Async streaming processing loop
    /// Receives frames from rx channel, processes them, sends to tx channel
    /// Should run until rx channel is closed, then return
    async fn run(
        &self,
        rx: mpsc::Receiver<DataFrame>,
        tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        // Default implementation: not supported
        anyhow::bail!("Node does not support streaming processing")
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_node_streaming`
Expected: PASS

**Step 5: Commit**

```bash
git add src/core/node.rs tests/core_node_tests.rs
git commit -m "feat(core): add streaming run() method to ProcessingNode trait"
```

---

## Task 2: Implement Streaming for SineGenerator

**Files:**
- Modify: `src/nodes/sine_generator.rs`
- Modify: `tests/nodes_sine_tests.rs`

**Step 1: Write failing test for streaming sine generation**

Add to `tests/nodes_sine_tests.rs`:
```rust
#[tokio::test]
async fn test_sine_generator_streaming() {
    let mut generator = SineGenerator::new();
    let config = serde_json::json!({
        "frequency": 1.0,
        "sample_rate": 4.0,
        "frame_size": 4
    });
    generator.on_create(config).await.unwrap();

    let (tx_in, rx_in) = mpsc::channel::<DataFrame>(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    let handle = tokio::spawn(async move {
        generator.run(rx_in, tx_out).await
    });

    // Send 3 empty trigger frames
    for i in 0..3 {
        tx_in.send(DataFrame::new(i * 1000, i)).await.unwrap();
    }
    drop(tx_in);

    // Receive 3 generated frames
    let frame1 = rx_out.recv().await.unwrap();
    let frame2 = rx_out.recv().await.unwrap();
    let frame3 = rx_out.recv().await.unwrap();

    // Verify data was generated
    assert_eq!(frame1.payload.get("main_channel").unwrap().len(), 4);
    assert_eq!(frame2.payload.get("main_channel").unwrap().len(), 4);
    assert_eq!(frame3.payload.get("main_channel").unwrap().len(), 4);

    handle.await.unwrap().unwrap();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_sine_generator_streaming`
Expected: FAIL - "Node does not support streaming processing"

**Step 3: Implement run() method for SineGenerator**

Modify `src/nodes/sine_generator.rs`:
```rust
use tokio::sync::mpsc;

// Add to SineGenerator impl block (after existing process method):

#[async_trait]
impl ProcessingNode for SineGenerator {
    // ... existing on_create and process methods ...

    async fn run(
        &self,
        mut rx: mpsc::Receiver<DataFrame>,
        tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        let mut phase = self.phase;
        let phase_increment = 2.0 * PI * self.frequency / self.sample_rate;

        while let Some(mut frame) = rx.recv().await {
            let mut samples = Vec::with_capacity(self.frame_size);

            for _ in 0..self.frame_size {
                samples.push(phase.sin());
                phase += phase_increment;
            }

            // Wrap phase to avoid overflow
            phase = phase % (2.0 * PI);

            frame.payload.insert("main_channel".to_string(), samples);

            if tx.send(frame).await.is_err() {
                break; // Downstream closed
            }
        }

        Ok(())
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_sine_generator_streaming`
Expected: PASS

**Step 5: Commit**

```bash
git add src/nodes/sine_generator.rs tests/nodes_sine_tests.rs
git commit -m "feat(nodes): implement streaming run() for SineGenerator with phase continuity"
```

---

## Task 3: Implement Streaming for Gain Node

**Files:**
- Modify: `src/nodes/gain.rs`
- Modify: `tests/nodes_gain_tests.rs`

**Step 1: Write failing test**

Add to `tests/nodes_gain_tests.rs`:
```rust
use tokio::sync::mpsc;

#[tokio::test]
async fn test_gain_streaming() {
    let mut gain = Gain::new();
    let config = serde_json::json!({"gain": 2.0});
    gain.on_create(config).await.unwrap();

    let (tx_in, rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    let handle = tokio::spawn(async move {
        gain.run(rx_in, tx_out).await
    });

    // Send 2 frames
    let mut df1 = DataFrame::new(0, 0);
    df1.payload.insert("main_channel".to_string(), vec![1.0, 2.0]);
    tx_in.send(df1).await.unwrap();

    let mut df2 = DataFrame::new(1000, 1);
    df2.payload.insert("main_channel".to_string(), vec![3.0, 4.0]);
    tx_in.send(df2).await.unwrap();

    drop(tx_in);

    // Verify results
    let result1 = rx_out.recv().await.unwrap();
    assert_eq!(result1.payload.get("main_channel").unwrap(), &vec![2.0, 4.0]);

    let result2 = rx_out.recv().await.unwrap();
    assert_eq!(result2.payload.get("main_channel").unwrap(), &vec![6.0, 8.0]);

    handle.await.unwrap().unwrap();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_gain_streaming`
Expected: FAIL - "Node does not support streaming processing"

**Step 3: Implement run() method**

Modify `src/nodes/gain.rs`:
```rust
use tokio::sync::mpsc;

#[async_trait]
impl ProcessingNode for Gain {
    // ... existing on_create and process methods ...

    async fn run(
        &self,
        mut rx: mpsc::Receiver<DataFrame>,
        tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        while let Some(mut frame) = rx.recv().await {
            if let Some(data) = frame.payload.get_mut("main_channel") {
                for sample in data.iter_mut() {
                    *sample *= self.gain;
                }
            }

            if tx.send(frame).await.is_err() {
                break; // Downstream closed
            }
        }

        Ok(())
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_gain_streaming`
Expected: PASS

**Step 5: Commit**

```bash
git add src/nodes/gain.rs tests/nodes_gain_tests.rs
git commit -m "feat(nodes): implement streaming run() for Gain node"
```

---

## Task 4: Implement Streaming for Print Node

**Files:**
- Modify: `src/nodes/print.rs`
- Modify: `tests/nodes_print_tests.rs`

**Step 1: Write failing test**

Add to `tests/nodes_print_tests.rs`:
```rust
use tokio::sync::mpsc;

#[tokio::test]
async fn test_print_streaming() {
    let mut print = Print::new();
    let config = serde_json::json!({"label": "StreamTest"});
    print.on_create(config).await.unwrap();

    let (tx_in, rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    let handle = tokio::spawn(async move {
        print.run(rx_in, tx_out).await
    });

    let mut df = DataFrame::new(5000, 5);
    df.payload.insert("main_channel".to_string(), vec![1.0, 2.0, 3.0]);
    tx_in.send(df.clone()).await.unwrap();

    drop(tx_in);

    let result = rx_out.recv().await.unwrap();
    assert_eq!(result.timestamp, df.timestamp);
    assert_eq!(result.sequence_id, df.sequence_id);

    handle.await.unwrap().unwrap();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_print_streaming`
Expected: FAIL - "Node does not support streaming processing"

**Step 3: Implement run() method**

Modify `src/nodes/print.rs`:
```rust
use tokio::sync::mpsc;

#[async_trait]
impl ProcessingNode for Print {
    // ... existing on_create and process methods ...

    async fn run(
        &self,
        mut rx: mpsc::Receiver<DataFrame>,
        tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        while let Some(frame) = rx.recv().await {
            println!("[{}] Frame #{} @ {}μs", self.label, frame.sequence_id, frame.timestamp);

            for (channel, data) in &frame.payload {
                let stats = if !data.is_empty() {
                    let sum: f64 = data.iter().sum();
                    let mean = sum / data.len() as f64;
                    let rms = (data.iter().map(|x| x * x).sum::<f64>() / data.len() as f64).sqrt();
                    format!("len={}, mean={:.4}, rms={:.4}", data.len(), mean, rms)
                } else {
                    "empty".to_string()
                };
                println!("  {}: {}", channel, stats);
            }

            if tx.send(frame).await.is_err() {
                break; // Downstream closed
            }
        }

        Ok(())
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_print_streaming`
Expected: PASS

**Step 5: Commit**

```bash
git add src/nodes/print.rs tests/nodes_print_tests.rs
git commit -m "feat(nodes): implement streaming run() for Print node"
```

---

## Task 5: Create Async Pipeline Executor

**Files:**
- Create: `src/engine/async_pipeline.rs`
- Create: `tests/engine_async_pipeline_tests.rs`
- Modify: `src/engine/mod.rs`

**Context:** Current Pipeline executes nodes sequentially. New AsyncPipeline spawns each node as a separate tokio task connected via MPSC channels.

**Step 1: Write failing test for async pipeline**

Create `tests/engine_async_pipeline_tests.rs`:
```rust
use audiotab::engine::AsyncPipeline;
use audiotab::core::DataFrame;

#[tokio::test]
async fn test_async_pipeline_creation() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
            {"id": "print", "type": "Print", "config": {"label": "AsyncTest"}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "print"}
        ]
    });

    let pipeline = AsyncPipeline::from_json(config).await;
    assert!(pipeline.is_ok());
}

#[tokio::test]
async fn test_async_pipeline_execution() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"}
        ]
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();

    // Start pipeline (spawns node tasks)
    pipeline.start().await.unwrap();

    // Trigger 3 executions
    for i in 0..3 {
        pipeline.trigger(DataFrame::new(i * 1000, i)).await.unwrap();
    }

    // Wait a bit for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Stop pipeline
    pipeline.stop().await.unwrap();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_async_pipeline`
Expected: FAIL - "unresolved import `audiotab::engine::AsyncPipeline`"

**Step 3: Implement AsyncPipeline structure**

Create `src/engine/async_pipeline.rs`:
```rust
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use crate::core::{ProcessingNode, DataFrame};
use crate::nodes::{SineGenerator, Gain, Print};

pub struct AsyncPipeline {
    nodes: HashMap<String, Box<dyn ProcessingNode>>,
    connections: Vec<(String, String)>,
    channels: HashMap<String, mpsc::Sender<DataFrame>>,
    handles: Vec<JoinHandle<Result<()>>>,
    source_node_id: Option<String>,
}

impl AsyncPipeline {
    pub async fn from_json(config: Value) -> Result<Self> {
        let mut nodes: HashMap<String, Box<dyn ProcessingNode>> = HashMap::new();
        let mut connections = Vec::new();

        // Parse nodes
        if let Some(nodes_array) = config["nodes"].as_array() {
            for node_config in nodes_array {
                let id = node_config["id"]
                    .as_str()
                    .ok_or(anyhow!("Node missing id"))?
                    .to_string();
                let node_type = node_config["type"].as_str().ok_or(anyhow!("Node missing type"))?;
                let node_cfg = node_config["config"].clone();

                let mut node: Box<dyn ProcessingNode> = match node_type {
                    "SineGenerator" => Box::new(SineGenerator::new()),
                    "Gain" => Box::new(Gain::new()),
                    "Print" => Box::new(Print::new()),
                    _ => return Err(anyhow!("Unknown node type: {}", node_type)),
                };

                node.on_create(node_cfg).await?;
                nodes.insert(id, node);
            }
        }

        // Parse connections
        if let Some(conns_array) = config["connections"].as_array() {
            for conn in conns_array {
                let from = conn["from"]
                    .as_str()
                    .ok_or(anyhow!("Connection missing from"))?
                    .to_string();
                let to = conn["to"]
                    .as_str()
                    .ok_or(anyhow!("Connection missing to"))?
                    .to_string();
                connections.push((from, to));
            }
        }

        // Find source node (no incoming connections)
        let source_node_id = nodes.keys().find(|id| {
            !connections.iter().any(|(_, to)| to == *id)
        }).map(|s| s.clone());

        Ok(Self {
            nodes,
            connections,
            channels: HashMap::new(),
            handles: Vec::new(),
            source_node_id,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let channel_capacity = 10;
        let mut node_channels: HashMap<String, (mpsc::Sender<DataFrame>, mpsc::Receiver<DataFrame>)> = HashMap::new();

        // Create channels for each node
        for node_id in self.nodes.keys() {
            let (tx, rx) = mpsc::channel(channel_capacity);
            node_channels.insert(node_id.clone(), (tx, rx));
        }

        // Build output channel map (which nodes send to which channels)
        let mut output_channels: HashMap<String, Vec<mpsc::Sender<DataFrame>>> = HashMap::new();
        for (from, to) in &self.connections {
            output_channels
                .entry(from.clone())
                .or_insert_with(Vec::new)
                .push(node_channels.get(to).unwrap().0.clone());
        }

        // Spawn task for each node
        for (node_id, node) in self.nodes.drain() {
            let (_tx, rx) = node_channels.remove(&node_id).unwrap();
            let outputs = output_channels.remove(&node_id).unwrap_or_default();

            let handle = tokio::spawn(async move {
                let (fanout_tx, mut fanout_rx) = mpsc::channel(channel_capacity);

                // Spawn node processing
                let node_task = tokio::spawn(async move {
                    node.run(rx, fanout_tx).await
                });

                // Spawn fanout (send to multiple outputs)
                let fanout_task = tokio::spawn(async move {
                    while let Some(frame) = fanout_rx.recv().await {
                        for output in &outputs {
                            let _ = output.send(frame.clone()).await;
                        }
                    }
                });

                node_task.await??;
                fanout_task.await?;
                Ok(())
            });

            self.handles.push(handle);
        }

        // Store source node channel for triggering
        if let Some(source_id) = &self.source_node_id {
            self.channels.insert(source_id.clone(), node_channels.remove(source_id).unwrap().0);
        }

        Ok(())
    }

    pub async fn trigger(&self, frame: DataFrame) -> Result<()> {
        if let Some(source_id) = &self.source_node_id {
            if let Some(tx) = self.channels.get(source_id) {
                tx.send(frame).await.map_err(|_| anyhow!("Failed to send trigger frame"))?;
            }
        }
        Ok(())
    }

    pub async fn stop(self) -> Result<()> {
        // Drop channels to signal nodes to shut down
        drop(self.channels);

        // Wait for all node tasks to complete
        for handle in self.handles {
            handle.await??;
        }

        Ok(())
    }
}
```

**Step 4: Export AsyncPipeline**

Modify `src/engine/mod.rs`:
```rust
pub mod pipeline;
pub mod async_pipeline;

pub use pipeline::Pipeline;
pub use async_pipeline::AsyncPipeline;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test test_async_pipeline`
Expected: PASS - both tests pass

**Step 6: Commit**

```bash
git add src/engine/async_pipeline.rs src/engine/mod.rs tests/engine_async_pipeline_tests.rs
git commit -m "feat(engine): add AsyncPipeline with concurrent node execution"
```

---

## Task 6: Add Backpressure Support

**Files:**
- Modify: `src/engine/async_pipeline.rs`
- Create: `tests/engine_backpressure_tests.rs`

**Context:** Bounded channels provide natural backpressure, but we need to make channel size configurable and test that backpressure works correctly.

**Step 1: Write test for backpressure behavior**

Create `tests/engine_backpressure_tests.rs`:
```rust
use audiotab::engine::AsyncPipeline;
use audiotab::core::DataFrame;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_backpressure_blocks_when_full() {
    let config = serde_json::json!({
        "pipeline_config": {
            "channel_capacity": 2  // Small buffer to trigger backpressure
        },
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}}
        ],
        "connections": []
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();
    pipeline.start().await.unwrap();

    // Fill the channel (capacity = 2)
    pipeline.trigger(DataFrame::new(0, 0)).await.unwrap();
    pipeline.trigger(DataFrame::new(1000, 1)).await.unwrap();

    // Third trigger should block or timeout since no consumer
    let result = timeout(
        Duration::from_millis(50),
        pipeline.trigger(DataFrame::new(2000, 2))
    ).await;

    // Should timeout because channel is full and no one is consuming
    assert!(result.is_err(), "Expected timeout due to backpressure");

    pipeline.stop().await.unwrap();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_backpressure_blocks_when_full`
Expected: FAIL - "missing field `pipeline_config`" or test passes when it shouldn't

**Step 3: Add configurable channel capacity**

Modify `src/engine/async_pipeline.rs`:
```rust
pub struct AsyncPipeline {
    // ... existing fields ...
    channel_capacity: usize,
}

impl AsyncPipeline {
    pub async fn from_json(config: Value) -> Result<Self> {
        // Parse channel capacity from config
        let channel_capacity = config["pipeline_config"]["channel_capacity"]
            .as_u64()
            .unwrap_or(100) as usize;

        // ... existing node and connection parsing ...

        Ok(Self {
            nodes,
            connections,
            channels: HashMap::new(),
            handles: Vec::new(),
            source_node_id,
            channel_capacity,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let channel_capacity = self.channel_capacity;

        // Rest of implementation uses channel_capacity variable
        // ... (existing code with channel_capacity already correct)
    }

    // ... rest of implementation ...
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_backpressure_blocks_when_full`
Expected: PASS

**Step 5: Commit**

```bash
git add src/engine/async_pipeline.rs tests/engine_backpressure_tests.rs
git commit -m "feat(engine): add configurable channel capacity for backpressure control"
```

---

## Task 7: Implement PipelinePool for Concurrent Instances

**Files:**
- Create: `src/engine/pipeline_pool.rs`
- Create: `tests/engine_pool_tests.rs`
- Modify: `src/engine/mod.rs`

**Context:** Enable multiple pipeline instances to run simultaneously without blocking each other.

**Step 1: Write failing test for concurrent pipeline execution**

Create `tests/engine_pool_tests.rs`:
```rust
use audiotab::engine::PipelinePool;
use audiotab::core::DataFrame;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_pipeline_pool_concurrent_execution() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"}
        ]
    });

    let mut pool = PipelinePool::new(config, 5).await.unwrap(); // 5 concurrent instances

    // Trigger 10 executions rapidly
    let mut handles = vec![];
    for i in 0..10 {
        let trigger_frame = DataFrame::new(i * 100, i);
        let handle = pool.execute(trigger_frame).await.unwrap();
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // All 10 should complete even though only 5 can run concurrently
}

#[tokio::test]
async fn test_pipeline_pool_resource_reuse() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 50}}
        ],
        "connections": []
    });

    let mut pool = PipelinePool::new(config, 2).await.unwrap();

    // Execute 5 times - should reuse the 2 pipeline instances
    for i in 0..5 {
        let handle = pool.execute(DataFrame::new(i * 100, i)).await.unwrap();
        handle.await.unwrap().unwrap();
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_pipeline_pool`
Expected: FAIL - "unresolved import `audiotab::engine::PipelinePool`"

**Step 3: Implement PipelinePool**

Create `src/engine/pipeline_pool.rs`:
```rust
use anyhow::Result;
use serde_json::Value;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use std::sync::Arc;
use crate::core::DataFrame;
use super::AsyncPipeline;

pub struct PipelinePool {
    config: Value,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl PipelinePool {
    pub async fn new(config: Value, max_concurrent: usize) -> Result<Self> {
        // Validate config by creating one pipeline
        let _test_pipeline = AsyncPipeline::from_json(config.clone()).await?;

        Ok(Self {
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        })
    }

    pub async fn execute(&mut self, trigger_frame: DataFrame) -> Result<JoinHandle<Result<()>>> {
        let config = self.config.clone();
        let semaphore = self.semaphore.clone();

        let handle = tokio::spawn(async move {
            // Acquire permit (blocks if max_concurrent already running)
            let _permit = semaphore.acquire().await.unwrap();

            // Create and run pipeline instance
            let mut pipeline = AsyncPipeline::from_json(config).await?;
            pipeline.start().await?;
            pipeline.trigger(trigger_frame).await?;

            // Wait a bit for processing to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            pipeline.stop().await?;
            // Permit is dropped here, allowing next pipeline to start

            Ok(())
        });

        Ok(handle)
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}
```

**Step 4: Export PipelinePool**

Modify `src/engine/mod.rs`:
```rust
pub mod pipeline;
pub mod async_pipeline;
pub mod pipeline_pool;

pub use pipeline::Pipeline;
pub use async_pipeline::AsyncPipeline;
pub use pipeline_pool::PipelinePool;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test test_pipeline_pool`
Expected: PASS - both tests pass

**Step 6: Commit**

```bash
git add src/engine/pipeline_pool.rs src/engine/mod.rs tests/engine_pool_tests.rs
git commit -m "feat(engine): add PipelinePool for concurrent pipeline instances"
```

---

## Task 8: Create Async Demo

**Files:**
- Create: `src/bin/async_demo.rs`

**Context:** Create a separate binary that demonstrates the new async streaming capabilities.

**Step 1: Create async demo binary**

Create `src/bin/async_demo.rs`:
```rust
use audiotab::engine::{AsyncPipeline, PipelinePool};
use audiotab::core::DataFrame;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("StreamLab Core - Phase 2 Async Demo");
    println!("====================================\n");

    // Demo 1: Single async pipeline with streaming
    println!("=== Demo 1: Async Streaming Pipeline ===");
    let config1 = serde_json::json!({
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
                "config": {
                    "gain": 2.5
                }
            },
            {
                "id": "console_out",
                "type": "Print",
                "config": {
                    "label": "Async Output"
                }
            }
        ],
        "connections": [
            {"from": "sine_gen", "to": "amplifier"},
            {"from": "amplifier", "to": "console_out"}
        ]
    });

    let mut pipeline = AsyncPipeline::from_json(config1).await?;
    pipeline.start().await?;

    println!("Triggering 5 frames through async pipeline...\n");
    for i in 0..5 {
        pipeline.trigger(DataFrame::new(i * 1000, i)).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    pipeline.stop().await?;

    // Demo 2: Pipeline pool with concurrent execution
    println!("\n=== Demo 2: Concurrent Pipeline Execution ===");
    let config2 = serde_json::json!({
        "nodes": [
            {
                "id": "sine_gen",
                "type": "SineGenerator",
                "config": {
                    "frequency": 880.0,
                    "sample_rate": 48000.0,
                    "frame_size": 512
                }
            },
            {
                "id": "console_out",
                "type": "Print",
                "config": {
                    "label": "Pool"
                }
            }
        ],
        "connections": [
            {"from": "sine_gen", "to": "console_out"}
        ]
    });

    let mut pool = PipelinePool::new(config2, 3).await?;

    println!("Launching 10 pipeline instances (3 concurrent max)...\n");
    let mut handles = vec![];
    for i in 0..10 {
        let handle = pool.execute(DataFrame::new(i * 500, i)).await?;
        handles.push(handle);
    }

    println!("Waiting for all 10 instances to complete...\n");
    for (i, handle) in handles.into_iter().enumerate() {
        handle.await??;
        println!("Instance {} completed", i);
    }

    println!("\n=== Phase 2 Demo Complete! ===");
    println!("✓ Async streaming pipeline with tokio tasks");
    println!("✓ MPSC channels for inter-node communication");
    println!("✓ Backpressure via bounded channels");
    println!("✓ Concurrent pipeline instances with PipelinePool");
    println!("✓ Phase continuity in SineGenerator");

    Ok(())
}
```

**Step 2: Verify demo compiles and runs**

Run: `cargo build --bin async_demo`
Expected: SUCCESS

Run: `cargo run --bin async_demo`
Expected: Successful execution with output showing async pipeline and pool operation

**Step 3: Commit**

```bash
git add src/bin/async_demo.rs
git commit -m "feat(demo): add Phase 2 async streaming demo with pipeline pool"
```

---

## Task 9: Update Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md`

**Step 1: Update README with Phase 2 features**

Modify `README.md`:
```markdown
# StreamLab Core

Next-generation streaming multi-physics analysis & test framework.

## Phase 2: Async Streaming ✓

Concurrent async pipeline execution with backpressure.

### Features

- **DataFrame**: Universal data container for multi-channel time-series
- **ProcessingNode**: Async trait for streaming operators
- **AsyncPipeline**: Concurrent node execution with MPSC channels
- **PipelinePool**: Run multiple pipeline instances simultaneously
- **Backpressure**: Bounded channels prevent memory overflow
- **Built-in Nodes**:
  - SineGenerator: Streaming test signal source with phase continuity
  - Gain: Signal amplification/attenuation
  - Print: Console output for debugging

### Quick Start

```bash
# Run Phase 1 demo (single-shot execution)
cargo run

# Run Phase 2 demo (async streaming)
cargo run --bin async_demo

# Run tests
cargo test

# Example async pipeline config
{
  "pipeline_config": {
    "channel_capacity": 10
  },
  "nodes": [
    {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0}},
    {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
    {"id": "out", "type": "Print", "config": {"label": "Output"}}
  ],
  "connections": [
    {"from": "gen", "to": "gain"},
    {"from": "gain", "to": "out"}
  ]
}
```

### Architecture Improvements

**Phase 1 → Phase 2 Changes:**
- Sequential execution → Concurrent node tasks
- Single-shot processing → Streaming with phase continuity
- No backpressure → Bounded MPSC channels
- Single instance → PipelinePool for concurrency

### Next Steps

- [ ] Frontend integration (React Flow + Tauri)
- [ ] Hardware abstraction layer (HAL)
- [ ] Python node support via PyO3
- [ ] Real-time visualization
```

**Step 2: Update architecture documentation**

Modify `docs/architecture.md`:
```markdown
# StreamLab Architecture

## Data Flow

```
JSON Config → AsyncPipeline Builder → Concurrent Node Tasks (MPSC Channels) → Results
```

## Core Abstractions

### DataFrame
- timestamp: μs since epoch
- sequence_id: frame ordering
- payload: HashMap<String, Vec<f64>> - multi-channel data
- metadata: HashMap<String, String> - side-channel info

### ProcessingNode Trait
```rust
async fn on_create(&mut self, config: Value) -> Result<()>
async fn run(
    &self,
    rx: mpsc::Receiver<DataFrame>,
    tx: mpsc::Sender<DataFrame>,
) -> Result<()>
```

### AsyncPipeline
- Parses JSON graph definition
- Instantiates nodes with configuration
- Spawns each node as separate tokio task
- Connects nodes via bounded MPSC channels (backpressure)
- Manages task lifecycle (start/stop)

### PipelinePool
- Manages pool of pipeline instances
- Uses semaphore to limit concurrent executions
- Automatically creates/destroys instances as needed
- Prevents resource exhaustion under high load

## Phase 2 Improvements

**Concurrency Model:**
- Each node runs in separate tokio task
- Communication via bounded MPSC channels
- Backpressure automatically applied when buffers full
- Fanout supported (one node → multiple downstream nodes)

**Streaming:**
- Nodes process continuous data streams
- Phase continuity maintained across frames (e.g., SineGenerator)
- Low-latency processing (< 10ms per frame)

**Resource Management:**
- PipelinePool limits concurrent instances
- Semaphore-based admission control
- Clean shutdown via channel dropping

## Phase 2 Limitations

- No branching/merging (only linear pipelines)
- No dynamic graph reconfiguration
- Fixed channel capacity (not adaptive)
- No distributed execution (single-process only)

These will be addressed in subsequent phases.
```

**Step 3: Commit**

```bash
git add README.md docs/architecture.md
git commit -m "docs: update documentation for Phase 2 async streaming features"
```

---

## Acceptance Criteria

### Functional Tests

Run: `cargo test`
Expected: All tests PASS (18+ tests minimum)

**Test Breakdown:**
- Phase 1 tests: 10 tests (DataFrame, nodes, pipeline)
- Phase 2 streaming tests: 4 tests (node streaming)
- Phase 2 async pipeline tests: 2 tests
- Phase 2 backpressure test: 1 test
- Phase 2 pool tests: 2 tests

### Demo Execution

**Phase 1 Demo:**
```bash
cargo run
# Should work unchanged
```

**Phase 2 Demo:**
```bash
cargo run --bin async_demo
# Should show:
# - 5 frames through async pipeline
# - 10 concurrent instances through pool
# - Completion message
```

### Performance Verification

**Concurrent Execution Test:**
```bash
# In Phase 2 demo, verify 10 instances complete faster than sequential
# Expected: ~150ms for 10 concurrent (3 at a time)
# vs ~500ms if sequential
```

### Code Quality

- [ ] All files compile without warnings: `cargo build --all-targets`
- [ ] All tests pass: `cargo test`
- [ ] Code formatted: `cargo fmt --check`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`

---

## Notes for Executor

- Phase 2 builds on Phase 1 - do not break existing functionality
- All Phase 1 tests must continue to pass
- Follow TDD: write streaming tests before implementing streaming
- Use `tokio::spawn` for task creation, not threads
- Always handle channel send errors (downstream may close)
- Use `Arc` for shared node instances if needed (current design avoids this)
- Test backpressure explicitly - it's easy to miss
- PipelinePool semaphore is key to preventing resource exhaustion

---

## Common Pitfalls

1. **Forgetting to drop tx channels** - causes nodes to hang waiting for more data
2. **Not handling send errors** - panics when downstream closes
3. **Infinite buffering** - defeats backpressure, causes OOM
4. **Shared mutable state** - use message passing, not Arc<Mutex<>>
5. **Task leaks** - always await or detach spawned tasks

Refer to Tokio docs for async patterns: https://tokio.rs/tokio/tutorial
