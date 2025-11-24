# Phase 2: Frontend & Builder - Design Document

**Date**: 2025-11-24
**Status**: Approved

## Overview

Phase 2 adds a Tauri desktop application with React Flow-based visual pipeline editor. Users can drag-and-drop nodes to build processing graphs and deploy them to the Rust backend.

## Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Project structure | Integrated monorepo | Tight coupling between frontend/backend |
| Frontend framework | Vite + React 19 + TypeScript | Fast dev, official Tauri support |
| State management | Zustand | Simple API, minimal boilerplate |
| UI components | shadcn/ui + Tailwind CSS | Matches spec, accessible, customizable |
| Node registry access | Tauri Commands (HTTP deferred) | Simpler for Phase 2 |
| Node registration | Static registry, developer-managed | Predictable, testable |
| Initial nodes | AudioSource, TriggerSource, DebugSink, FFT, Gain, Filter | Core processing set for framework validation |
| Hardware support | Simulated only (deferred) | Separate module later |
| UI scope | Full functionality, minimal polish | Grouping, zoom, undo/redo, minimap |
| Status updates | Events + State Snapshots | Robust real-time + sync |

## Project Structure

```
audiotab/
├── Cargo.toml              # Workspace root (updated)
├── src/                    # Existing Rust library (unchanged)
│   ├── engine/
│   ├── hal/
│   └── lib.rs
├── src-tauri/              # Tauri backend (NEW)
│   ├── Cargo.toml          # Tauri app crate
│   ├── src/
│   │   ├── main.rs         # Tauri entry point
│   │   ├── commands/       # Tauri command handlers
│   │   │   ├── mod.rs
│   │   │   ├── nodes.rs    # get_node_registry
│   │   │   └── pipeline.rs # deploy_graph, get_states
│   │   └── state.rs        # App state (pipelines, registry)
│   └── tauri.conf.json
├── src-frontend/           # React frontend (NEW)
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── stores/         # Zustand stores
│       ├── components/     # React components
│       │   ├── FlowEditor/ # React Flow canvas
│       │   ├── NodePalette/
│       │   └── ui/         # shadcn components
│       ├── hooks/          # Tauri invoke/listen hooks
│       └── types/          # TypeScript types
└── docs/
```

## Architecture

### Backend (src-tauri)

#### Tauri Commands

```rust
// nodes.rs
#[tauri::command]
fn get_node_registry(state: State<AppState>) -> Vec<NodeMetadata> {
    state.registry.list_nodes()
}

// pipeline.rs
#[tauri::command]
async fn deploy_graph(state: State<AppState>, graph: GraphJson) -> Result<String, String> {
    // Validates graph, creates pipeline, returns pipeline_id
}

#[tauri::command]
fn get_all_pipeline_states(state: State<AppState>) -> Vec<PipelineStatus> {
    state.pipelines.get_all_states()
}

#[tauri::command]
async fn control_pipeline(state: State<AppState>, id: String, action: PipelineAction) -> Result<(), String> {
    // start, stop, pause
}
```

#### Tauri Events

```rust
// Emitted when pipeline state changes
app.emit("pipeline-status", PipelineStatusEvent {
    id: String,
    state: PipelineState,
    error: Option<String>,
})?;
```

#### Node Registry

Static registration of available nodes:

```rust
pub struct NodeMetadata {
    pub id: String,           // "audio_source"
    pub name: String,         // "Audio Source"
    pub category: String,     // "Sources"
    pub inputs: Vec<PortMetadata>,
    pub outputs: Vec<PortMetadata>,
    pub parameters: JsonSchema,  // For dynamic UI generation
}

impl NodeRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(AudioSourceMeta::metadata());
        registry.register(TriggerSourceMeta::metadata());
        registry.register(DebugSinkMeta::metadata());
        registry.register(FFTNodeMeta::metadata());
        registry.register(GainNodeMeta::metadata());
        registry.register(FilterNodeMeta::metadata());
        registry
    }
}
```

### Frontend (src-frontend)

#### Zustand Stores

```typescript
// stores/flowStore.ts
interface FlowStore {
  nodes: Node[];
  edges: Edge[];
  addNode: (type: string, position: XYPosition) => void;
  connect: (connection: Connection) => void;
  deleteSelected: () => void;
  undo: () => void;
  redo: () => void;
  exportGraph: () => GraphJson;
}

// stores/pipelineStore.ts
interface PipelineStore {
  pipelines: Map<string, PipelineStatus>;
  deployGraph: (graph: GraphJson) => Promise<string>;
  controlPipeline: (id: string, action: PipelineAction) => Promise<void>;
}
```

#### Components

```
FlowEditor/
├── FlowEditor.tsx      # Main React Flow canvas
├── BaseNode.tsx        # Generic node component
├── NodePort.tsx        # Input/output handles
└── EdgeLabel.tsx       # Edge styling

NodePalette/
├── NodePalette.tsx     # Sidebar with draggable nodes
├── NodeCategory.tsx    # Collapsible category
└── DraggableNode.tsx   # Drag source

StatusBar/
├── StatusBar.tsx       # Bottom status bar
└── PipelineStatus.tsx  # Individual pipeline status
```

#### Tauri Hooks

```typescript
// hooks/useTauriCommands.ts
export function useNodeRegistry() {
  return useQuery(['nodes'], () => invoke<NodeMetadata[]>('get_node_registry'));
}

export function useDeployGraph() {
  return useMutation((graph: GraphJson) => invoke<string>('deploy_graph', { graph }));
}

// hooks/useTauriEvents.ts
export function usePipelineStatus(callback: (status: PipelineStatusEvent) => void) {
  useEffect(() => {
    const unlisten = listen<PipelineStatusEvent>('pipeline-status', (event) => {
      callback(event.payload);
    });
    return () => { unlisten.then(fn => fn()); };
  }, [callback]);
}
```

## Initial Node Set

### Sources
- **AudioSource**: Wraps SimulatedAudioSource, outputs audio frames
- **TriggerSource**: Wraps SimulatedTriggerSource, outputs trigger events

### Processors
- **FFTNode**: Computes FFT on audio frames (rustfft)
- **GainNode**: Multiplies audio by gain factor
- **FilterNode**: Simple lowpass/highpass filter

### Sinks
- **DebugSink**: Logs received data, shows in UI console

## UI Features

### Phase 2 Scope
- [x] Drag nodes from palette to canvas
- [x] Connect nodes via ports
- [x] Delete nodes/edges
- [x] Export graph to JSON
- [x] Deploy to backend
- [x] Minimap
- [x] Zoom controls
- [x] Center view button
- [x] Node grouping/subgraphs
- [x] Undo/redo
- [x] Auto-layout
- [x] Dark theme (basic)

### Deferred
- [ ] Live data visualization on edges
- [ ] Advanced theming/polish
- [ ] Keyboard shortcuts
- [ ] Context menus

## Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontend                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ NodePalette  │───>│  FlowEditor  │───>│  StatusBar   │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                   │                   ▲               │
│         │                   │                   │               │
│         ▼                   ▼                   │               │
│  ┌─────────────────────────────────────────────────────┐       │
│  │                    Zustand Stores                    │       │
│  └─────────────────────────────────────────────────────┘       │
│                            │                   ▲               │
└────────────────────────────│───────────────────│───────────────┘
                             │ invoke()          │ events
                             ▼                   │
┌────────────────────────────────────────────────────────────────┐
│                      Tauri Commands                             │
│  get_node_registry │ deploy_graph │ control_pipeline           │
└────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌────────────────────────────────────────────────────────────────┐
│                     Rust Engine (src/)                          │
│  NodeRegistry │ AsyncPipeline │ PipelineScheduler              │
└────────────────────────────────────────────────────────────────┘
```

## Testing Strategy

### Backend Tests
- Unit tests for Tauri commands (mock state)
- Integration tests for graph deployment
- Node registry validation

### Frontend Tests
- Component tests with React Testing Library
- Flow interaction tests
- Zustand store tests

### E2E Tests
- Tauri + WebDriver for full app testing (Phase 2.5)

## Dependencies

### Rust (src-tauri/Cargo.toml)
```toml
[dependencies]
tauri = { version = "2", features = ["devtools"] }
audiotab = { path = "../" }  # Main library
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
rustfft = "6"  # For FFTNode
```

### Frontend (src-frontend/package.json)
```json
{
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "@xyflow/react": "^12.0.0",
    "@tauri-apps/api": "^2.0.0",
    "zustand": "^5.0.0",
    "@tanstack/react-query": "^5.0.0"
  },
  "devDependencies": {
    "vite": "^6.0.0",
    "@vitejs/plugin-react": "^4.0.0",
    "typescript": "^5.0.0",
    "tailwindcss": "^3.4.0",
    "@types/react": "^19.0.0"
  }
}
```
