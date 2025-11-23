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
