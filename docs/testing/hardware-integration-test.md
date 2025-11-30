# Hardware Integration Test Procedure

## Goal
Verify complete data flow: Physical Microphone → HAL → DeviceManager → AudioSourceNode → Pipeline → RingBuffer → Visualization

## Prerequisites
- MacBook with built-in microphone OR USB audio interface
- Application running in development mode

## Test Procedure

### Phase 1: Device Management

1. Navigate to Device Manager page (`/configure/hardware`)
2. Click "Discover" button
3. **Verify**: At least one audio input device appears in "Available Devices" panel
4. Click "Add" on built-in microphone (or USB device)
5. **Verify**: Device appears in "Device Profiles" panel with default configuration

### Phase 2: Graph Configuration

1. Navigate to Process Configuration page (`/configure/process`)
2. Click "Edit Mode" to enable editing
3. Add an AudioSourceNode to the graph
4. Select the AudioSourceNode
5. In properties panel (if integrated), select device from dropdown
   - **Note**: As of 2025-12-01, NodePropertiesPanel component exists but is not yet integrated into ProcessConfiguration UI
   - **Workaround**: Device selection will need to be added via direct graph JSON editing or future UI integration
6. Add a DebugSinkNode
7. Connect AudioSourceNode output → DebugSinkNode input
8. Click "Deploy Configuration"
9. **Verify**: Success message appears in status bar

### Phase 3: Pipeline Execution

1. Navigate to Home page (`/`)
2. Click "Start Kernel" (if not already running)
3. **Verify**: Kernel status shows "Running"
4. Click "Start Pipeline" for the deployed configuration
5. **Verify**: Pipeline status shows "Running"
6. Click "Trigger Frame"
7. **Verify**: Console shows debug output from DebugSinkNode with audio data

### Phase 4: Visualization

1. Make a sound near the microphone (clap, speak, etc.)
2. Navigate to Visualization Demo page (`/view/visualization`)
3. **Verify**: Waveform shows non-zero audio levels
4. **Verify**: Waveform reacts to sound input

## Expected Results

- ✅ Device discovery finds real hardware
- ✅ Device profiles persist across app restarts
- ⚠️  AudioSourceNode device selection (UI integration pending)
- ⏳ Device channels injection during pipeline deployment (structure present, full implementation pending)
- ✅ Pipeline processes audio frames
- ✅ RingBuffer contains audio data
- ✅ Visualization displays waveform

## Known Limitations (as of 2025-12-01)

### Pending Implementations:
1. **Device Injection (Task 9)**:
   - Structure added to deploy_graph command
   - Actual device creation and channel injection not yet implemented
   - Requires async device handling which is complex in sync deployment context

2. **Properties Panel Integration (Task 10)**:
   - NodePropertiesPanel component created
   - Not yet integrated into ProcessConfiguration page
   - Device dropdown functional in isolation but needs UI hookup

3. **Channel Mapping**:
   - Channel mapping data structures complete
   - Real-time channel mapping application not yet implemented
   - Default identity mapping used

4. **Calibration**:
   - Calibration data model complete
   - Calibration application not yet functional

5. **Multi-Device Support**:
   - Only one device can be active at a time
   - Concurrent device management not yet implemented

6. **Device Error Recovery**:
   - No automatic reconnection on device disconnection
   - Manual restart required if device fails

## Future Work

- Complete device injection in pipeline deployment
- Integrate NodePropertiesPanel into ProcessConfiguration UI
- Implement real-time channel mapping
- Add calibration functionality
- Support multiple concurrent devices
- Add device error recovery and reconnection
- Create advanced channel mapping UI visualizer
- Add device templates/presets system

## Test Results Log

| Date | Tester | Phase 1 | Phase 2 | Phase 3 | Phase 4 | Notes |
|------|--------|---------|---------|---------|---------|-------|
| 2025-12-01 | - | ⏳ | ⏳ | ⏳ | ⏳ | Initial test document created |

## Notes

This test procedure documents the **intended** end-to-end flow for hardware device management. As of the implementation completion date (2025-12-01), the core infrastructure is in place but some integration steps are pending. The test procedure will be fully executable once:
1. Device injection is completed in deploy_graph
2. NodePropertiesPanel is integrated into the UI
