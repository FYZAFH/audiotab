# Phase 6 Manual Test Procedure

## Prerequisites
- Application built and running in dev mode: `cargo tauri dev`
- No compilation errors
- All automated tests passing: `cargo test`

## Test 1: Simple Graph Deployment

**Objective:** Verify graph can be created in UI and deployed to backend

**Steps:**
1. Open application
2. Navigate to "Configure > Process Configuration"
3. Click "Edit Mode" button
4. Drag "Sine Generator" node from palette to canvas
5. Drag "Print" node from palette to canvas
6. Connect Sine Generator output to Print input
7. Click "Deploy Configuration" button

**Expected Result:**
- Status bar shows: "Successfully deployed pipeline: pipeline_<uuid>"
- No errors in console
- No errors in terminal (backend logs)

**Actual Result:** [ ]

---

## Test 2: Invalid Graph Rejection

**Objective:** Verify invalid graphs are rejected gracefully

**Steps:**
1. Open browser devtools console
2. Create graph with invalid node by running:
   ```javascript
   invoke('deploy_graph', {
     graph: {
       nodes: [{id: 'bad', type: 'NonExistent', parameters: {}}],
       edges: []
     }
   })
   ```

**Expected Result:**
- Promise rejects with error
- Status bar shows error in red: "❌ Failed to create pipeline..."
- Backend logs show: "Unknown node type: NonExistent"

**Actual Result:** [ ]

---

## Test 3: Multi-Node Pipeline

**Objective:** Verify complex graphs work

**Steps:**
1. Edit Mode ON
2. Create graph:
   - Sine Generator → Gain → FFT → Print
3. Set Gain to 0.5
4. Deploy

**Expected Result:**
- Deployment succeeds
- Pipeline ID returned
- All 4 nodes registered in backend (check logs)

**Actual Result:** [ ]

---

## Test 4: Error Recovery

**Objective:** Verify errors don't crash the app

**Steps:**
1. Deploy invalid graph (Test 2)
2. Verify error shown
3. Fix graph (add valid nodes)
4. Deploy again

**Expected Result:**
- Second deployment succeeds
- Previous error cleared from status bar

**Actual Result:** [ ]

---

## Completion Checklist

- [ ] Test 1 passes
- [ ] Test 2 passes
- [ ] Test 3 passes
- [ ] Test 4 passes
- [ ] No memory leaks (check Activity Monitor after 10 deployments)
- [ ] No zombie processes
- [ ] Backend logs clean (no panics/errors)
