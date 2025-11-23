# Phase 3: Production Readiness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform StreamLab Core into a production-ready system with error recovery, observability, and efficient resource management.

**Architecture:** Add three core capabilities: 1) NodeMetrics with atomic counters for observability, 2) ResilientNode wrapper with configurable error policies and restart strategies, 3) BufferPool system with Arc-based zero-copy sharing. Uses composition pattern to wrap existing nodes without breaking changes.

**Tech Stack:** tokio, std::sync::{atomic::AtomicU64, Arc, Mutex}, anyhow, serde_json

---

## Task 1: Create NodeMetrics Foundation

**Files:**
- Create: `src/observability/mod.rs`
- Create: `src/observability/metrics.rs`
- Create: `tests/observability_metrics_tests.rs`
- Modify: `src/lib.rs` (add observability module)

**Step 1: Write failing test for NodeMetrics**

Create test file that verifies atomic counter increments:

```rust
use audiotab::observability::NodeMetrics;
use std::sync::Arc;

#[test]
fn test_metrics_creation() {
    let metrics = NodeMetrics::new("test_node");
    assert_eq!(metrics.node_id(), "test_node");
    assert_eq!(metrics.frames_processed(), 0);
    assert_eq!(metrics.errors_count(), 0);
}

#[test]
fn test_metrics_increment() {
    let metrics = Arc::new(NodeMetrics::new("test_node"));

    metrics.record_frame_processed();
    metrics.record_frame_processed();
    assert_eq!(metrics.frames_processed(), 2);

    metrics.record_error();
    assert_eq!(metrics.errors_count(), 1);
}

#[tokio::test]
async fn test_metrics_latency_tracking() {
    let metrics = NodeMetrics::new("test_node");

    let start = metrics.start_processing();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    metrics.finish_processing(start);

    let avg_latency = metrics.avg_latency_us();
    assert!(avg_latency >= 10_000); // At least 10ms in microseconds
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test observability_metrics_tests`
Expected: FAIL with "no `observability` in the root"

**Step 3: Create metrics module structure**

In `src/lib.rs`, add:
```rust
pub mod observability;
```

Create `src/observability/mod.rs`:
```rust
pub mod metrics;

pub use metrics::NodeMetrics;
```

Create `src/observability/metrics.rs`:
```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct NodeMetrics {
    node_id: String,
    frames_processed: AtomicU64,
    errors_count: AtomicU64,
    total_latency_us: AtomicU64,
}

impl NodeMetrics {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            frames_processed: AtomicU64::new(0),
            errors_count: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn frames_processed(&self) -> u64 {
        self.frames_processed.load(Ordering::Relaxed)
    }

    pub fn errors_count(&self) -> u64 {
        self.errors_count.load(Ordering::Relaxed)
    }

    pub fn record_frame_processed(&self) {
        self.frames_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn start_processing(&self) -> Instant {
        Instant::now()
    }

    pub fn finish_processing(&self, start: Instant) {
        let latency_us = start.elapsed().as_micros() as u64;
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    pub fn avg_latency_us(&self) -> u64 {
        let frames = self.frames_processed();
        if frames == 0 {
            return 0;
        }
        self.total_latency_us.load(Ordering::Relaxed) / frames
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test observability_metrics_tests`
Expected: PASS (all 3 tests)

**Step 5: Commit**

```bash
git add src/lib.rs src/observability/ tests/observability_metrics_tests.rs
git commit -m "feat(observability): add NodeMetrics with atomic counters

- Lock-free metrics tracking with AtomicU64
- Track frames processed, errors, and latency
- Thread-safe for concurrent access from multiple nodes

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Implement MetricsCollector

**Files:**
- Create: `src/observability/collector.rs`
- Create: `tests/observability_collector_tests.rs`
- Modify: `src/observability/mod.rs`

**Step 1: Write failing test for MetricsCollector**

```rust
use audiotab::observability::{NodeMetrics, MetricsCollector};
use std::sync::Arc;

#[test]
fn test_collector_registration() {
    let mut collector = MetricsCollector::new();
    let metrics = Arc::new(NodeMetrics::new("node1"));

    collector.register("node1", metrics.clone());

    let snapshot = collector.snapshot();
    assert_eq!(snapshot.len(), 1);
    assert!(snapshot.contains_key("node1"));
}

#[test]
fn test_collector_aggregation() {
    let mut collector = MetricsCollector::new();

    let m1 = Arc::new(NodeMetrics::new("node1"));
    let m2 = Arc::new(NodeMetrics::new("node2"));

    m1.record_frame_processed();
    m1.record_frame_processed();
    m2.record_frame_processed();

    collector.register("node1", m1);
    collector.register("node2", m2);

    let snapshot = collector.snapshot();

    assert_eq!(snapshot.get("node1").unwrap().frames_processed, 2);
    assert_eq!(snapshot.get("node2").unwrap().frames_processed, 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test observability_collector_tests`
Expected: FAIL with "no `MetricsCollector` in `observability`"

**Step 3: Implement MetricsCollector**

In `src/observability/mod.rs`, add:
```rust
pub mod collector;

pub use collector::MetricsCollector;
```

Create `src/observability/collector.rs`:
```rust
use std::collections::HashMap;
use std::sync::Arc;
use super::NodeMetrics;

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub node_id: String,
    pub frames_processed: u64,
    pub errors_count: u64,
    pub avg_latency_us: u64,
}

pub struct MetricsCollector {
    metrics: HashMap<String, Arc<NodeMetrics>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    pub fn register(&mut self, node_id: impl Into<String>, metrics: Arc<NodeMetrics>) {
        self.metrics.insert(node_id.into(), metrics);
    }

    pub fn snapshot(&self) -> HashMap<String, MetricsSnapshot> {
        self.metrics
            .iter()
            .map(|(id, metrics)| {
                (
                    id.clone(),
                    MetricsSnapshot {
                        node_id: metrics.node_id().to_string(),
                        frames_processed: metrics.frames_processed(),
                        errors_count: metrics.errors_count(),
                        avg_latency_us: metrics.avg_latency_us(),
                    },
                )
            })
            .collect()
    }

    pub fn get_node_metrics(&self, node_id: &str) -> Option<Arc<NodeMetrics>> {
        self.metrics.get(node_id).cloned()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test observability_collector_tests`
Expected: PASS (all 2 tests)

**Step 5: Commit**

```bash
git add src/observability/ tests/observability_collector_tests.rs
git commit -m "feat(observability): add MetricsCollector for aggregation

- Central registry for all node metrics
- Snapshot API for current state
- HashMap-based storage with Arc sharing

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Create PipelineMonitor

**Files:**
- Create: `src/observability/monitor.rs`
- Create: `tests/observability_monitor_tests.rs`
- Modify: `src/observability/mod.rs`

**Step 1: Write failing test for PipelineMonitor**

```rust
use audiotab::observability::{NodeMetrics, MetricsCollector, PipelineMonitor};
use std::sync::Arc;

#[test]
fn test_monitor_report() {
    let mut collector = MetricsCollector::new();

    let m1 = Arc::new(NodeMetrics::new("gen"));
    let m2 = Arc::new(NodeMetrics::new("gain"));

    m1.record_frame_processed();
    m1.record_frame_processed();
    m2.record_frame_processed();
    m2.record_error();

    collector.register("gen", m1);
    collector.register("gain", m2);

    let monitor = PipelineMonitor::new(collector);
    let report = monitor.generate_report();

    assert!(report.contains("gen"));
    assert!(report.contains("gain"));
    assert!(report.contains("2 frames"));
    assert!(report.contains("1 error"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test observability_monitor_tests`
Expected: FAIL with "no `PipelineMonitor` in `observability`"

**Step 3: Implement PipelineMonitor**

In `src/observability/mod.rs`, add:
```rust
pub mod monitor;

pub use monitor::PipelineMonitor;
```

Create `src/observability/monitor.rs`:
```rust
use super::MetricsCollector;

pub struct PipelineMonitor {
    collector: MetricsCollector,
}

impl PipelineMonitor {
    pub fn new(collector: MetricsCollector) -> Self {
        Self { collector }
    }

    pub fn generate_report(&self) -> String {
        let snapshot = self.collector.snapshot();

        if snapshot.is_empty() {
            return "No nodes registered".to_string();
        }

        let mut report = String::from("=== Pipeline Metrics ===\n");

        for (node_id, metrics) in snapshot.iter() {
            report.push_str(&format!(
                "\n[{}]\n  Frames: {} processed\n  Errors: {}\n  Avg Latency: {}μs\n",
                node_id,
                metrics.frames_processed,
                if metrics.errors_count > 0 {
                    format!("{} error{}", metrics.errors_count, if metrics.errors_count == 1 { "" } else { "s" })
                } else {
                    "0 errors".to_string()
                },
                metrics.avg_latency_us
            ));
        }

        report
    }

    pub fn collector(&self) -> &MetricsCollector {
        &self.collector
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test observability_monitor_tests`
Expected: PASS

**Step 5: Commit**

```bash
git add src/observability/ tests/observability_monitor_tests.rs
git commit -m "feat(observability): add PipelineMonitor for reporting

- Human-readable metrics reporting
- Aggregates data from MetricsCollector
- Formats per-node statistics

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Define Error Policy Types

**Files:**
- Create: `src/resilience/mod.rs`
- Create: `src/resilience/policy.rs`
- Create: `tests/resilience_policy_tests.rs`
- Modify: `src/lib.rs`

**Step 1: Write failing test for ErrorPolicy**

```rust
use audiotab::resilience::ErrorPolicy;
use audiotab::core::DataFrame;

#[test]
fn test_error_policy_propagate() {
    let policy = ErrorPolicy::Propagate;

    match policy {
        ErrorPolicy::Propagate => { /* expected */ }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_error_policy_skip_frame() {
    let policy = ErrorPolicy::SkipFrame;

    match policy {
        ErrorPolicy::SkipFrame => { /* expected */ }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_error_policy_use_default() {
    let default_frame = DataFrame::new(0, 0);
    let policy = ErrorPolicy::UseDefault(default_frame.clone());

    match policy {
        ErrorPolicy::UseDefault(frame) => {
            assert_eq!(frame.timestamp, default_frame.timestamp);
        }
        _ => panic!("Wrong variant"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test resilience_policy_tests`
Expected: FAIL with "no `resilience` in the root"

**Step 3: Create resilience module with ErrorPolicy**

In `src/lib.rs`, add:
```rust
pub mod resilience;
```

Create `src/resilience/mod.rs`:
```rust
pub mod policy;

pub use policy::{ErrorPolicy, RestartStrategy};
```

Create `src/resilience/policy.rs`:
```rust
use crate::core::DataFrame;

#[derive(Debug, Clone)]
pub enum ErrorPolicy {
    /// Propagate error up (current behavior - stops pipeline)
    Propagate,

    /// Skip the errored frame and continue processing
    SkipFrame,

    /// Use a default/empty frame when error occurs
    UseDefault(DataFrame),
}

#[derive(Debug, Clone)]
pub enum RestartStrategy {
    /// Never restart node after error
    Never,

    /// Restart immediately on error
    Immediate,

    /// Exponential backoff restart
    Exponential {
        base_ms: u64,
        max_ms: u64,
        max_attempts: usize,
    },

    /// Circuit breaker pattern
    CircuitBreaker {
        error_threshold: usize,
        timeout_ms: u64,
    },
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test resilience_policy_tests`
Expected: PASS (all 3 tests)

**Step 5: Commit**

```bash
git add src/lib.rs src/resilience/ tests/resilience_policy_tests.rs
git commit -m "feat(resilience): add ErrorPolicy and RestartStrategy types

- ErrorPolicy: Propagate, SkipFrame, UseDefault
- RestartStrategy: Never, Immediate, Exponential, CircuitBreaker
- Foundation for error recovery system

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Implement ResilientNode Wrapper

**Files:**
- Create: `src/resilience/resilient_node.rs`
- Create: `tests/resilience_node_tests.rs`
- Modify: `src/resilience/mod.rs`

**Step 1: Write failing test for ResilientNode**

```rust
use audiotab::resilience::{ResilientNode, ErrorPolicy};
use audiotab::core::{ProcessingNode, DataFrame};
use audiotab::nodes::Gain;
use audiotab::observability::NodeMetrics;
use tokio::sync::mpsc;
use std::sync::Arc;
use anyhow::Result;

#[tokio::test]
async fn test_resilient_node_success() {
    let gain = Box::new(Gain::new());
    let mut gain_configured = gain;
    gain_configured.on_create(serde_json::json!({"gain": 2.0})).await.unwrap();

    let metrics = Arc::new(NodeMetrics::new("gain"));
    let resilient = ResilientNode::new(
        gain_configured,
        metrics.clone(),
        ErrorPolicy::Propagate,
    );

    let (tx_in, rx_in) = mpsc::channel(10);
    let (tx_out, mut rx_out) = mpsc::channel(10);

    // Send test frame
    let mut frame = DataFrame::new(0, 0);
    frame.payload.insert("main_channel".to_string(), vec![1.0, 2.0]);
    tx_in.send(frame).await.unwrap();
    drop(tx_in);

    // Run resilient node
    tokio::spawn(async move {
        resilient.run(rx_in, tx_out).await.unwrap();
    });

    // Verify output
    let output = rx_out.recv().await.unwrap();
    assert_eq!(output.payload.get("main_channel").unwrap(), &vec![2.0, 4.0]);

    // Verify metrics
    assert_eq!(metrics.frames_processed(), 1);
    assert_eq!(metrics.errors_count(), 0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test resilience_node_tests`
Expected: FAIL with "no `ResilientNode` in `resilience`"

**Step 3: Implement ResilientNode wrapper**

In `src/resilience/mod.rs`, add:
```rust
pub mod resilient_node;

pub use resilient_node::ResilientNode;
```

Create `src/resilience/resilient_node.rs`:
```rust
use crate::core::{ProcessingNode, DataFrame};
use crate::observability::NodeMetrics;
use super::ErrorPolicy;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct ResilientNode {
    inner: Box<dyn ProcessingNode>,
    metrics: Arc<NodeMetrics>,
    error_policy: ErrorPolicy,
}

impl ResilientNode {
    pub fn new(
        inner: Box<dyn ProcessingNode>,
        metrics: Arc<NodeMetrics>,
        error_policy: ErrorPolicy,
    ) -> Self {
        Self {
            inner,
            metrics,
            error_policy,
        }
    }
}

#[async_trait]
impl ProcessingNode for ResilientNode {
    async fn on_create(&mut self, config: Value) -> Result<()> {
        self.inner.on_create(config).await
    }

    async fn run(
        &self,
        mut rx: mpsc::Receiver<DataFrame>,
        tx: mpsc::Sender<DataFrame>,
    ) -> Result<()> {
        while let Some(frame) = rx.recv().await {
            let start = self.metrics.start_processing();

            // Create temp channels for inner node
            let (inner_tx, mut inner_rx) = mpsc::channel(1);
            let (temp_tx, temp_rx) = mpsc::channel(1);

            // Send frame to inner node
            if temp_tx.send(frame.clone()).await.is_err() {
                break;
            }
            drop(temp_tx);

            // Process through inner node
            let inner = &self.inner;
            let process_result = tokio::spawn(async move {
                inner.run(temp_rx, inner_tx).await
            }).await;

            match process_result {
                Ok(Ok(())) => {
                    // Success - forward output
                    if let Some(output) = inner_rx.recv().await {
                        self.metrics.finish_processing(start);
                        self.metrics.record_frame_processed();

                        if tx.send(output).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    // Error occurred
                    self.metrics.record_error();

                    match &self.error_policy {
                        ErrorPolicy::Propagate => {
                            return Err(anyhow::anyhow!("Node error"));
                        }
                        ErrorPolicy::SkipFrame => {
                            // Just skip this frame
                            continue;
                        }
                        ErrorPolicy::UseDefault(default_frame) => {
                            if tx.send(default_frame.clone()).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test resilience_node_tests`
Expected: PASS

**Step 5: Commit**

```bash
git add src/resilience/ tests/resilience_node_tests.rs
git commit -m "feat(resilience): implement ResilientNode wrapper

- Wraps ProcessingNode with error handling
- Tracks metrics (latency, frames, errors)
- Applies ErrorPolicy on failures
- Composition pattern - no changes to existing nodes

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: Create BufferPool System

**Files:**
- Create: `src/buffers/mod.rs`
- Create: `src/buffers/pool.rs`
- Create: `tests/buffers_pool_tests.rs`
- Modify: `src/lib.rs`

**Step 1: Write failing test for BufferPool**

```rust
use audiotab::buffers::BufferPool;

#[test]
fn test_buffer_pool_get_and_return() {
    let pool = BufferPool::new(1024);

    let buffer1 = pool.get();
    assert!(buffer1.capacity() >= 1024);

    drop(buffer1); // Returns to pool

    let buffer2 = pool.get();
    // Should reuse the buffer
    assert!(buffer2.capacity() >= 1024);
}

#[test]
fn test_buffer_pool_multiple_buffers() {
    let pool = BufferPool::new(512);

    let buf1 = pool.get();
    let buf2 = pool.get();
    let buf3 = pool.get();

    // All should have correct capacity
    assert!(buf1.capacity() >= 512);
    assert!(buf2.capacity() >= 512);
    assert!(buf3.capacity() >= 512);

    drop(buf1);
    drop(buf2);
    drop(buf3);

    // Pool should have 3 buffers available
    let buf4 = pool.get();
    assert!(buf4.capacity() >= 512);
}

#[test]
fn test_buffer_pool_concurrent() {
    use std::sync::Arc;
    use std::thread;

    let pool = Arc::new(BufferPool::new(256));
    let mut handles = vec![];

    for _ in 0..10 {
        let pool_clone = pool.clone();
        let handle = thread::spawn(move || {
            let _buffer = pool_clone.get();
            thread::sleep(std::time::Duration::from_millis(10));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test buffers_pool_tests`
Expected: FAIL with "no `buffers` in the root"

**Step 3: Implement BufferPool**

In `src/lib.rs`, add:
```rust
pub mod buffers;
```

Create `src/buffers/mod.rs`:
```rust
pub mod pool;

pub use pool::{BufferPool, PooledBuffer};
```

Create `src/buffers/pool.rs`:
```rust
use std::sync::{Arc, Mutex};

pub struct BufferPool {
    buffers: Arc<Mutex<Vec<Vec<f64>>>>,
    capacity: usize,
}

impl BufferPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(Vec::new())),
            capacity,
        }
    }

    pub fn get(&self) -> PooledBuffer {
        let mut buffers = self.buffers.lock().unwrap();

        let buffer = buffers.pop().unwrap_or_else(|| {
            Vec::with_capacity(self.capacity)
        });

        PooledBuffer {
            buffer: Some(buffer),
            pool: self.buffers.clone(),
        }
    }

    pub fn pool_size(&self) -> usize {
        self.buffers.lock().unwrap().len()
    }
}

impl Clone for BufferPool {
    fn clone(&self) -> Self {
        Self {
            buffers: self.buffers.clone(),
            capacity: self.capacity,
        }
    }
}

pub struct PooledBuffer {
    buffer: Option<Vec<f64>>,
    pool: Arc<Mutex<Vec<Vec<f64>>>>,
}

impl PooledBuffer {
    pub fn capacity(&self) -> usize {
        self.buffer.as_ref().map(|b| b.capacity()).unwrap_or(0)
    }

    pub fn as_slice(&self) -> &[f64] {
        self.buffer.as_ref().map(|b| b.as_slice()).unwrap_or(&[])
    }

    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        self.buffer.as_mut().map(|b| b.as_mut_slice()).unwrap_or(&mut [])
    }

    pub fn push(&mut self, value: f64) {
        if let Some(buffer) = &mut self.buffer {
            buffer.push(value);
        }
    }

    pub fn clear(&mut self) {
        if let Some(buffer) = &mut self.buffer {
            buffer.clear();
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.as_ref().map(|b| b.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(mut buffer) = self.buffer.take() {
            buffer.clear();
            let mut pool = self.pool.lock().unwrap();
            pool.push(buffer);
        }
    }
}

impl std::ops::Deref for PooledBuffer {
    type Target = Vec<f64>;

    fn deref(&self) -> &Self::Target {
        self.buffer.as_ref().unwrap()
    }
}

impl std::ops::DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer.as_mut().unwrap()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test buffers_pool_tests`
Expected: PASS (all 3 tests)

**Step 5: Commit**

```bash
git add src/lib.rs src/buffers/ tests/buffers_pool_tests.rs
git commit -m "feat(buffers): implement BufferPool for memory reuse

- Lock-based buffer pool with automatic return on drop
- PooledBuffer wrapper with Deref for Vec<f64> API
- Thread-safe with Mutex, reduces allocations
- First step toward zero-copy architecture

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: Refactor DataFrame for Zero-Copy

**Files:**
- Modify: `src/core/dataframe.rs`
- Modify: `tests/core_dataframe_tests.rs`
- Modify: `src/nodes/sine_generator.rs`
- Modify: `src/nodes/gain.rs`
- Modify: `src/nodes/print.rs`

**Step 1: Write failing test for Arc-based DataFrame**

Add to `tests/core_dataframe_tests.rs`:
```rust
use std::sync::Arc;

#[test]
fn test_dataframe_zero_copy_clone() {
    let mut frame = DataFrame::new(1000, 1);
    frame.payload.insert("channel".to_string(), Arc::new(vec![1.0, 2.0, 3.0]));

    let cloned = frame.clone();

    // Both should share the same Arc
    assert_eq!(
        Arc::strong_count(frame.payload.get("channel").unwrap()),
        2
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_dataframe_zero_copy_clone`
Expected: FAIL (type mismatch - payload is Vec<f64> not Arc<Vec<f64>>)

**Step 3: Refactor DataFrame to use Arc<Vec<f64>>**

In `src/core/dataframe.rs`, change:
```rust
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DataFrame {
    pub timestamp: u64,
    pub sequence_id: u64,
    pub payload: HashMap<String, Arc<Vec<f64>>>,
    pub metadata: HashMap<String, String>,
}

impl DataFrame {
    pub fn new(timestamp: u64, sequence_id: u64) -> Self {
        Self {
            timestamp,
            sequence_id,
            payload: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}
```

**Step 4: Update all nodes to use Arc<Vec<f64>>**

In `src/nodes/sine_generator.rs`, update `run()`:
```rust
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

        phase %= 2.0 * PI;

        frame.payload.insert("main_channel".to_string(), Arc::new(samples));

        if tx.send(frame).await.is_err() {
            break;
        }
    }

    Ok(())
}
```

In `src/nodes/gain.rs`, update `run()`:
```rust
async fn run(
    &self,
    mut rx: mpsc::Receiver<DataFrame>,
    tx: mpsc::Sender<DataFrame>,
) -> Result<()> {
    while let Some(mut frame) = rx.recv().await {
        if let Some(data) = frame.payload.get("main_channel") {
            // Clone Arc data, apply gain, wrap in new Arc
            let mut amplified: Vec<f64> = data.iter().map(|&x| x * self.gain).collect();
            frame.payload.insert("main_channel".to_string(), Arc::new(amplified));
        }

        if tx.send(frame).await.is_err() {
            break;
        }
    }

    Ok(())
}
```

In `src/nodes/print.rs`, update `run()`:
```rust
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
            break;
        }
    }

    Ok(())
}
```

**Step 5: Run all tests to verify they pass**

Run: `cargo test`
Expected: PASS (all existing tests + new zero-copy test)

**Step 6: Commit**

```bash
git add src/core/dataframe.rs src/nodes/*.rs tests/core_dataframe_tests.rs
git commit -m "refactor(core): use Arc<Vec<f64>> for zero-copy DataFrame

- Change DataFrame payload to Arc<Vec<f64>> for reference counting
- Cloning DataFrame now shares data instead of copying
- Update all nodes to work with Arc-wrapped data
- Reduces allocations in fanout scenarios

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: Integrate Observability into AsyncPipeline

**Files:**
- Modify: `src/engine/async_pipeline.rs`
- Create: `tests/engine_observability_integration_tests.rs`

**Step 1: Write failing integration test**

```rust
use audiotab::engine::AsyncPipeline;
use audiotab::core::DataFrame;
use audiotab::observability::PipelineMonitor;

#[tokio::test]
async fn test_pipeline_with_metrics() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
            {"id": "print", "type": "Print", "config": {"label": "Test"}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "print"}
        ]
    });

    let mut pipeline = AsyncPipeline::from_json(config).await.unwrap();
    pipeline.start().await.unwrap();

    // Trigger some frames
    for i in 0..5 {
        pipeline.trigger(DataFrame::new(i * 100, i)).await.unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Get metrics
    let monitor = pipeline.get_monitor().unwrap();
    let report = monitor.generate_report();

    assert!(report.contains("gen"));
    assert!(report.contains("gain"));
    assert!(report.contains("print"));

    pipeline.stop().await.unwrap();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test engine_observability_integration_tests`
Expected: FAIL (AsyncPipeline doesn't have `get_monitor()` method)

**Step 3: Add metrics collection to AsyncPipeline**

In `src/engine/async_pipeline.rs`, modify struct and implementation:

```rust
use crate::observability::{NodeMetrics, MetricsCollector, PipelineMonitor};
use crate::resilience::{ResilientNode, ErrorPolicy};
use std::sync::Arc;

pub struct AsyncPipeline {
    nodes: HashMap<String, Box<dyn ProcessingNode>>,
    connections: Vec<(String, String)>,
    channels: HashMap<String, mpsc::Sender<DataFrame>>,
    handles: Vec<JoinHandle<Result<()>>>,
    source_node_id: Option<String>,
    channel_capacity: usize,
    metrics_collector: Option<MetricsCollector>,
}

impl AsyncPipeline {
    pub async fn from_json(config: Value) -> Result<Self> {
        // ... existing parsing ...

        Ok(Self {
            nodes,
            connections,
            channels: HashMap::new(),
            handles: Vec::new(),
            source_node_id,
            channel_capacity,
            metrics_collector: Some(MetricsCollector::new()),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let channel_capacity = self.channel_capacity;
        let mut node_channels: HashMap<String, (mpsc::Sender<DataFrame>, mpsc::Receiver<DataFrame>)> = HashMap::new();

        // Create channels for each node
        for node_id in self.nodes.keys() {
            let (tx, rx) = mpsc::channel(channel_capacity);
            node_channels.insert(node_id.clone(), (tx, rx));
        }

        // Save source node channel before spawning
        if let Some(source_id) = &self.source_node_id {
            if let Some((tx, _)) = node_channels.get(source_id) {
                self.channels.insert(source_id.clone(), tx.clone());
            }
        }

        // Build output channel map
        let mut output_channels: HashMap<String, Vec<mpsc::Sender<DataFrame>>> = HashMap::new();
        for (from, to) in &self.connections {
            output_channels
                .entry(from.clone())
                .or_insert_with(Vec::new)
                .push(node_channels.get(to).unwrap().0.clone());
        }

        // Wrap nodes with ResilientNode and metrics
        let mut collector = self.metrics_collector.take().unwrap();

        for (node_id, node) in self.nodes.drain() {
            let (_tx, rx) = node_channels.remove(&node_id).unwrap();
            let outputs = output_channels.remove(&node_id).unwrap_or_default();

            // Create metrics for this node
            let metrics = Arc::new(NodeMetrics::new(&node_id));
            collector.register(&node_id, metrics.clone());

            // Wrap with ResilientNode
            let resilient = ResilientNode::new(node, metrics, ErrorPolicy::Propagate);

            let handle = tokio::spawn(async move {
                let (fanout_tx, mut fanout_rx) = mpsc::channel(channel_capacity);

                // Spawn node processing
                let node_task = tokio::spawn(async move {
                    resilient.run(rx, fanout_tx).await
                });

                // Spawn fanout
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

        self.metrics_collector = Some(collector);
        Ok(())
    }

    pub fn get_monitor(&self) -> Option<PipelineMonitor> {
        self.metrics_collector.as_ref().map(|c| PipelineMonitor::new(c.clone()))
    }

    // ... rest of existing methods ...
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test engine_observability_integration_tests`
Expected: PASS

**Step 5: Commit**

```bash
git add src/engine/async_pipeline.rs tests/engine_observability_integration_tests.rs
git commit -m "feat(engine): integrate observability into AsyncPipeline

- Wrap all nodes with ResilientNode for metrics tracking
- Add MetricsCollector to AsyncPipeline
- Expose PipelineMonitor via get_monitor() API
- Automatic metrics collection for all pipeline nodes

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 9: Create Production Readiness Demo

**Files:**
- Create: `src/bin/production_demo.rs`

**Step 1: Write production demo binary**

Create `src/bin/production_demo.rs`:
```rust
use audiotab::engine::AsyncPipeline;
use audiotab::core::DataFrame;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("StreamLab Core - Phase 3 Production Readiness Demo");
    println!("=================================================\n");

    // Demo 1: Pipeline with metrics monitoring
    println!("=== Demo 1: Observability & Metrics ===");
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
                    "label": "Production Output"
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

    println!("Processing 10 frames with metrics tracking...\n");
    for i in 0..10 {
        pipeline.trigger(DataFrame::new(i * 1000, i)).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Display metrics
    if let Some(monitor) = pipeline.get_monitor() {
        println!("\n{}", monitor.generate_report());
    }

    pipeline.stop().await?;

    println!("\n=== Phase 3 Features Demonstrated ===");
    println!("✓ NodeMetrics with atomic counters");
    println!("✓ MetricsCollector for aggregation");
    println!("✓ PipelineMonitor for human-readable reports");
    println!("✓ ResilientNode wrapper (error handling ready)");
    println!("✓ BufferPool system (memory optimization ready)");
    println!("✓ Zero-copy DataFrame with Arc<Vec<f64>>");
    println!("\n=== Production Readiness Complete! ===");

    Ok(())
}
```

**Step 2: Build and run demo**

Run: `cargo build --bin production_demo`
Expected: SUCCESS

Run: `cargo run --bin production_demo`
Expected: Output showing metrics report with frame counts and latencies

**Step 3: Commit**

```bash
git add src/bin/production_demo.rs
git commit -m "feat(demo): add Phase 3 production readiness demo

- Demonstrates observability with metrics tracking
- Shows PipelineMonitor reporting
- Highlights all Phase 3 features
- Working example of production-ready pipeline

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 10: Update Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md`

**Step 1: Update README.md with Phase 3 features**

Add Phase 3 section after Phase 2:
```markdown
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
```

**Step 2: Update docs/architecture.md with Phase 3 details**

Add Phase 3 section:
```markdown
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
```

**Step 3: Run all tests to verify everything works**

Run: `cargo test`
Expected: PASS (all tests including Phase 3 features)

**Step 4: Commit documentation**

```bash
git add README.md docs/architecture.md
git commit -m "docs: update documentation for Phase 3 production features

- Add Phase 3 section to README with observability features
- Document error recovery system (ErrorPolicy, RestartStrategy)
- Explain resource management (BufferPool, zero-copy DataFrame)
- Update architecture.md with detailed Phase 3 design
- Document limitations and future work

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Summary

**Phase 3 Complete: 10 Tasks**

1. ✓ NodeMetrics foundation (atomic counters)
2. ✓ MetricsCollector (aggregation)
3. ✓ PipelineMonitor (reporting)
4. ✓ ErrorPolicy types (Propagate, SkipFrame, UseDefault)
5. ✓ ResilientNode wrapper (error handling)
6. ✓ BufferPool system (memory reuse)
7. ✓ DataFrame Arc refactor (zero-copy)
8. ✓ AsyncPipeline integration (observability)
9. ✓ Production demo binary
10. ✓ Documentation updates

**Production Capabilities:**
- ✓ Observability: Real-time metrics and monitoring
- ✓ Error Recovery: Configurable failure policies
- ✓ Resource Efficiency: Buffer pooling and zero-copy
- ✓ Backward Compatible: All Phase 1/2 features intact

**Tech Stack:**
- tokio for async runtime
- std::sync::atomic for lock-free counters
- Arc/Mutex for thread-safe sharing
- Composition pattern for extensibility

**Next Phase (4):**
- Real-time scheduling
- Dynamic reconfiguration
- Hardware abstraction layer
- Advanced buffer management
