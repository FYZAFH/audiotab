# Hardware End-to-End Integration Test

## Goal
Verify complete data flow from physical microphone through pipeline to visualization with real hardware.

## Prerequisites
- Physical microphone (built-in or USB)
- Application running in development mode
- No other audio applications using the microphone

## Test Procedure

### Part 1: Device Setup (5 minutes)

1. Start application: `cd src-tauri && cargo run`
2. Navigate to Device Manager: `/configure/hardware`
3. Click "Discover Devices"
4. **Verify**: At least one audio input device appears
5. Click "Add" on your microphone
6. **Verify**: Device appears in "Device Profiles" panel
7. **Record**: Device profile ID (e.g., "cpal-audio-input-0-123456")

### Part 2: Graph Configuration (5 minutes)

1. Navigate to Process Configuration: `/configure/process`
2. Click "Edit Mode"
3. **Verify**: NodePropertiesPanel appears on right side
4. From node palette, drag "Audio Source" onto canvas
5. Click the Audio Source node to select it
6. **Verify**: Properties panel shows "Audio Device" dropdown
7. Select your microphone from dropdown
8. **Verify**: Node shows device name (visual feedback)
9. From node palette, drag "Debug Sink" onto canvas
10. Connect: Audio Source output → Debug Sink input
11. Click "Deploy Configuration"
12. **Verify**: Green success message appears
13. **Record**: Pipeline ID from success message

### Part 3: Pipeline Execution (10 minutes)

1. Open browser console (F12)
2. Navigate to Home: `/`
3. Click "Start Kernel"
4. **Verify**: Kernel status shows "Running"
5. Find your deployed pipeline in list
6. Click "Start Pipeline"
7. **Verify**: Pipeline status changes to "Running"
8. **Verify**: Console shows: "Successfully injected device channels for '<device-id>'"

### Part 4: Hardware Data Flow Verification (5 minutes)

1. Click "Trigger Frame" button
2. **Verify**: Console shows debug output from DebugSinkNode
3. **Verify**: Debug output contains audio data (non-zero values)
4. Make a loud sound near microphone (clap hands)
5. Click "Trigger Frame" again
6. **Verify**: Audio levels in debug output are higher than before
7. Be silent
8. Click "Trigger Frame" again
9. **Verify**: Audio levels drop back to near-zero

### Part 5: Visualization (5 minutes)

1. Navigate to Visualization: `/view/visualization`
2. **Verify**: Waveform display is visible
3. Make continuous sound (hum, speak, music)
4. **Verify**: Waveform shows non-zero amplitude
5. **Verify**: Waveform responds to sound in real-time
6. Stop making sound
7. **Verify**: Waveform drops to near-zero

## Success Criteria

**All must pass:**
- ✅ Device discovery finds real hardware
- ✅ Device profile persists (survives app restart)
- ✅ NodePropertiesPanel integrated in UI
- ✅ Device dropdown populated with profiles
- ✅ Pipeline deployment injects DeviceChannels
- ✅ AudioSourceNode receives hardware data
- ✅ Console shows correct audio levels
- ✅ Waveform visualization reacts to sound

## Failure Analysis

**If device discovery fails:**
- Check system permissions (macOS: System Preferences > Security > Microphone)
- Verify no other app is using microphone
- Check console for cpal errors

**If channel injection fails:**
- Check console for "Device manager lock poisoned" error
- Verify device was added to Device Manager
- Check device_profile_id matches between node and profile

**If no audio data appears:**
- Verify device is actually streaming (check Activity Monitor for audio process)
- Check console for "try_recv" errors
- Verify pipeline is in "Running" state
- Check RingBuffer is receiving data

**If visualization doesn't update:**
- Verify RingBuffer is configured correctly
- Check browser console for React errors
- Verify WebSocket connection is active

## Cleanup

After test:
```bash
# Stop pipeline
# Stop kernel
# Close application
# (Device profiles persist for future tests)
```

## Test Results Log

| Date | Tester | Part 1 | Part 2 | Part 3 | Part 4 | Part 5 | Notes |
|------|--------|--------|--------|--------|--------|--------|-------|
| 2025-12-01 | - | ⏳ | ⏳ | ⏳ | ⏳ | ⏳ | Ready for testing |
