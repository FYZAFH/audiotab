# Phase 2: Frontend & Builder Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Tauri desktop app with React Flow visual editor for audio processing pipeline configuration.

**Architecture:** Tauri v2 wraps Rust backend (src-tauri) and React frontend (src-frontend). Frontend uses React Flow for node canvas, Zustand for state, Tauri commands for backend communication. Backend exposes node registry and pipeline deployment via Tauri commands.

**Tech Stack:** Tauri v2, Vite, React 19, TypeScript, React Flow, Zustand, shadcn/ui, Tailwind CSS, rustfft

---

## Task A: Initialize Tauri Project Structure

### A1: Update workspace Cargo.toml

**Files:**
- Modify: `Cargo.toml`

**Step 1: Update workspace configuration**

Replace the entire `Cargo.toml` with:

```toml
[workspace]
members = [".", "src-tauri"]
resolver = "2"

[package]
name = "audiotab"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.40", features = ["full"] }
anyhow = "1.0"
async-trait = "0.1"

[dev-dependencies]
tokio-test = "0.4"
```

**Step 2: Verify workspace structure**

Run: `cargo check`
Expected: SUCCESS (workspace recognized)

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: configure Cargo workspace for Tauri"
```

### A2: Initialize Tauri app

**Files:**
- Create: `src-tauri/` directory

**Step 1: Install Tauri CLI**

Run: `cargo install tauri-cli --version '^2.0.0'`
Expected: Installs cargo-tauri binary

**Step 2: Initialize Tauri project**

Run: `cargo tauri init --ci`

When prompted:
- App name: `audiotab`
- Window title: `StreamLab Core`
- Web assets location: `../src-frontend/dist`
- Dev server URL: `http://localhost:5173`
- Frontend dev command: `npm run dev`
- Frontend build command: `npm run build`

Expected: Creates `src-tauri/` directory with initial files

**Step 3: Verify structure**

Run: `ls src-tauri/`
Expected: `Cargo.toml`, `tauri.conf.json`, `src/`, `icons/`

**Step 4: Commit**

```bash
git add src-tauri/
git commit -m "chore: initialize Tauri v2 project"
```

### A3: Configure Tauri

**Files:**
- Modify: `src-tauri/tauri.conf.json`

**Step 1: Update Tauri configuration**

Replace `src-tauri/tauri.conf.json` with:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "StreamLab Core",
  "version": "0.1.0",
  "identifier": "com.audiotab.streamlab",
  "build": {
    "beforeDevCommand": "cd ../src-frontend && npm run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "cd ../src-frontend && npm run build",
    "frontendDist": "../src-frontend/dist"
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  },
  "app": {
    "windows": [
      {
        "title": "StreamLab Core",
        "width": 1280,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  }
}
```

**Step 2: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "chore: configure Tauri window and build settings"
```

---

## Task B: Setup Backend (src-tauri)

### B1: Create app state structure

**Files:**
- Create: `src-tauri/src/state.rs`

**Step 1: Define AppState**

Create `src-tauri/src/state.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use audiotab::engine::{AsyncPipeline, PipelineState};

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<NodeRegistry>,
    pub pipelines: Arc<Mutex<HashMap<String, PipelineHandle>>>,
}

pub struct PipelineHandle {
    pub id: String,
    pub pipeline: AsyncPipeline,
    pub state: PipelineState,
}

pub struct NodeRegistry {
    nodes: Vec<NodeMetadata>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeMetadata {
    pub id: String,
    pub name: String,
    pub category: String,
    pub inputs: Vec<PortMetadata>,
    pub outputs: Vec<PortMetadata>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortMetadata {
    pub id: String,
    pub name: String,
    pub data_type: String,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn register(&mut self, meta: NodeMetadata) {
        self.nodes.push(meta);
    }

    pub fn list_nodes(&self) -> Vec<NodeMetadata> {
        self.nodes.clone()
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        // Will add nodes in Task F
        registry
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(NodeRegistry::with_defaults()),
            pipelines: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Export from main**

Modify `src-tauri/src/main.rs` to add module:

```rust
mod state;

fn main() {
    // Will update later
}
```

**Step 3: Update Cargo.toml**

Edit `src-tauri/Cargo.toml` to add dependencies:

```toml
[dependencies]
tauri = { version = "2", features = ["devtools"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.40", features = ["full"] }
audiotab = { path = "../" }
```

**Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat(tauri): add AppState and NodeRegistry structure"
```

### B2: Implement node registry command

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/nodes.rs`

**Step 1: Create commands module**

Create `src-tauri/src/commands/mod.rs`:

```rust
pub mod nodes;
pub mod pipeline;
```

Create `src-tauri/src/commands/nodes.rs`:

```rust
use crate::state::{AppState, NodeMetadata};
use tauri::State;

#[tauri::command]
pub fn get_node_registry(state: State<AppState>) -> Vec<NodeMetadata> {
    state.registry.list_nodes()
}
```

**Step 2: Add commands module to main**

Modify `src-tauri/src/main.rs`:

```rust
mod state;
mod commands;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::nodes::get_node_registry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src-tauri/src/commands/
git commit -m "feat(tauri): add get_node_registry command"
```

### B3: Implement pipeline commands

**Files:**
- Create: `src-tauri/src/commands/pipeline.rs`

**Step 1: Define pipeline commands**

Create `src-tauri/src/commands/pipeline.rs`:

```rust
use crate::state::{AppState, PipelineHandle};
use audiotab::engine::PipelineState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct GraphJson {
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PipelineStatus {
    pub id: String,
    pub state: String,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineAction {
    Start,
    Stop,
    Pause,
}

#[tauri::command]
pub async fn deploy_graph(
    state: State<'_, AppState>,
    graph: GraphJson,
) -> Result<String, String> {
    // For now, just create a placeholder pipeline ID
    let pipeline_id = format!("pipeline_{}", uuid::Uuid::new_v4());

    // TODO: Parse graph and create actual pipeline in Task F
    println!("Deploying graph with {} nodes, {} edges",
             graph.nodes.len(), graph.edges.len());

    Ok(pipeline_id)
}

#[tauri::command]
pub fn get_all_pipeline_states(state: State<AppState>) -> Vec<PipelineStatus> {
    let pipelines = state.pipelines.lock().unwrap();
    pipelines
        .values()
        .map(|handle| PipelineStatus {
            id: handle.id.clone(),
            state: format!("{:?}", handle.state),
            error: None,
        })
        .collect()
}

#[tauri::command]
pub async fn control_pipeline(
    state: State<'_, AppState>,
    id: String,
    action: PipelineAction,
) -> Result<(), String> {
    println!("Control pipeline {}: {:?}", id, action);
    // TODO: Implement actual control in Task F
    Ok(())
}
```

**Step 2: Add uuid dependency**

Edit `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri = { version = "2", features = ["devtools"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.40", features = ["full"] }
audiotab = { path = "../" }
uuid = { version = "1.0", features = ["v4"] }
```

**Step 3: Register commands**

Modify `src-tauri/src/main.rs`:

```rust
mod state;
mod commands;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::nodes::get_node_registry,
            commands::pipeline::deploy_graph,
            commands::pipeline::get_all_pipeline_states,
            commands::pipeline::control_pipeline,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add src-tauri/
git commit -m "feat(tauri): add pipeline deployment and control commands"
```

---

## Task C: Setup Frontend (src-frontend)

### C1: Initialize Vite + React project

**Files:**
- Create: `src-frontend/` directory

**Step 1: Create Vite project**

Run from project root:
```bash
npm create vite@latest src-frontend -- --template react-ts
```

Expected: Creates src-frontend with Vite + React + TypeScript

**Step 2: Navigate and install dependencies**

Run:
```bash
cd src-frontend
npm install
```

Expected: Installs base dependencies

**Step 3: Verify dev server**

Run: `npm run dev`
Expected: Server starts on http://localhost:5173
Stop server with Ctrl+C

**Step 4: Commit**

```bash
git add src-frontend/
git commit -m "chore: initialize Vite + React + TypeScript frontend"
```

### C2: Install core dependencies

**Files:**
- Modify: `src-frontend/package.json`

**Step 1: Install dependencies**

Run from `src-frontend/`:
```bash
npm install @tauri-apps/api@^2.0.0 zustand@^5.0.0 @xyflow/react@^12.0.0 @tanstack/react-query@^5.0.0
npm install -D tailwindcss@^3.4.0 postcss@^8.4.0 autoprefixer@^10.4.0
```

Expected: Packages installed

**Step 2: Initialize Tailwind**

Run: `npx tailwindcss init -p`
Expected: Creates `tailwind.config.js` and `postcss.config.js`

**Step 3: Configure Tailwind**

Replace `src-frontend/tailwind.config.js`:

```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
```

**Step 4: Add Tailwind directives**

Replace `src-frontend/src/index.css`:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

**Step 5: Verify build**

Run: `npm run build`
Expected: Build succeeds, creates `dist/`

**Step 6: Commit**

```bash
git add src-frontend/
git commit -m "chore: install core frontend dependencies (Tauri, Zustand, React Flow, Tailwind)"
```

### C3: Setup shadcn/ui

**Files:**
- Create: `src-frontend/components.json`
- Create: `src-frontend/src/components/ui/` directory

**Step 1: Initialize shadcn/ui**

Run from `src-frontend/`:
```bash
npx shadcn@latest init
```

When prompted:
- Style: Default
- Base color: Slate
- CSS variables: Yes

Expected: Creates `components.json` and `src/lib/utils.ts`

**Step 2: Install base components**

Run:
```bash
npx shadcn@latest add button card input label select separator
```

Expected: Components added to `src/components/ui/`

**Step 3: Verify compilation**

Run: `npm run build`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src-frontend/
git commit -m "chore: setup shadcn/ui with base components"
```

---

## Task D: Implement Node Registry Frontend

### D1: Create TypeScript types

**Files:**
- Create: `src-frontend/src/types/nodes.ts`

**Step 1: Define node types**

Create `src-frontend/src/types/nodes.ts`:

```typescript
export interface PortMetadata {
  id: string;
  name: string;
  data_type: string;
}

export interface NodeMetadata {
  id: string;
  name: string;
  category: string;
  inputs: PortMetadata[];
  outputs: PortMetadata[];
  parameters: Record<string, any>;
}

export interface GraphNode {
  id: string;
  type: string;
  position: { x: number; y: number };
  data: {
    label: string;
    metadata: NodeMetadata;
    parameters: Record<string, any>;
  };
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  sourceHandle: string;
  targetHandle: string;
}

export interface GraphJson {
  nodes: any[];
  edges: any[];
}

export interface PipelineStatus {
  id: string;
  state: string;
  error?: string;
}

export type PipelineAction = 'start' | 'stop' | 'pause';
```

**Step 2: Create Tauri command hooks**

Create `src-frontend/src/hooks/useTauriCommands.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';
import { useQuery, useMutation } from '@tanstack/react-query';
import type { NodeMetadata, GraphJson, PipelineStatus, PipelineAction } from '../types/nodes';

export function useNodeRegistry() {
  return useQuery({
    queryKey: ['nodes'],
    queryFn: () => invoke<NodeMetadata[]>('get_node_registry'),
  });
}

export function useDeployGraph() {
  return useMutation({
    mutationFn: (graph: GraphJson) => invoke<string>('deploy_graph', { graph }),
  });
}

export function usePipelineStates() {
  return useQuery({
    queryKey: ['pipeline-states'],
    queryFn: () => invoke<PipelineStatus[]>('get_all_pipeline_states'),
  });
}

export function useControlPipeline() {
  return useMutation({
    mutationFn: ({ id, action }: { id: string; action: PipelineAction }) =>
      invoke<void>('control_pipeline', { id, action }),
  });
}
```

**Step 3: Verify compilation**

Run: `npm run build`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src-frontend/src/types/ src-frontend/src/hooks/
git commit -m "feat(frontend): add TypeScript types and Tauri command hooks"
```

---

## Task E: Implement Zustand Stores

### E1: Create flow store

**Files:**
- Create: `src-frontend/src/stores/flowStore.ts`

**Step 1: Implement flow store**

Create `src-frontend/src/stores/flowStore.ts`:

```typescript
import { create } from 'zustand';
import { Node, Edge, Connection, addEdge, applyNodeChanges, applyEdgeChanges, NodeChange, EdgeChange } from '@xyflow/react';

interface FlowState {
  nodes: Node[];
  edges: Edge[];
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (type: string, position: { x: number; y: number }, metadata: any) => void;
  deleteSelected: () => void;
  exportGraph: () => { nodes: any[]; edges: any[] };
}

export const useFlowStore = create<FlowState>((set, get) => ({
  nodes: [],
  edges: [],

  onNodesChange: (changes) => {
    set({
      nodes: applyNodeChanges(changes, get().nodes),
    });
  },

  onEdgesChange: (changes) => {
    set({
      edges: applyEdgeChanges(changes, get().edges),
    });
  },

  onConnect: (connection) => {
    set({
      edges: addEdge(connection, get().edges),
    });
  },

  addNode: (type, position, metadata) => {
    const newNode: Node = {
      id: `${type}-${Date.now()}`,
      type: 'custom',
      position,
      data: {
        label: metadata.name,
        metadata,
        parameters: {},
      },
    };
    set({ nodes: [...get().nodes, newNode] });
  },

  deleteSelected: () => {
    const { nodes, edges } = get();
    set({
      nodes: nodes.filter((n) => !n.selected),
      edges: edges.filter((e) => !e.selected),
    });
  },

  exportGraph: () => {
    const { nodes, edges } = get();
    return {
      nodes: nodes.map((n) => ({
        id: n.id,
        type: n.data.metadata.id,
        position: n.position,
        parameters: n.data.parameters,
      })),
      edges: edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle,
      })),
    };
  },
}));
```

**Step 2: Verify compilation**

Run: `npm run build`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src-frontend/src/stores/flowStore.ts
git commit -m "feat(frontend): implement Zustand flow store"
```

---

## Task F: Implement React Flow Editor

### F1: Create BaseNode component

**Files:**
- Create: `src-frontend/src/components/FlowEditor/BaseNode.tsx`

**Step 1: Implement BaseNode**

Create `src-frontend/src/components/FlowEditor/BaseNode.tsx`:

```typescript
import { memo } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { Card } from '../ui/card';

export default memo(({ data }: NodeProps) => {
  const metadata = data.metadata;

  return (
    <Card className="min-w-[150px] bg-slate-800 border-slate-600">
      <div className="p-3">
        <div className="font-semibold text-sm text-white mb-2">{data.label}</div>

        {/* Input Ports */}
        {metadata.inputs.map((input: any, idx: number) => (
          <Handle
            key={input.id}
            type="target"
            position={Position.Left}
            id={input.id}
            style={{ top: `${((idx + 1) * 100) / (metadata.inputs.length + 1)}%` }}
            className="w-3 h-3 bg-blue-500"
          />
        ))}

        {/* Output Ports */}
        {metadata.outputs.map((output: any, idx: number) => (
          <Handle
            key={output.id}
            type="source"
            position={Position.Right}
            id={output.id}
            style={{ top: `${((idx + 1) * 100) / (metadata.outputs.length + 1)}%` }}
            className="w-3 h-3 bg-green-500"
          />
        ))}

        {/* Parameter count indicator */}
        {Object.keys(metadata.parameters).length > 0 && (
          <div className="text-xs text-slate-400 mt-2">
            {Object.keys(metadata.parameters).length} params
          </div>
        )}
      </div>
    </Card>
  );
});
```

**Step 2: Create FlowEditor component**

Create `src-frontend/src/components/FlowEditor/FlowEditor.tsx`:

```typescript
import { useCallback } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  ConnectionMode,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useFlowStore } from '../../stores/flowStore';
import BaseNode from './BaseNode';

const nodeTypes = {
  custom: BaseNode,
};

export default function FlowEditor() {
  const { nodes, edges, onNodesChange, onEdgesChange, onConnect } = useFlowStore();

  const onDrop = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    const metadata = JSON.parse(event.dataTransfer.getData('application/reactflow'));
    const position = {
      x: event.clientX,
      y: event.clientY,
    };
    useFlowStore.getState().addNode(metadata.id, position, metadata);
  }, []);

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  return (
    <div className="w-full h-full bg-slate-900" onDrop={onDrop} onDragOver={onDragOver}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        nodeTypes={nodeTypes}
        connectionMode={ConnectionMode.Loose}
        fitView
      >
        <Background />
        <Controls />
        <MiniMap />
      </ReactFlow>
    </div>
  );
}
```

**Step 3: Verify compilation**

Run: `npm run build`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src-frontend/src/components/FlowEditor/
git commit -m "feat(frontend): implement React Flow editor with BaseNode"
```

### F2: Create NodePalette component

**Files:**
- Create: `src-frontend/src/components/NodePalette/NodePalette.tsx`

**Step 1: Implement NodePalette**

Create `src-frontend/src/components/NodePalette/NodePalette.tsx`:

```typescript
import { useNodeRegistry } from '../../hooks/useTauriCommands';
import { Card } from '../ui/card';
import { Separator } from '../ui/separator';

export default function NodePalette() {
  const { data: nodes, isLoading } = useNodeRegistry();

  const onDragStart = (event: React.DragEvent, metadata: any) => {
    event.dataTransfer.setData('application/reactflow', JSON.stringify(metadata));
    event.dataTransfer.effectAllowed = 'move';
  };

  if (isLoading) {
    return (
      <div className="w-64 bg-slate-800 p-4">
        <div className="text-white">Loading nodes...</div>
      </div>
    );
  }

  const categories = [...new Set(nodes?.map((n) => n.category) || [])];

  return (
    <div className="w-64 bg-slate-800 p-4 overflow-y-auto">
      <h2 className="text-white font-bold text-lg mb-4">Node Palette</h2>
      {categories.map((category) => (
        <div key={category} className="mb-4">
          <h3 className="text-slate-300 font-semibold text-sm mb-2">{category}</h3>
          <div className="space-y-2">
            {nodes
              ?.filter((n) => n.category === category)
              .map((node) => (
                <Card
                  key={node.id}
                  draggable
                  onDragStart={(e) => onDragStart(e, node)}
                  className="p-3 cursor-move bg-slate-700 border-slate-600 hover:bg-slate-600"
                >
                  <div className="text-white text-sm">{node.name}</div>
                  <div className="text-slate-400 text-xs mt-1">
                    {node.inputs.length} in, {node.outputs.length} out
                  </div>
                </Card>
              ))}
          </div>
          <Separator className="my-3 bg-slate-600" />
        </div>
      ))}
    </div>
  );
}
```

**Step 2: Verify compilation**

Run: `npm run build`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src-frontend/src/components/NodePalette/
git commit -m "feat(frontend): implement draggable NodePalette"
```

### F3: Create main App layout

**Files:**
- Modify: `src-frontend/src/App.tsx`

**Step 1: Implement App layout**

Replace `src-frontend/src/App.tsx`:

```typescript
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import FlowEditor from './components/FlowEditor/FlowEditor';
import NodePalette from './components/NodePalette/NodePalette';
import { Button } from './components/ui/button';
import { useFlowStore } from './stores/flowStore';
import { useDeployGraph } from './hooks/useTauriCommands';

const queryClient = new QueryClient();

function AppContent() {
  const exportGraph = useFlowStore((state) => state.exportGraph);
  const deployMutation = useDeployGraph();

  const handleDeploy = async () => {
    const graph = exportGraph();
    try {
      const pipelineId = await deployMutation.mutateAsync(graph);
      console.log('Deployed pipeline:', pipelineId);
    } catch (error) {
      console.error('Deploy failed:', error);
    }
  };

  return (
    <div className="flex flex-col h-screen bg-slate-900">
      {/* Top Bar */}
      <div className="h-14 bg-slate-800 border-b border-slate-700 flex items-center px-4">
        <h1 className="text-white text-xl font-bold">StreamLab Core</h1>
        <div className="ml-auto space-x-2">
          <Button onClick={handleDeploy} disabled={deployMutation.isPending}>
            {deployMutation.isPending ? 'Deploying...' : 'Deploy'}
          </Button>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        <NodePalette />
        <div className="flex-1">
          <FlowEditor />
        </div>
      </div>

      {/* Status Bar */}
      <div className="h-8 bg-slate-800 border-t border-slate-700 flex items-center px-4">
        <span className="text-slate-400 text-sm">
          {deployMutation.isSuccess && 'Pipeline deployed successfully'}
          {deployMutation.isError && 'Deploy failed'}
        </span>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AppContent />
    </QueryClientProvider>
  );
}
```

**Step 2: Verify compilation**

Run: `npm run build`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src-frontend/src/App.tsx
git commit -m "feat(frontend): implement main App layout with deploy button"
```

---

## Task G: Add Initial Nodes to Registry

### G1: Define node metadata

**Files:**
- Create: `src-tauri/src/nodes/mod.rs`
- Create: `src-tauri/src/nodes/metadata.rs`

**Step 1: Create nodes module**

Create `src-tauri/src/nodes/mod.rs`:

```rust
pub mod metadata;

pub use metadata::*;
```

Create `src-tauri/src/nodes/metadata.rs`:

```rust
use crate::state::{NodeMetadata, PortMetadata};
use serde_json::json;

pub fn audio_source_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "audio_source".to_string(),
        name: "Audio Source".to_string(),
        category: "Sources".to_string(),
        inputs: vec![],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "Audio Out".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        parameters: json!({
            "sample_rate": { "type": "number", "default": 48000 },
            "buffer_size": { "type": "number", "default": 1024 },
        }),
    }
}

pub fn trigger_source_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "trigger_source".to_string(),
        name: "Trigger Source".to_string(),
        category: "Sources".to_string(),
        inputs: vec![],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "Trigger Out".to_string(),
            data_type: "trigger".to_string(),
        }],
        parameters: json!({
            "mode": { "type": "string", "default": "periodic" },
            "interval_ms": { "type": "number", "default": 100 },
        }),
    }
}

pub fn debug_sink_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "debug_sink".to_string(),
        name: "Debug Sink".to_string(),
        category: "Sinks".to_string(),
        inputs: vec![PortMetadata {
            id: "input".to_string(),
            name: "Data In".to_string(),
            data_type: "any".to_string(),
        }],
        outputs: vec![],
        parameters: json!({
            "log_level": { "type": "string", "default": "info" },
        }),
    }
}

pub fn fft_node_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "fft".to_string(),
        name: "FFT".to_string(),
        category: "Processors".to_string(),
        inputs: vec![PortMetadata {
            id: "input".to_string(),
            name: "Audio In".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "FFT Out".to_string(),
            data_type: "fft_result".to_string(),
        }],
        parameters: json!({
            "window_type": { "type": "string", "default": "hann" },
        }),
    }
}

pub fn gain_node_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "gain".to_string(),
        name: "Gain".to_string(),
        category: "Processors".to_string(),
        inputs: vec![PortMetadata {
            id: "input".to_string(),
            name: "Audio In".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "Audio Out".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        parameters: json!({
            "gain_db": { "type": "number", "default": 0.0 },
        }),
    }
}

pub fn filter_node_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "filter".to_string(),
        name: "Filter".to_string(),
        category: "Processors".to_string(),
        inputs: vec![PortMetadata {
            id: "input".to_string(),
            name: "Audio In".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "Audio Out".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        parameters: json!({
            "type": { "type": "string", "default": "lowpass" },
            "cutoff_hz": { "type": "number", "default": 1000.0 },
        }),
    }
}
```

**Step 2: Add nodes module to main**

Modify `src-tauri/src/main.rs`:

```rust
mod state;
mod commands;
mod nodes;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::nodes::get_node_registry,
            commands::pipeline::deploy_graph,
            commands::pipeline::get_all_pipeline_states,
            commands::pipeline::control_pipeline,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 3: Register nodes in state**

Modify `src-tauri/src/state.rs` to populate registry:

```rust
// Add to imports
use crate::nodes::*;

// Update with_defaults implementation
impl NodeRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(audio_source_metadata());
        registry.register(trigger_source_metadata());
        registry.register(debug_sink_metadata());
        registry.register(fft_node_metadata());
        registry.register(gain_node_metadata());
        registry.register(filter_node_metadata());
        registry
    }
}
```

**Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add src-tauri/src/nodes/ src-tauri/src/state.rs src-tauri/src/main.rs
git commit -m "feat(tauri): add metadata for 6 initial nodes"
```

---

## Task H: First Integration Test

### H1: Test end-to-end flow

**Step 1: Start dev server**

Run: `cd src-tauri && cargo tauri dev`

Expected:
- Frontend builds
- Tauri window opens
- Node palette shows 6 nodes in 3 categories
- Can drag nodes to canvas
- Can connect nodes
- Deploy button exists

**Step 2: Manual testing**

1. Drag AudioSource to canvas
2. Drag FFT to canvas
3. Connect AudioSource output to FFT input
4. Click Deploy button
5. Check console for pipeline ID

Expected: No errors, pipeline ID logged

**Step 3: Document test results**

Create `docs/phase2-manual-test-checklist.md`:

```markdown
# Phase 2 Manual Test Checklist

## Basic UI
- [ ] App launches without errors
- [ ] Node palette visible on left
- [ ] Canvas visible in center
- [ ] Deploy button in top bar

## Node Palette
- [ ] 6 nodes visible
- [ ] Grouped into 3 categories (Sources, Processors, Sinks)
- [ ] Nodes are draggable

## Canvas Interactions
- [ ] Can drag nodes from palette to canvas
- [ ] Nodes render with ports
- [ ] Can connect nodes
- [ ] Minimap shows overview
- [ ] Zoom controls work
- [ ] Can delete nodes (select + Delete key)

## Deployment
- [ ] Deploy button sends graph to backend
- [ ] Pipeline ID returned
- [ ] No console errors
```

**Step 4: Commit**

```bash
git add docs/phase2-manual-test-checklist.md
git commit -m "docs: add Phase 2 manual test checklist"
```

---

## Task I: Add Event System for Pipeline Status

### I1: Implement event emitter in backend

**Files:**
- Modify: `src-tauri/src/commands/pipeline.rs`

**Step 1: Add event emission**

Modify `src-tauri/src/commands/pipeline.rs`:

```rust
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize, Clone)]
pub struct PipelineStatusEvent {
    pub id: String,
    pub state: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn deploy_graph(
    app: AppHandle,
    state: State<'_, AppState>,
    graph: GraphJson,
) -> Result<String, String> {
    let pipeline_id = format!("pipeline_{}", uuid::Uuid::new_v4());

    println!("Deploying graph with {} nodes, {} edges",
             graph.nodes.len(), graph.edges.len());

    // Emit status event
    let _ = app.emit("pipeline-status", PipelineStatusEvent {
        id: pipeline_id.clone(),
        state: "Deploying".to_string(),
        error: None,
    });

    // Simulate deployment delay
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let _ = app.emit("pipeline-status", PipelineStatusEvent {
        id: pipeline_id.clone(),
        state: "Running".to_string(),
        error: None,
    });

    Ok(pipeline_id)
}
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add src-tauri/src/commands/pipeline.rs
git commit -m "feat(tauri): add pipeline status event emission"
```

### I2: Add event listener in frontend

**Files:**
- Create: `src-frontend/src/hooks/useTauriEvents.ts`

**Step 1: Implement event hook**

Create `src-frontend/src/hooks/useTauriEvents.ts`:

```typescript
import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface PipelineStatusEvent {
  id: string;
  state: string;
  error?: string;
}

export function usePipelineStatusEvents(
  callback: (event: PipelineStatusEvent) => void
) {
  useEffect(() => {
    const unlisten = listen<PipelineStatusEvent>('pipeline-status', (event) => {
      callback(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [callback]);
}
```

**Step 2: Add status display to App**

Modify `src-frontend/src/App.tsx`:

```typescript
import { useState } from 'react';
import { usePipelineStatusEvents } from './hooks/useTauriEvents';

function AppContent() {
  const [lastStatus, setLastStatus] = useState<string>('');
  const exportGraph = useFlowStore((state) => state.exportGraph);
  const deployMutation = useDeployGraph();

  usePipelineStatusEvents((event) => {
    setLastStatus(`Pipeline ${event.id}: ${event.state}`);
  });

  const handleDeploy = async () => {
    const graph = exportGraph();
    try {
      const pipelineId = await deployMutation.mutateAsync(graph);
      console.log('Deployed pipeline:', pipelineId);
    } catch (error) {
      console.error('Deploy failed:', error);
    }
  };

  return (
    <div className="flex flex-col h-screen bg-slate-900">
      {/* ... existing top bar ... */}

      {/* ... existing main content ... */}

      {/* Status Bar - Updated */}
      <div className="h-8 bg-slate-800 border-t border-slate-700 flex items-center px-4">
        <span className="text-slate-400 text-sm">{lastStatus || 'Ready'}</span>
      </div>
    </div>
  );
}
```

**Step 3: Verify compilation**

Run: `npm run build`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src-frontend/src/hooks/useTauriEvents.ts src-frontend/src/App.tsx
git commit -m "feat(frontend): add pipeline status event listener"
```

---

## Task J: Add Undo/Redo to Flow Editor

### J1: Implement history in flow store

**Files:**
- Modify: `src-frontend/src/stores/flowStore.ts`

**Step 1: Add history tracking**

Replace `src-frontend/src/stores/flowStore.ts`:

```typescript
import { create } from 'zustand';
import { Node, Edge, Connection, addEdge, applyNodeChanges, applyEdgeChanges, NodeChange, EdgeChange } from '@xyflow/react';

interface FlowSnapshot {
  nodes: Node[];
  edges: Edge[];
}

interface FlowState {
  nodes: Node[];
  edges: Edge[];
  history: FlowSnapshot[];
  historyIndex: number;
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (type: string, position: { x: number; y: number }, metadata: any) => void;
  deleteSelected: () => void;
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
  exportGraph: () => { nodes: any[]; edges: any[] };
}

const saveHistory = (state: FlowState): void => {
  const snapshot = { nodes: state.nodes, edges: state.edges };
  const newHistory = state.history.slice(0, state.historyIndex + 1);
  newHistory.push(snapshot);
  state.history = newHistory.slice(-50); // Keep last 50
  state.historyIndex = state.history.length - 1;
};

export const useFlowStore = create<FlowState>((set, get) => ({
  nodes: [],
  edges: [],
  history: [{ nodes: [], edges: [] }],
  historyIndex: 0,

  onNodesChange: (changes) => {
    set((state) => {
      const newNodes = applyNodeChanges(changes, state.nodes);
      saveHistory(state);
      return { nodes: newNodes };
    });
  },

  onEdgesChange: (changes) => {
    set((state) => {
      const newEdges = applyEdgeChanges(changes, state.edges);
      saveHistory(state);
      return { edges: newEdges };
    });
  },

  onConnect: (connection) => {
    set((state) => {
      const newEdges = addEdge(connection, state.edges);
      saveHistory(state);
      return { edges: newEdges };
    });
  },

  addNode: (type, position, metadata) => {
    set((state) => {
      const newNode: Node = {
        id: `${type}-${Date.now()}`,
        type: 'custom',
        position,
        data: { label: metadata.name, metadata, parameters: {} },
      };
      saveHistory(state);
      return { nodes: [...state.nodes, newNode] };
    });
  },

  deleteSelected: () => {
    set((state) => {
      saveHistory(state);
      return {
        nodes: state.nodes.filter((n) => !n.selected),
        edges: state.edges.filter((e) => !e.selected),
      };
    });
  },

  undo: () => {
    const { history, historyIndex } = get();
    if (historyIndex > 0) {
      const snapshot = history[historyIndex - 1];
      set({
        nodes: snapshot.nodes,
        edges: snapshot.edges,
        historyIndex: historyIndex - 1,
      });
    }
  },

  redo: () => {
    const { history, historyIndex } = get();
    if (historyIndex < history.length - 1) {
      const snapshot = history[historyIndex + 1];
      set({
        nodes: snapshot.nodes,
        edges: snapshot.edges,
        historyIndex: historyIndex + 1,
      });
    }
  },

  canUndo: () => get().historyIndex > 0,
  canRedo: () => get().historyIndex < get().history.length - 1,

  exportGraph: () => {
    const { nodes, edges } = get();
    return {
      nodes: nodes.map((n) => ({
        id: n.id,
        type: n.data.metadata.id,
        position: n.position,
        parameters: n.data.parameters,
      })),
      edges: edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        sourceHandle: e.sourceHandle,
        targetHandle: e.targetHandle,
      })),
    };
  },
}));
```

**Step 2: Add undo/redo buttons to UI**

Modify `src-frontend/src/App.tsx` to add buttons:

```typescript
function AppContent() {
  const [lastStatus, setLastStatus] = useState<string>('');
  const exportGraph = useFlowStore((state) => state.exportGraph);
  const undo = useFlowStore((state) => state.undo);
  const redo = useFlowStore((state) => state.redo);
  const canUndo = useFlowStore((state) => state.canUndo());
  const canRedo = useFlowStore((state) => state.canRedo());
  const deployMutation = useDeployGraph();

  // ... existing event listener ...

  return (
    <div className="flex flex-col h-screen bg-slate-900">
      <div className="h-14 bg-slate-800 border-b border-slate-700 flex items-center px-4">
        <h1 className="text-white text-xl font-bold">StreamLab Core</h1>
        <div className="ml-auto space-x-2">
          <Button onClick={undo} disabled={!canUndo} variant="outline">
            Undo
          </Button>
          <Button onClick={redo} disabled={!canRedo} variant="outline">
            Redo
          </Button>
          <Button onClick={handleDeploy} disabled={deployMutation.isPending}>
            {deployMutation.isPending ? 'Deploying...' : 'Deploy'}
          </Button>
        </div>
      </div>
      {/* ... rest unchanged ... */}
    </div>
  );
}
```

**Step 3: Verify compilation**

Run: `npm run build`
Expected: SUCCESS

**Step 4: Test undo/redo**

Run: `cargo tauri dev`
Test:
1. Add node -> Undo -> node disappears
2. Redo -> node reappears

**Step 5: Commit**

```bash
git add src-frontend/src/stores/flowStore.ts src-frontend/src/App.tsx
git commit -m "feat(frontend): add undo/redo with history tracking"
```

---

## Task K: Final Polish and Documentation

### K1: Add keyboard shortcuts

**Files:**
- Create: `src-frontend/src/hooks/useKeyboardShortcuts.ts`

**Step 1: Implement keyboard shortcuts**

Create `src-frontend/src/hooks/useKeyboardShortcuts.ts`:

```typescript
import { useEffect } from 'react';
import { useFlowStore } from '../stores/flowStore';

export function useKeyboardShortcuts() {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Undo: Ctrl+Z / Cmd+Z
      if ((event.ctrlKey || event.metaKey) && event.key === 'z' && !event.shiftKey) {
        event.preventDefault();
        useFlowStore.getState().undo();
      }

      // Redo: Ctrl+Shift+Z / Cmd+Shift+Z
      if ((event.ctrlKey || event.metaKey) && event.key === 'z' && event.shiftKey) {
        event.preventDefault();
        useFlowStore.getState().redo();
      }

      // Delete: Delete / Backspace
      if (event.key === 'Delete' || event.key === 'Backspace') {
        event.preventDefault();
        useFlowStore.getState().deleteSelected();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);
}
```

**Step 2: Use hook in App**

Modify `src-frontend/src/App.tsx`:

```typescript
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';

function AppContent() {
  useKeyboardShortcuts();
  // ... rest unchanged ...
}
```

**Step 3: Verify**

Run: `npm run build`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src-frontend/src/hooks/useKeyboardShortcuts.ts src-frontend/src/App.tsx
git commit -m "feat(frontend): add keyboard shortcuts (undo/redo/delete)"
```

### K2: Update project documentation

**Files:**
- Modify: `README.md`
- Create: `docs/phase2-completion.md`

**Step 1: Create completion doc**

Create `docs/phase2-completion.md`:

```markdown
# Phase 2 Completion Summary

**Date**: 2025-11-24

## Implemented Features

### Backend (src-tauri)
- ✅ Tauri v2 application structure
- ✅ Node registry with 6 nodes (AudioSource, TriggerSource, DebugSink, FFT, Gain, Filter)
- ✅ Tauri commands: `get_node_registry`, `deploy_graph`, `get_all_pipeline_states`, `control_pipeline`
- ✅ Event system for pipeline status updates
- ✅ AppState management with thread-safe pipeline storage

### Frontend (src-frontend)
- ✅ Vite + React 19 + TypeScript
- ✅ Zustand state management
- ✅ React Flow visual editor
- ✅ shadcn/ui + Tailwind CSS
- ✅ Draggable node palette with 3 categories
- ✅ Node connections with port visualization
- ✅ Undo/redo with 50-step history
- ✅ Keyboard shortcuts (Ctrl+Z, Ctrl+Shift+Z, Delete)
- ✅ Minimap and zoom controls
- ✅ Deploy to backend with status feedback

## File Structure

```
audiotab/
├── src/                    # Core Rust library (Phase 1)
├── src-tauri/              # Tauri backend
│   ├── src/
│   │   ├── commands/       # Tauri command handlers
│   │   ├── nodes/          # Node metadata
│   │   └── state.rs        # App state
│   └── tauri.conf.json
└── src-frontend/           # React frontend
    ├── src/
    │   ├── components/
    │   │   ├── FlowEditor/ # React Flow canvas
    │   │   ├── NodePalette/# Draggable nodes
    │   │   └── ui/         # shadcn components
    │   ├── hooks/          # Tauri integration
    │   ├── stores/         # Zustand stores
    │   └── types/          # TypeScript types
    └── package.json
```

## Usage

### Development

```bash
# Start dev server
cd src-tauri
cargo tauri dev
```

### Build

```bash
# Build production app
cd src-tauri
cargo tauri build
```

### Testing

1. Drag nodes from palette to canvas
2. Connect nodes via ports
3. Click Deploy to send graph to backend
4. Watch status bar for pipeline updates
5. Use Undo/Redo to modify graph

## Next Steps (Phase 3)

- [ ] Python node integration via PyO3
- [ ] Actual pipeline execution (currently placeholder)
- [ ] FFT implementation with rustfft
- [ ] Real-time data visualization
- [ ] Advanced node parameters UI
```

**Step 2: Update README**

Modify `README.md` to add Phase 2 status:

```markdown
## Development Status

### Phase 1: Core Engine ✅ COMPLETE
- [x] Hardware Abstraction Layer (HAL)
- [x] Pipeline State Machine
- [x] Priority-based Scheduling
- [x] Simulated devices (Audio + Trigger)
- [x] Comprehensive tests (48 tests passing)

### Phase 2: Frontend & Builder ✅ COMPLETE
- [x] Tauri v2 desktop application
- [x] React Flow visual editor
- [x] Node palette with 6 initial nodes
- [x] Undo/redo system
- [x] Keyboard shortcuts
- [x] Pipeline deployment via Tauri commands
- [x] Status event system

### Phase 3: Hybrid Runtime & Plugin System 🚧 IN PROGRESS
- [ ] PyO3 Python bridge
- [ ] Dynamic node loading
- [ ] Advanced DSP nodes

---

## Running Phase 2

```bash
# Development
cd src-tauri
cargo tauri dev

# Production build
cargo tauri build
```
```

**Step 3: Commit**

```bash
git add README.md docs/phase2-completion.md
git commit -m "docs: document Phase 2 completion"
```

---

## Completion Checklist

After completing all tasks:

- [ ] All Rust code compiles (`cargo check` in src-tauri)
- [ ] All frontend code builds (`npm run build` in src-frontend)
- [ ] App launches (`cargo tauri dev`)
- [ ] Node palette shows 6 nodes
- [ ] Can drag and connect nodes
- [ ] Deploy button works
- [ ] Status events display
- [ ] Undo/redo works
- [ ] Keyboard shortcuts work
- [ ] Manual test checklist complete
- [ ] Documentation updated

## Notes for Implementation

- **TDD approach**: Where feasible, write tests first (especially for stores and hooks)
- **Frequent commits**: Commit after each subtask completion
- **YAGNI**: Don't add features beyond what's specified
- **DRY**: Reuse components and hooks
- **Type safety**: Use TypeScript types everywhere

## Execution Options

See Task A for starting implementation.
