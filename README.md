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
