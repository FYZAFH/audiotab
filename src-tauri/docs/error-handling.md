# Error Handling Patterns

## Pipeline Deployment Errors

### Graph Translation Errors
**Causes:**
- Missing nodes/edges arrays in frontend JSON
- Invalid JSON structure
- Malformed node/edge data

**Action:**
- Return error to frontend via Result<String, String>
- Emit error event via `pipeline-status` with state "Error"
- Include descriptive error message in event payload

**Example:**
```rust
let backend_json = match translate_graph(frontend_json) {
    Ok(json) => json,
    Err(e) => {
        let error_msg = format!("Graph translation failed: {}", e);
        let _ = app.emit("pipeline-status", PipelineStatusEvent {
            id: pipeline_id.clone(),
            state: "Error".to_string(),
            error: Some(error_msg.clone()),
        });
        return Err(error_msg);
    }
};
```

### Pipeline Creation Errors
**Causes:**
- Unknown node type (node not registered)
- Invalid node configuration (missing required parameters)
- Invalid connections (disconnected nodes, cycles)
- Node initialization failures (on_create errors)

**Action:**
- Return error to frontend
- Emit error event with detailed message
- Do NOT store pipeline in AppState (creation failed)

**Example:**
```rust
let pipeline = match AsyncPipeline::from_json(backend_json).await {
    Ok(p) => p,
    Err(e) => {
        let error_msg = format!("Pipeline creation failed: {}", e);
        let _ = app.emit("pipeline-status", PipelineStatusEvent {
            id: pipeline_id.clone(),
            state: "Error".to_string(),
            error: Some(error_msg.clone()),
        });
        return Err(error_msg);
    }
};
```

### Execution Errors
**Causes:**
- Kernel not running
- Kernel not initialized
- Device not available
- Hardware access denied

**Action:**
- Return error to frontend
- Keep pipeline in Created/Idle state (pipeline exists but not running)
- User can retry execution after fixing kernel/device issues

**Example:**
```rust
kernel_manager.execute_pipeline(pipeline_arc.clone())
    .map_err(|e| format!("Failed to execute pipeline: {}", e))?;
```

## Runtime Errors

### Node Processing Errors
**Causes:**
- Async task panic (unexpected failure)
- Data processing failure (invalid input)
- Resource exhaustion (out of memory)
- Downstream channel closed

**Action:**
- ResilientNode wrapper catches errors per ErrorPolicy
- Error logged with metrics collector
- Pipeline marked as Error state
- Emit error event to frontend
- Graceful shutdown of affected node tasks

**Error Policies:**
- `Propagate`: Stop pipeline on first error
- `Retry`: Retry failed operation N times
- `Skip`: Skip failed frame and continue
- `Fallback`: Use default/safe value

**Monitoring:**
```rust
// Check node metrics for error counts
let monitor = pipeline.get_monitor()?;
let node_metrics = monitor.get_node_metrics("node-id");
println!("Errors: {}", node_metrics.error_count);
```

### Device Errors
**Causes:**
- Hardware disconnection (USB device unplugged)
- Buffer overrun (processing too slow)
- Buffer underrun (source too slow)
- Driver crash

**Action:**
- Attempt recovery (reconnect, reset buffers)
- If recovery fails after N attempts, mark pipeline as Error
- Emit error event with device-specific information
- User must restart kernel/pipeline

**TODO:** Device error recovery not yet implemented

## Frontend Error Display

### Status Event Handling
Frontend subscribes to `pipeline-status` events:

```typescript
listen('pipeline-status', (event) => {
  const status = event.payload as PipelineStatusEvent;

  if (status.state === 'Error' && status.error) {
    // Display error to user
    setLastStatus(`❌ ${status.error}`);
  }
});
```

### User-Friendly Error Messages
Map technical errors to user-friendly descriptions:

- "Graph translation failed" → "Invalid graph structure. Please check node connections."
- "Pipeline creation failed" → "Failed to create pipeline. Check node configurations."
- "Unknown node type" → "Unknown node type. This node may not be registered."
- "Kernel not running" → "Audio kernel is not running. Please start it first."

## Error Recovery Patterns

### Retry Pattern
For transient errors (network, device):
1. Catch error
2. Wait with exponential backoff
3. Retry operation
4. If exceeds max retries, fail permanently

### Fallback Pattern
For non-critical errors:
1. Catch error
2. Use safe default value
3. Log warning
4. Continue processing

### Circuit Breaker Pattern (Future)
For cascading failures:
1. Monitor error rate
2. If error rate exceeds threshold, "open circuit"
3. Fail fast without attempting operation
4. Periodically test if error condition cleared
5. "Close circuit" when service restored

## Testing Error Paths

### Unit Tests
Test each error condition in isolation:
```rust
#[tokio::test]
async fn test_deploy_invalid_graph_returns_error() {
    let graph = GraphJson {
        nodes: vec![
            json!({"id": "invalid-1", "type": "NonExistentNode", "parameters": {}})
        ],
        edges: vec![],
    };

    let result = deploy_graph(app.handle(), State::from(&state), graph).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Pipeline creation failed"));
}
```

### Integration Tests
Test error propagation across layers:
```rust
#[tokio::test]
async fn test_error_event_emitted() {
    // Deploy invalid graph
    // Verify error event received by frontend
    // Verify pipeline not stored in state
}
```

### Manual Tests
Test user-visible error handling:
1. Deploy invalid graph → verify error shown in UI
2. Start pipeline without kernel → verify friendly error
3. Disconnect device mid-execution → verify recovery attempted

## Logging Best Practices

### Error Logging
Always log errors with context:
```rust
println!("Pipeline creation error: {}", error_msg);
// Or with structured logging:
log::error!(
    target: "pipeline",
    pipeline_id = %pipeline_id,
    error = %error_msg;
    "Failed to create pipeline"
);
```

### Debug Logging
Log state transitions for debugging:
```rust
println!("Pipeline {} state: {:?} -> {:?}", id, old_state, new_state);
```

### Avoid Logging Sensitive Data
Do NOT log:
- Audio samples (too large)
- User credentials
- Device serial numbers
- Full error stack traces in production

## Future Improvements

### Planned Features
1. **Error Subscription API** - `pipeline.subscribe_errors()` → broadcast channel
2. **Structured Error Types** - Replace String errors with typed enum
3. **Error Aggregation** - Collect multiple errors before reporting
4. **Automatic Recovery** - Self-healing pipelines for common failures
5. **Error Analytics** - Track error rates, patterns, correlations

### Open Questions
1. Should pipeline auto-retry on transient errors?
2. What's the max retry count for device reconnection?
3. Should we persist error logs to disk?
4. How to handle partial pipeline failures (some nodes OK, some failed)?

## References

- `src-tauri/src/commands/pipeline.rs` - Main error handling implementation
- `src/resilience/mod.rs` - ResilientNode error policies
- `src/observability/metrics.rs` - Error metrics tracking
- `src/engine/state.rs` - Pipeline state machine
