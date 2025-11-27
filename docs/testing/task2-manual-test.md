# Task 2 Manual Test Documentation

## Pipeline Deployment Command Manual Tests

### Prerequisites
- Application built and running in dev mode: `cargo tauri dev`
- No compilation errors
- All automated tests passing: `cargo test --lib`

### Test 1: Graph Translation

**Objective:** Verify the graph translator correctly converts frontend format to backend format

**Steps:**
1. Run the manual test:
   ```bash
   cd src-tauri
   cargo test manual_test_deploy_sine_to_print -- --ignored --nocapture
   ```

**Expected Result:**
- Console shows graph translation output
- Backend JSON has correct structure:
  - `nodes` array with `id`, `type`, `config` fields
  - `connections` array with `from`, `to` fields
  - `pipeline_config` with defaults
- Node type mapping works: `SineGenerator` → `AudioSourceNode`, `Print` → `DebugSinkNode`
- Test passes

**Actual Result:** ✅ PASS

---

### Test 2: Pipeline Creation from Translated Graph

**Objective:** Verify AsyncPipeline can be created from translated graph

**Steps:**
1. Use the same manual test as above
2. Check that pipeline creation succeeds

**Expected Result:**
- Pipeline creation does not return error
- Message "Pipeline created successfully!" appears
- Test passes

**Actual Result:** ✅ PASS

---

### Test 3: Error Handling for Invalid Graph

**Objective:** Verify invalid graphs are rejected gracefully

**Steps:**
1. Run the error test:
   ```bash
   cd src-tauri
   cargo test test_deploy_invalid_graph_returns_error
   ```

**Expected Result:**
- Test passes
- Pipeline creation fails for unknown node type
- Error message is clear

**Actual Result:** ✅ PASS

---

### Test 4: Pipeline Storage in AppState

**Objective:** Verify pipelines are correctly stored in AppState

**Steps:**
1. Run the storage test:
   ```bash
   cd src-tauri
   cargo test test_deploy_graph_creates_pipeline
   ```

**Expected Result:**
- Pipeline is stored in state.pipelines HashMap
- Pipeline ID is correct format: `pipeline_<uuid>`
- Initial state is `Idle`
- Test passes

**Actual Result:** ✅ PASS

---

## Integration Test Summary

All automated tests pass:
```bash
cd src-tauri
cargo test --lib
```

Result: **22 passed; 0 failed; 1 ignored**

---

## Manual Testing in Full Application

To test the deploy_graph command in the running Tauri application:

1. Start the app:
   ```bash
   cargo tauri dev
   ```

2. Open the browser console in the app

3. Navigate to "Configure > Process Configuration"

4. Create a simple graph:
   - Drag "Sine Generator" node onto canvas
   - Drag "Print" node onto canvas
   - Connect them

5. Click "Deploy Configuration"

6. Check the backend logs for:
   - "Deploying graph with X nodes, Y edges"
   - "Translated graph: <json>"
   - "Pipeline <id> created successfully"

7. Check the frontend status bar for:
   - "Successfully deployed pipeline: pipeline_<uuid>"

---

## Notes

- The deploy_graph command now fully implements:
  ✅ Graph translation (frontend → backend format)
  ✅ Pipeline creation from translated graph
  ✅ Pipeline storage in AppState
  ✅ Status event emission
  ✅ Error handling and propagation

- Pipeline execution (actually running nodes) is intentionally NOT implemented yet
  - This will be Task 3 (KernelManager integration)
  - For now, pipelines are created and stored in `Idle` state

- All tests verify the core deployment logic works correctly
- Manual testing in the full app requires UI interaction (tested separately)
