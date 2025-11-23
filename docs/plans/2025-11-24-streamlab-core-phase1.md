# StreamLab Core - Phase 1: Core Engine Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the foundational Rust backend capable of parsing JSON configuration and executing a simple streaming pipeline: "Sine Wave Generator -> Gain Processor -> Console Output".

**Architecture:** Actor-based async runtime using Tokio. Each processing node is a separate async task communicating via bounded MPSC channels (backpressure). DataFrame is the universal data container passed between nodes. Pipeline lifecycle managed by an Orchestrator that spawns node tasks from JSON graph definitions.

**Tech Stack:** Rust 2024 edition, tokio (async runtime), serde_json (config parsing), async-trait (trait support), anyhow (error handling)

---

## Task 1: Project Setup & Dependencies

**Files:**
- Modify: `Cargo.toml`
- Create: `.gitignore` (if needed)

**Step 1: Add required dependencies to Cargo.toml**

```toml
[package]
name = "audiotab"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
anyhow = "1.0"

[dev-dependencies]
```

**Step 2: Verify dependencies compile**

Run: `cargo build`
Expected: SUCCESS - dependencies downloaded and compiled

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add core dependencies (tokio, serde, async-trait, anyhow)"
```

---

## Task 2: Define DataFrame Structure

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/dataframe.rs`
- Modify: `src/main.rs`
- Create: `tests/core_dataframe_tests.rs`

**Step 1: Create module structure**

Create `src/core/mod.rs`:
```rust
pub mod dataframe;

pub use dataframe::DataFrame;
```

**Step 2: Write failing test for DataFrame creation**

Create `tests/core_dataframe_tests.rs`:
```rust
use audiotab::core::DataFrame;
use std::collections::HashMap;

#[test]
fn test_dataframe_creation() {
    let df = DataFrame::new(1000, 1);
    assert_eq!(df.timestamp, 1000);
    assert_eq!(df.sequence_id, 1);
    assert!(df.payload.is_empty());
    assert!(df.metadata.is_empty());
}

#[test]
fn test_dataframe_with_data() {
    let mut df = DataFrame::new(2000, 2);
    df.payload.insert("channel1".to_string(), vec![1.0, 2.0, 3.0]);

    assert_eq!(df.payload.get("channel1").unwrap(), &vec![1.0, 2.0, 3.0]);
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test test_dataframe`
Expected: FAIL - "DataFrame not found" or module errors

**Step 4: Implement DataFrame**

Create `src/core/dataframe.rs`:
```rust
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Basic data unit passed between processing nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFrame {
    /// Timestamp in microseconds since epoch
    pub timestamp: u64,

    /// Sequential frame number for ordering
    pub sequence_id: u64,

    /// Multi-channel data keyed by channel name
    pub payload: HashMap<String, Vec<f64>>,

    /// Side-channel information (gain, sample_rate, etc)
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

**Step 5: Export core module in main**

Modify `src/main.rs`:
```rust
pub mod core;

fn main() {
    println!("Hello, world!");
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test test_dataframe`
Expected: PASS - all 2 tests pass

**Step 7: Commit**

```bash
git add src/core/ src/main.rs tests/
git commit -m "feat(core): add DataFrame structure for inter-node communication"
```

---

## Task 3: Define ProcessingNode Trait

**Files:**
- Create: `src/core/node.rs`
- Modify: `src/core/mod.rs`
- Create: `tests/core_node_tests.rs`

**Step 1: Write failing test for ProcessingNode trait**

Create `tests/core_node_tests.rs`:
```rust
use audiotab::core::{DataFrame, ProcessingNode};
use anyhow::Result;
use async_trait::async_trait;

struct DummyNode {
    multiplier: f64,
}

#[async_trait]
impl ProcessingNode for DummyNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        self.multiplier = config["multiplier"].as_f64().unwrap_or(1.0);
        Ok(())
    }

    async fn process(&self, input: DataFrame) -> Result<DataFrame> {
        let mut output = input.clone();
        if let Some(data) = output.payload.get_mut("test") {
            for value in data.iter_mut() {
                *value *= self.multiplier;
            }
        }
        Ok(output)
    }
}

#[tokio::test]
async fn test_node_process() {
    let mut node = DummyNode { multiplier: 1.0 };
    let config = serde_json::json!({"multiplier": 2.0});

    node.on_create(config).await.unwrap();

    let mut df = DataFrame::new(0, 0);
    df.payload.insert("test".to_string(), vec![1.0, 2.0, 3.0]);

    let result = node.process(df).await.unwrap();
    assert_eq!(result.payload.get("test").unwrap(), &vec![2.0, 4.0, 6.0]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_node_process`
Expected: FAIL - "ProcessingNode not found"

**Step 3: Define ProcessingNode trait**

Create `src/core/node.rs`:
```rust
use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use super::DataFrame;

/// Base trait for all processing nodes in the pipeline
#[async_trait]
pub trait ProcessingNode: Send + Sync {
    /// Called once when node is instantiated with config from JSON
    async fn on_create(&mut self, config: Value) -> Result<()>;

    /// Process a single DataFrame and return result
    /// Can handle both data flow and control flow signals
    async fn process(&self, input: DataFrame) -> Result<DataFrame>;
}
```

**Step 4: Export node module**

Modify `src/core/mod.rs`:
```rust
pub mod dataframe;
pub mod node;

pub use dataframe::DataFrame;
pub use node::ProcessingNode;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test test_node_process`
Expected: PASS - test passes

**Step 6: Commit**

```bash
git add src/core/node.rs src/core/mod.rs tests/core_node_tests.rs
git commit -m "feat(core): add ProcessingNode trait for pipeline nodes"
```

---

## Task 4: Implement Sine Wave Generator Node

**Files:**
- Create: `src/nodes/mod.rs`
- Create: `src/nodes/sine_generator.rs`
- Modify: `src/main.rs`
- Create: `tests/nodes_sine_tests.rs`

**Step 1: Write failing test for SineGenerator**

Create `tests/nodes_sine_tests.rs`:
```rust
use audiotab::nodes::SineGenerator;
use audiotab::core::{ProcessingNode, DataFrame};

#[tokio::test]
async fn test_sine_generator_creates_data() {
    let mut gen = SineGenerator::new();
    let config = serde_json::json!({
        "frequency": 440.0,
        "sample_rate": 48000.0,
        "frame_size": 1024
    });

    gen.on_create(config).await.unwrap();

    let empty_frame = DataFrame::new(0, 0);
    let result = gen.process(empty_frame).await.unwrap();

    // Should create main_channel data
    assert!(result.payload.contains_key("main_channel"));
    assert_eq!(result.payload.get("main_channel").unwrap().len(), 1024);
}

#[tokio::test]
async fn test_sine_wave_values() {
    let mut gen = SineGenerator::new();
    let config = serde_json::json!({
        "frequency": 1.0,  // 1 Hz for easy verification
        "sample_rate": 8.0,  // 8 samples per second
        "frame_size": 8
    });

    gen.on_create(config).await.unwrap();

    let result = gen.process(DataFrame::new(0, 0)).await.unwrap();
    let data = result.payload.get("main_channel").unwrap();

    // At 1 Hz with 8 samples/sec, we should get one complete sine cycle
    // Samples at 0°, 45°, 90°, 135°, 180°, 225°, 270°, 315°
    assert!(data[0].abs() < 0.01);  // sin(0) ≈ 0
    assert!((data[2] - 1.0).abs() < 0.01);  // sin(90°) ≈ 1
    assert!(data[4].abs() < 0.01);  // sin(180°) ≈ 0
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_sine`
Expected: FAIL - "SineGenerator not found"

**Step 3: Implement SineGenerator**

Create `src/nodes/sine_generator.rs`:
```rust
use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use std::f64::consts::PI;
use crate::core::{ProcessingNode, DataFrame};

pub struct SineGenerator {
    frequency: f64,
    sample_rate: f64,
    frame_size: usize,
    phase: f64,  // Current phase for continuous generation
}

impl SineGenerator {
    pub fn new() -> Self {
        Self {
            frequency: 440.0,
            sample_rate: 48000.0,
            frame_size: 1024,
            phase: 0.0,
        }
    }
}

#[async_trait]
impl ProcessingNode for SineGenerator {
    async fn on_create(&mut self, config: Value) -> Result<()> {
        if let Some(freq) = config["frequency"].as_f64() {
            self.frequency = freq;
        }
        if let Some(sr) = config["sample_rate"].as_f64() {
            self.sample_rate = sr;
        }
        if let Some(size) = config["frame_size"].as_u64() {
            self.frame_size = size as usize;
        }
        Ok(())
    }

    async fn process(&self, mut input: DataFrame) -> Result<DataFrame> {
        let mut samples = Vec::with_capacity(self.frame_size);
        let phase_increment = 2.0 * PI * self.frequency / self.sample_rate;

        for i in 0..self.frame_size {
            let phase = self.phase + (i as f64) * phase_increment;
            samples.push(phase.sin());
        }

        input.payload.insert("main_channel".to_string(), samples);
        Ok(input)
    }
}
```

**Step 4: Create nodes module**

Create `src/nodes/mod.rs`:
```rust
pub mod sine_generator;

pub use sine_generator::SineGenerator;
```

**Step 5: Export nodes in main**

Modify `src/main.rs`:
```rust
pub mod core;
pub mod nodes;

fn main() {
    println!("Hello, world!");
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test test_sine`
Expected: PASS - all sine tests pass

**Step 7: Commit**

```bash
git add src/nodes/ src/main.rs tests/nodes_sine_tests.rs
git commit -m "feat(nodes): add SineGenerator node for test signal generation"
```

---

## Task 5: Implement Gain Processor Node

**Files:**
- Create: `src/nodes/gain.rs`
- Modify: `src/nodes/mod.rs`
- Create: `tests/nodes_gain_tests.rs`

**Step 1: Write failing test for Gain node**

Create `tests/nodes_gain_tests.rs`:
```rust
use audiotab::nodes::Gain;
use audiotab::core::{ProcessingNode, DataFrame};

#[tokio::test]
async fn test_gain_multiplication() {
    let mut gain = Gain::new();
    let config = serde_json::json!({"gain": 2.0});

    gain.on_create(config).await.unwrap();

    let mut df = DataFrame::new(0, 0);
    df.payload.insert("main_channel".to_string(), vec![1.0, 2.0, 3.0]);

    let result = gain.process(df).await.unwrap();
    assert_eq!(result.payload.get("main_channel").unwrap(), &vec![2.0, 4.0, 6.0]);
}

#[tokio::test]
async fn test_gain_attenuation() {
    let mut gain = Gain::new();
    let config = serde_json::json!({"gain": 0.5});

    gain.on_create(config).await.unwrap();

    let mut df = DataFrame::new(0, 0);
    df.payload.insert("main_channel".to_string(), vec![2.0, 4.0, 6.0]);

    let result = gain.process(df).await.unwrap();
    assert_eq!(result.payload.get("main_channel").unwrap(), &vec![1.0, 2.0, 3.0]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_gain`
Expected: FAIL - "Gain not found"

**Step 3: Implement Gain node**

Create `src/nodes/gain.rs`:
```rust
use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use crate::core::{ProcessingNode, DataFrame};

pub struct Gain {
    gain: f64,
}

impl Gain {
    pub fn new() -> Self {
        Self { gain: 1.0 }
    }
}

#[async_trait]
impl ProcessingNode for Gain {
    async fn on_create(&mut self, config: Value) -> Result<()> {
        if let Some(g) = config["gain"].as_f64() {
            self.gain = g;
        }
        Ok(())
    }

    async fn process(&self, mut input: DataFrame) -> Result<DataFrame> {
        // Apply gain to main_channel if it exists
        if let Some(data) = input.payload.get_mut("main_channel") {
            for sample in data.iter_mut() {
                *sample *= self.gain;
            }
        }
        Ok(input)
    }
}
```

**Step 4: Export Gain in nodes module**

Modify `src/nodes/mod.rs`:
```rust
pub mod sine_generator;
pub mod gain;

pub use sine_generator::SineGenerator;
pub use gain::Gain;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test test_gain`
Expected: PASS - all gain tests pass

**Step 6: Commit**

```bash
git add src/nodes/gain.rs src/nodes/mod.rs tests/nodes_gain_tests.rs
git commit -m "feat(nodes): add Gain node for signal amplification/attenuation"
```

---

## Task 6: Implement Print/Console Output Node

**Files:**
- Create: `src/nodes/print.rs`
- Modify: `src/nodes/mod.rs`
- Create: `tests/nodes_print_tests.rs`

**Step 1: Write test for Print node**

Create `tests/nodes_print_tests.rs`:
```rust
use audiotab::nodes::Print;
use audiotab::core::{ProcessingNode, DataFrame};

#[tokio::test]
async fn test_print_passthrough() {
    let mut print = Print::new();
    let config = serde_json::json!({"label": "Test"});

    print.on_create(config).await.unwrap();

    let mut df = DataFrame::new(1000, 1);
    df.payload.insert("main_channel".to_string(), vec![1.0, 2.0, 3.0]);

    let result = print.process(df.clone()).await.unwrap();

    // Print should pass through unchanged
    assert_eq!(result.timestamp, df.timestamp);
    assert_eq!(result.sequence_id, df.sequence_id);
    assert_eq!(result.payload, df.payload);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_print`
Expected: FAIL - "Print not found"

**Step 3: Implement Print node**

Create `src/nodes/print.rs`:
```rust
use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use crate::core::{ProcessingNode, DataFrame};

pub struct Print {
    label: String,
}

impl Print {
    pub fn new() -> Self {
        Self {
            label: "Output".to_string(),
        }
    }
}

#[async_trait]
impl ProcessingNode for Print {
    async fn on_create(&mut self, config: Value) -> Result<()> {
        if let Some(label) = config["label"].as_str() {
            self.label = label.to_string();
        }
        Ok(())
    }

    async fn process(&self, input: DataFrame) -> Result<DataFrame> {
        println!("[{}] Frame #{} @ {}μs", self.label, input.sequence_id, input.timestamp);

        for (channel, data) in &input.payload {
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

        Ok(input)
    }
}
```

**Step 4: Export Print in nodes module**

Modify `src/nodes/mod.rs`:
```rust
pub mod sine_generator;
pub mod gain;
pub mod print;

pub use sine_generator::SineGenerator;
pub use gain::Gain;
pub use print::Print;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test test_print`
Expected: PASS - test passes

**Step 6: Commit**

```bash
git add src/nodes/print.rs src/nodes/mod.rs tests/nodes_print_tests.rs
git commit -m "feat(nodes): add Print node for console output debugging"
```

---

## Task 7: Implement Pipeline Builder (Basic)

**Files:**
- Create: `src/engine/mod.rs`
- Create: `src/engine/pipeline.rs`
- Modify: `src/main.rs`
- Create: `tests/engine_pipeline_tests.rs`

**Step 1: Write failing test for Pipeline**

Create `tests/engine_pipeline_tests.rs`:
```rust
use audiotab::engine::Pipeline;
use audiotab::core::DataFrame;

#[tokio::test]
async fn test_pipeline_creation() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0}},
            {"id": "gain", "type": "Gain", "config": {"gain": 2.0}},
            {"id": "print", "type": "Print", "config": {"label": "Output"}}
        ],
        "connections": [
            {"from": "gen", "to": "gain"},
            {"from": "gain", "to": "print"}
        ]
    });

    let pipeline = Pipeline::from_json(config).await;
    assert!(pipeline.is_ok());
}

#[tokio::test]
async fn test_pipeline_execute() {
    let config = serde_json::json!({
        "nodes": [
            {"id": "gen", "type": "SineGenerator", "config": {"frequency": 440.0, "frame_size": 100}},
            {"id": "print", "type": "Print", "config": {"label": "Test"}}
        ],
        "connections": [
            {"from": "gen", "to": "print"}
        ]
    });

    let mut pipeline = Pipeline::from_json(config).await.unwrap();

    // Trigger one execution
    let result = pipeline.execute_once().await;
    assert!(result.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_pipeline`
Expected: FAIL - "Pipeline not found"

**Step 3: Implement basic Pipeline structure**

Create `src/engine/pipeline.rs`:
```rust
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::core::{ProcessingNode, DataFrame};
use crate::nodes::{SineGenerator, Gain, Print};

pub struct Pipeline {
    nodes: HashMap<String, Box<dyn ProcessingNode>>,
    connections: Vec<(String, String)>,
}

impl Pipeline {
    pub async fn from_json(config: Value) -> Result<Self> {
        let mut nodes: HashMap<String, Box<dyn ProcessingNode>> = HashMap::new();
        let mut connections = Vec::new();

        // Parse nodes
        if let Some(nodes_array) = config["nodes"].as_array() {
            for node_config in nodes_array {
                let id = node_config["id"].as_str()
                    .ok_or(anyhow!("Node missing id"))?
                    .to_string();
                let node_type = node_config["type"].as_str()
                    .ok_or(anyhow!("Node missing type"))?;
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
                let from = conn["from"].as_str()
                    .ok_or(anyhow!("Connection missing from"))?
                    .to_string();
                let to = conn["to"].as_str()
                    .ok_or(anyhow!("Connection missing to"))?
                    .to_string();
                connections.push((from, to));
            }
        }

        Ok(Self { nodes, connections })
    }

    pub async fn execute_once(&mut self) -> Result<()> {
        // Simple linear execution for now (no parallelism)
        // Start with empty frame
        let mut current_frame = DataFrame::new(0, 0);

        // Build execution order (simple topological sort for linear pipeline)
        let mut executed = std::collections::HashSet::new();
        let mut execution_order = Vec::new();

        // Find source node (no incoming connections)
        for (id, _) in &self.nodes {
            let has_incoming = self.connections.iter()
                .any(|(_, to)| to == id);
            if !has_incoming {
                execution_order.push(id.clone());
                executed.insert(id.clone());
                break;
            }
        }

        // Follow connections to build order
        while execution_order.len() < self.nodes.len() {
            let last = execution_order.last().unwrap();
            if let Some((_, next)) = self.connections.iter()
                .find(|(from, _)| from == last) {
                if !executed.contains(next) {
                    execution_order.push(next.clone());
                    executed.insert(next.clone());
                }
            } else {
                break;
            }
        }

        // Execute in order
        for node_id in execution_order {
            if let Some(node) = self.nodes.get(&node_id) {
                current_frame = node.process(current_frame).await?;
            }
        }

        Ok(())
    }
}
```

**Step 4: Create engine module**

Create `src/engine/mod.rs`:
```rust
pub mod pipeline;

pub use pipeline::Pipeline;
```

**Step 5: Export engine in main**

Modify `src/main.rs`:
```rust
pub mod core;
pub mod nodes;
pub mod engine;

fn main() {
    println!("Hello, world!");
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test test_pipeline`
Expected: PASS - all pipeline tests pass

**Step 7: Commit**

```bash
git add src/engine/ src/main.rs tests/engine_pipeline_tests.rs
git commit -m "feat(engine): add basic Pipeline builder and linear executor"
```

---

## Task 8: Create Demo Main Function

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement demo main**

Modify `src/main.rs`:
```rust
pub mod core;
pub mod nodes;
pub mod engine;

use engine::Pipeline;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("StreamLab Core - Phase 1 Demo");
    println!("================================\n");

    // Define a simple pipeline: SineWave -> Gain -> Print
    let config = serde_json::json!({
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
                    "label": "Final Output"
                }
            }
        ],
        "connections": [
            {"from": "sine_gen", "to": "amplifier"},
            {"from": "amplifier", "to": "console_out"}
        ]
    });

    println!("Building pipeline from config...");
    let mut pipeline = Pipeline::from_json(config).await?;

    println!("Executing pipeline 3 times...\n");
    for i in 0..3 {
        println!("--- Execution {} ---", i + 1);
        pipeline.execute_once().await?;
        println!();
    }

    println!("Demo complete! Phase 1 objectives achieved:");
    println!("✓ DataFrame structure defined");
    println!("✓ ProcessingNode trait implemented");
    println!("✓ Three basic nodes: SineGenerator, Gain, Print");
    println!("✓ Pipeline builder parses JSON configuration");
    println!("✓ Linear pipeline execution works");

    Ok(())
}
```

**Step 2: Run the demo**

Run: `cargo run`
Expected: SUCCESS - Should print 3 executions with frame statistics

**Step 3: Run all tests**

Run: `cargo test`
Expected: PASS - all tests pass

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(demo): add Phase 1 demo showing Sine->Gain->Print pipeline"
```

---

## Task 9: Add Documentation

**Files:**
- Create: `README.md`
- Create: `docs/architecture.md`

**Step 1: Create README**

Create `README.md`:
```markdown
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

### Quick Start

```bash
# Run demo
cargo run

# Run tests
cargo test

# Example JSON config
{
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

### Next Steps

- [ ] Async multi-node execution with tokio channels
- [ ] Backpressure mechanism
- [ ] Concurrent pipeline instances (PipelinePool)
- [ ] HAL interfaces for real hardware
```

**Step 2: Create architecture doc**

Create `docs/architecture.md`:
```markdown
# StreamLab Architecture

## Data Flow

```
JSON Config → Pipeline Builder → Node Graph → Async Execution
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
async fn process(&self, input: DataFrame) -> Result<DataFrame>
```

### Pipeline
- Parses JSON graph definition
- Instantiates nodes with configuration
- Manages execution order (currently linear, will be async graph)

## Phase 1 Limitations

- Linear execution only (no parallelism)
- Synchronous node processing (blocking)
- No backpressure
- No concurrent pipeline instances

These will be addressed in subsequent phases.
```

**Step 3: Commit**

```bash
git add README.md docs/architecture.md
git commit -m "docs: add README and architecture documentation"
```

---

## Acceptance Criteria

### Functional Tests

Run: `cargo test`
Expected: All tests PASS (minimum 8 tests)

### Demo Execution

Run: `cargo run`
Expected output should show:
1. Pipeline building successfully
2. Three executions printing frame statistics
3. RMS values showing gain applied (should be 2.5x higher than baseline)
4. Completion message with checkmarks

### Code Quality

- [ ] All files compile without warnings: `cargo build --all-targets`
- [ ] All tests pass: `cargo test`
- [ ] Code formatted: `cargo fmt --check`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`

---

## Next Phase Preview

**Phase 2** will add:
- Async node execution with `tokio::spawn`
- MPSC channels between nodes for true streaming
- Backpressure via bounded channels
- PipelinePool for concurrent instances
- Integration test: 10 simultaneous pipeline executions

Estimated: 15-20 tasks, 3-4 hours total

---

## Notes for Executor

- This plan assumes TDD (Test-Driven Development)
- Each test MUST be run and verified to fail before implementation
- Each implementation MUST be tested to verify it passes
- Commit frequently (after each task completion)
- If a test passes on first run (before implementation), investigate why
- Follow DRY principle - avoid code duplication
- Follow YAGNI - don't add features not in the spec
- When stuck, refer to Rust async programming docs and tokio examples
