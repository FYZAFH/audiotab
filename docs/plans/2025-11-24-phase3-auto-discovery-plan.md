# Phase 3: Auto-Discovery Registry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement macro-based auto-discovery node registration system using `#[derive(StreamNode)]` and compile-time registration with the `inventory` crate, eliminating manual node registration.

**Architecture:** Create a proc_macro crate (`audiotab-macros`) that generates `NodeMetadata` from struct annotations. Use `inventory` crate for compile-time registration of all annotated nodes. Replace manual `NodeRegistry::with_defaults()` implementation with automatic discovery from inventory.

**Tech Stack:** Rust proc_macro, syn, quote, inventory crate, async-trait

---

## Task A: Create Proc Macro Crate Structure

### A1: Create proc_macro crate

**Files:**
- Create: `audiotab-macros/Cargo.toml`
- Create: `audiotab-macros/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

**Step 1: Create directory structure**

Run:
```bash
mkdir -p audiotab-macros/src
```

Expected: Directories created

**Step 2: Create proc_macro Cargo.toml**

Create `audiotab-macros/Cargo.toml`:

```toml
[package]
name = "audiotab-macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
syn = { version = "2.0", features = ["full"] }
quote = "1.0"
proc-macro2 = "1.0"
```

**Step 3: Update workspace Cargo.toml**

Modify `Cargo.toml` at workspace root:

```toml
[workspace]
members = [".", "src-tauri", "audiotab-macros"]
resolver = "2"

# ... rest unchanged
```

**Step 4: Create empty lib.rs**

Create `audiotab-macros/src/lib.rs`:

```rust
use proc_macro::TokenStream;

#[proc_macro_derive(StreamNode)]
pub fn derive_stream_node(_input: TokenStream) -> TokenStream {
    // Will implement in next task
    TokenStream::new()
}
```

**Step 5: Verify compilation**

Run: `cargo check -p audiotab-macros`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add audiotab-macros/ Cargo.toml
git commit -m "feat(macros): create proc_macro crate structure"
```

---

## Task B: Implement Core Node Trait

### B1: Define ProcessingNode trait in core library

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/node.rs`
- Modify: `src/lib.rs`

**Step 1: Create core module**

Create `src/core/mod.rs`:

```rust
pub mod node;

pub use node::{ProcessingNode, NodeContext};
```

**Step 2: Define ProcessingNode trait**

Create `src/core/node.rs`:

```rust
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Context passed to nodes during processing
#[derive(Clone, Debug)]
pub struct NodeContext {
    pub node_id: String,
    pub config: Value,
}

/// DataFrame represents data flowing through the pipeline
#[derive(Clone, Debug)]
pub struct DataFrame {
    pub timestamp: u64,
    pub sequence_id: u64,
    pub payload: HashMap<String, Arc<Vec<f64>>>,
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

/// Base trait that all processing nodes must implement
#[async_trait]
pub trait ProcessingNode: Send + Sync {
    /// Initialize the node with configuration
    async fn on_create(&mut self, config: Value) -> Result<()> {
        let _ = config;
        Ok(())
    }

    /// Process a single data frame
    async fn process(&mut self, input: DataFrame) -> Result<DataFrame>;

    /// Cleanup when node is destroyed
    async fn on_destroy(&mut self) -> Result<()> {
        Ok(())
    }
}
```

**Step 3: Export core module**

Modify `src/lib.rs` to add:

```rust
pub mod core;
pub mod engine;
pub mod hal;

pub use core::{ProcessingNode, NodeContext, DataFrame};
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add src/core/ src/lib.rs
git commit -m "feat(core): add ProcessingNode trait and DataFrame"
```

---

## Task C: Add Inventory Dependencies

### C1: Add inventory to workspace

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add inventory to workspace dependencies**

Modify `Cargo.toml`:

```toml
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
inventory = "0.3"

[dev-dependencies]
tokio-test = "0.4"
```

**Step 2: Add to tauri dependencies**

Modify `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri = { version = "2", features = ["devtools"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.40", features = ["full"] }
audiotab = { path = "../" }
uuid = { version = "1.0", features = ["v4"] }
log = "0.4"
tauri-plugin-log = "2.0.0-rc"
inventory = "0.3"
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add Cargo.toml src-tauri/Cargo.toml
git commit -m "feat: add inventory crate for compile-time registration"
```

---

## Task D: Implement NodeMetadata Registry Type

### D1: Create node metadata types

**Files:**
- Create: `src/registry/mod.rs`
- Create: `src/registry/metadata.rs`
- Modify: `src/lib.rs`

**Step 1: Create registry module**

Create `src/registry/mod.rs`:

```rust
pub mod metadata;

pub use metadata::{NodeMetadata, PortMetadata, ParameterSchema, NodeFactory};
```

**Step 2: Define metadata types**

Create `src/registry/metadata.rs`:

```rust
use crate::core::ProcessingNode;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Metadata describing a port (input or output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMetadata {
    pub id: String,
    pub name: String,
    pub data_type: String,
}

/// Schema for a configurable parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub default: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

/// Factory function type for creating node instances
pub type NodeFactory = fn() -> Box<dyn ProcessingNode>;

/// Complete metadata for a node type
#[derive(Clone)]
pub struct NodeMetadata {
    pub id: String,
    pub name: String,
    pub category: String,
    pub inputs: Vec<PortMetadata>,
    pub outputs: Vec<PortMetadata>,
    pub parameters: Vec<ParameterSchema>,
    pub factory: NodeFactory,
}

impl NodeMetadata {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: category.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            parameters: Vec::new(),
            factory: || panic!("No factory set"),
        }
    }

    pub fn with_factory(mut self, factory: NodeFactory) -> Self {
        self.factory = factory;
        self
    }

    pub fn add_input(mut self, id: impl Into<String>, name: impl Into<String>, data_type: impl Into<String>) -> Self {
        self.inputs.push(PortMetadata {
            id: id.into(),
            name: name.into(),
            data_type: data_type.into(),
        });
        self
    }

    pub fn add_output(mut self, id: impl Into<String>, name: impl Into<String>, data_type: impl Into<String>) -> Self {
        self.outputs.push(PortMetadata {
            id: id.into(),
            name: name.into(),
            data_type: data_type.into(),
        });
        self
    }

    pub fn add_parameter(mut self, param: ParameterSchema) -> Self {
        self.parameters.push(param);
        self
    }

    /// Create a new instance of this node type
    pub fn create_instance(&self) -> Box<dyn ProcessingNode> {
        (self.factory)()
    }
}

// Inventory submission type
inventory::collect!(NodeMetadata);
```

**Step 3: Export registry module**

Modify `src/lib.rs`:

```rust
pub mod core;
pub mod engine;
pub mod hal;
pub mod registry;

pub use core::{ProcessingNode, NodeContext, DataFrame};
pub use registry::{NodeMetadata, PortMetadata, ParameterSchema};
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add src/registry/ src/lib.rs
git commit -m "feat(registry): add NodeMetadata types with inventory support"
```

---

## Task E: Implement StreamNode Derive Macro

### E1: Parse node_meta attribute

**Files:**
- Modify: `audiotab-macros/src/lib.rs`
- Create: `audiotab-macros/src/node_meta.rs`

**Step 1: Add dependencies**

Modify `audiotab-macros/Cargo.toml`:

```toml
[package]
name = "audiotab-macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
syn = { version = "2.0", features = ["full", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
darling = "0.20"
```

**Step 2: Create parsing utilities**

Create `audiotab-macros/src/node_meta.rs`:

```rust
use darling::{FromAttributes, FromField};
use syn::{DeriveInput, Fields};

/// Parsed attributes from #[node_meta(...)]
#[derive(Debug, FromAttributes)]
#[darling(attributes(node_meta))]
pub struct NodeMetaArgs {
    pub name: String,
    pub category: String,
}

/// Parsed attributes from #[param(...)]
#[derive(Debug, FromField)]
#[darling(attributes(param))]
pub struct ParamField {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,

    #[darling(default)]
    pub default: Option<String>,

    #[darling(default)]
    pub min: Option<f64>,

    #[darling(default)]
    pub max: Option<f64>,
}

/// Parse inputs/outputs from #[port(...)]
#[derive(Debug, FromField)]
#[darling(attributes(input, output))]
pub struct PortField {
    pub ident: Option<syn::Ident>,

    #[darling(default)]
    pub name: Option<String>,

    #[darling(default)]
    pub data_type: Option<String>,
}

pub fn parse_node_info(input: &DeriveInput) -> darling::Result<NodeMetaArgs> {
    NodeMetaArgs::from_attributes(&input.attrs)
}

pub fn parse_fields(input: &DeriveInput) -> Vec<ParamField> {
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };

    fields
        .iter()
        .filter_map(|f| ParamField::from_field(f).ok())
        .collect()
}
```

**Step 3: Implement derive macro**

Modify `audiotab-macros/src/lib.rs`:

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

mod node_meta;
use node_meta::{parse_node_info, parse_fields};

#[proc_macro_derive(StreamNode, attributes(node_meta, param, input, output))]
pub fn derive_stream_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let node_info = match parse_node_info(&input) {
        Ok(info) => info,
        Err(e) => return e.write_errors().into(),
    };

    let fields = parse_fields(&input);

    let struct_name = &input.ident;
    let node_id = struct_name.to_string().to_lowercase();
    let node_name = &node_info.name;
    let category = &node_info.category;

    // Generate parameters
    let params = fields.iter().filter_map(|f| {
        let field_name = f.ident.as_ref()?.to_string();
        let default_val = f.default.as_ref().map(|s| s.as_str()).unwrap_or("null");
        let type_name = extract_type_name(&f.ty);

        let param_code = if let (Some(min), Some(max)) = (f.min, f.max) {
            quote! {
                audiotab::registry::ParameterSchema {
                    name: #field_name.to_string(),
                    param_type: #type_name.to_string(),
                    default: serde_json::json!(#default_val),
                    min: Some(#min),
                    max: Some(#max),
                }
            }
        } else {
            quote! {
                audiotab::registry::ParameterSchema {
                    name: #field_name.to_string(),
                    param_type: #type_name.to_string(),
                    default: serde_json::json!(#default_val),
                    min: None,
                    max: None,
                }
            }
        };

        Some(param_code)
    });

    let expanded = quote! {
        inventory::submit! {
            audiotab::registry::NodeMetadata::new(
                #node_id,
                #node_name,
                #category,
            )
            .with_factory(|| Box::new(#struct_name::default()))
            #(.add_parameter(#params))*
        }
    };

    TokenStream::from(expanded)
}

fn extract_type_name(ty: &syn::Type) -> &'static str {
    // Simple type extraction for common types
    let type_str = quote!(#ty).to_string();

    if type_str.contains("f64") || type_str.contains("f32") {
        "number"
    } else if type_str.contains("String") || type_str.contains("str") {
        "string"
    } else if type_str.contains("bool") {
        "boolean"
    } else {
        "unknown"
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check -p audiotab-macros`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add audiotab-macros/
git commit -m "feat(macros): implement StreamNode derive macro with metadata parsing"
```

---

## Task F: Add Port Metadata Macros

### F1: Implement input/output attribute parsing

**Files:**
- Modify: `audiotab-macros/src/lib.rs`
- Modify: `audiotab-macros/src/node_meta.rs`

**Step 1: Update PortField parsing**

Modify `audiotab-macros/src/node_meta.rs` to add:

```rust
pub fn parse_ports(input: &DeriveInput) -> (Vec<PortField>, Vec<PortField>) {
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return (Vec::new(), Vec::new()),
        },
        _ => return (Vec::new(), Vec::new()),
    };

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for field in fields.iter() {
        // Check for #[input] attribute
        if field.attrs.iter().any(|attr| attr.path().is_ident("input")) {
            if let Ok(port) = PortField::from_field(field) {
                inputs.push(port);
            }
        }

        // Check for #[output] attribute
        if field.attrs.iter().any(|attr| attr.path().is_ident("output")) {
            if let Ok(port) = PortField::from_field(field) {
                outputs.push(port);
            }
        }
    }

    (inputs, outputs)
}
```

**Step 2: Update derive macro to include ports**

Modify `audiotab-macros/src/lib.rs`:

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

mod node_meta;
use node_meta::{parse_node_info, parse_fields, parse_ports};

#[proc_macro_derive(StreamNode, attributes(node_meta, param, input, output))]
pub fn derive_stream_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let node_info = match parse_node_info(&input) {
        Ok(info) => info,
        Err(e) => return e.write_errors().into(),
    };

    let fields = parse_fields(&input);
    let (inputs, outputs) = parse_ports(&input);

    let struct_name = &input.ident;
    let node_id = struct_name.to_string().to_lowercase();
    let node_name = &node_info.name;
    let category = &node_info.category;

    // Generate input ports
    let input_ports = inputs.iter().map(|port| {
        let port_id = port.ident.as_ref().unwrap().to_string();
        let port_name = port.name.as_ref().unwrap_or(&port_id);
        let data_type = port.data_type.as_ref().map(|s| s.as_str()).unwrap_or("any");

        quote! {
            .add_input(#port_id, #port_name, #data_type)
        }
    });

    // Generate output ports
    let output_ports = outputs.iter().map(|port| {
        let port_id = port.ident.as_ref().unwrap().to_string();
        let port_name = port.name.as_ref().unwrap_or(&port_id);
        let data_type = port.data_type.as_ref().map(|s| s.as_str()).unwrap_or("any");

        quote! {
            .add_output(#port_id, #port_name, #data_type)
        }
    });

    // Generate parameters
    let params = fields.iter().filter_map(|f| {
        let field_name = f.ident.as_ref()?.to_string();

        // Skip fields with #[serde(skip)]
        let has_skip = f.default.is_none();
        if has_skip { return None; }

        let default_val = f.default.as_ref()?.as_str();
        let type_name = extract_type_name(&f.ty);

        let param_code = if let (Some(min), Some(max)) = (f.min, f.max) {
            quote! {
                audiotab::registry::ParameterSchema {
                    name: #field_name.to_string(),
                    param_type: #type_name.to_string(),
                    default: serde_json::json!(#default_val),
                    min: Some(#min),
                    max: Some(#max),
                }
            }
        } else {
            quote! {
                audiotab::registry::ParameterSchema {
                    name: #field_name.to_string(),
                    param_type: #type_name.to_string(),
                    default: serde_json::json!(#default_val),
                    min: None,
                    max: None,
                }
            }
        };

        Some(param_code)
    });

    let expanded = quote! {
        inventory::submit! {
            audiotab::registry::NodeMetadata::new(
                #node_id,
                #node_name,
                #category,
            )
            .with_factory(|| Box::new(#struct_name::default()))
            #(#input_ports)*
            #(#output_ports)*
            #(.add_parameter(#params))*
        }
    };

    TokenStream::from(expanded)
}

fn extract_type_name(ty: &syn::Type) -> &'static str {
    let type_str = quote!(#ty).to_string();

    if type_str.contains("f64") || type_str.contains("f32") {
        "number"
    } else if type_str.contains("String") || type_str.contains("str") {
        "string"
    } else if type_str.contains("bool") {
        "boolean"
    } else {
        "unknown"
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check -p audiotab-macros`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add audiotab-macros/
git commit -m "feat(macros): add input/output port metadata generation"
```

---

## Task G: Create Example Node with Macros

### G1: Implement GainNode using new macros

**Files:**
- Create: `src/nodes/mod.rs`
- Create: `src/nodes/gain.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`

**Step 1: Add macro dependency to workspace**

Modify `Cargo.toml`:

```toml
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
inventory = "0.3"
audiotab-macros = { path = "./audiotab-macros" }

[dev-dependencies]
tokio-test = "0.4"
```

**Step 2: Create nodes module**

Create `src/nodes/mod.rs`:

```rust
pub mod gain;

pub use gain::GainNode;
```

**Step 3: Implement GainNode with macros**

Create `src/nodes/gain.rs`:

```rust
use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize, Default)]
#[node_meta(name = "Gain", category = "Processors")]
pub struct GainNode {
    #[param(default = "0.0", min = -60.0, max = 20.0)]
    pub gain_db: f64,

    #[serde(skip)]
    gain_linear: f64,
}

#[async_trait]
impl ProcessingNode for GainNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        if let Some(gain_db) = config.get("gain_db").and_then(|v| v.as_f64()) {
            self.gain_db = gain_db;
        }

        // Convert dB to linear
        self.gain_linear = 10_f64.powf(self.gain_db / 20.0);

        Ok(())
    }

    async fn process(&mut self, mut frame: DataFrame) -> Result<()> {
        // Apply gain to all payload channels
        for (_key, data) in frame.payload.iter_mut() {
            let mut samples = data.as_ref().clone();
            for sample in samples.iter_mut() {
                *sample *= self.gain_linear;
            }
            *data = std::sync::Arc::new(samples);
        }

        Ok(frame)
    }
}
```

**Step 4: Export nodes module**

Modify `src/lib.rs`:

```rust
pub mod core;
pub mod engine;
pub mod hal;
pub mod registry;
pub mod nodes;

pub use core::{ProcessingNode, NodeContext, DataFrame};
pub use registry::{NodeMetadata, PortMetadata, ParameterSchema};
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: SUCCESS (or helpful error about missing port attributes)

**Step 6: Fix GainNode to add ports**

If compilation fails, update `src/nodes/gain.rs`:

```rust
#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Gain", category = "Processors")]
pub struct GainNode {
    #[input(name = "Audio In", data_type = "audio_frame")]
    _input: (),

    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "0.0", min = -60.0, max = 20.0)]
    pub gain_db: f64,

    #[serde(skip)]
    gain_linear: f64,
}

impl Default for GainNode {
    fn default() -> Self {
        Self {
            _input: (),
            _output: (),
            gain_db: 0.0,
            gain_linear: 1.0,
        }
    }
}
```

**Step 7: Verify compilation again**

Run: `cargo check`
Expected: SUCCESS

**Step 8: Commit**

```bash
git add src/nodes/ src/lib.rs Cargo.toml
git commit -m "feat(nodes): implement GainNode using StreamNode macro"
```

---

## Task H: Update NodeRegistry to Use Inventory

### H1: Replace manual registration with inventory

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Update NodeRegistry implementation**

Modify `src-tauri/src/state.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use audiotab::engine::{AsyncPipeline, PipelineState};
use audiotab::registry::NodeMetadata;

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
    nodes: HashMap<String, NodeMetadata>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeMetadataDto {
    pub id: String,
    pub name: String,
    pub category: String,
    pub inputs: Vec<PortMetadataDto>,
    pub outputs: Vec<PortMetadataDto>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortMetadataDto {
    pub id: String,
    pub name: String,
    pub data_type: String,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, meta: NodeMetadata) {
        self.nodes.insert(meta.id.clone(), meta);
    }

    pub fn list_nodes(&self) -> Vec<NodeMetadataDto> {
        self.nodes
            .values()
            .map(|meta| NodeMetadataDto {
                id: meta.id.clone(),
                name: meta.name.clone(),
                category: meta.category.clone(),
                inputs: meta.inputs.iter().map(|p| PortMetadataDto {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    data_type: p.data_type.clone(),
                }).collect(),
                outputs: meta.outputs.iter().map(|p| PortMetadataDto {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    data_type: p.data_type.clone(),
                }).collect(),
                parameters: serde_json::to_value(&meta.parameters).unwrap_or_default(),
            })
            .collect()
    }

    pub fn from_inventory() -> Self {
        let mut registry = Self::new();

        // Collect all nodes from inventory
        for meta in inventory::iter::<NodeMetadata> {
            registry.register(meta.clone());
        }

        registry
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::from_inventory()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(NodeRegistry::from_inventory()),
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

**Step 2: Update main.rs to import nodes**

Modify `src-tauri/src/main.rs` to add:

```rust
mod state;
mod commands;

use state::AppState;

fn main() {
    // Import all nodes to trigger inventory registration
    audiotab::nodes::GainNode;

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

**Step 3: Remove old nodes module**

Run:
```bash
rm -rf src-tauri/src/nodes/
```

**Step 4: Update commands/nodes.rs**

Modify `src-tauri/src/commands/nodes.rs`:

```rust
use crate::state::{AppState, NodeMetadataDto};
use tauri::State;

#[tauri::command]
pub fn get_node_registry(state: State<AppState>) -> Vec<NodeMetadataDto> {
    state.registry.list_nodes()
}
```

**Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(tauri): replace manual node registration with inventory auto-discovery"
```

---

## Task I: Migrate Remaining Nodes to Macros

### I1: Implement AudioSourceNode

**Files:**
- Create: `src/nodes/audio_source.rs`
- Modify: `src/nodes/mod.rs`

**Step 1: Implement AudioSourceNode**

Create `src/nodes/audio_source.rs`:

```rust
use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Audio Source", category = "Sources")]
pub struct AudioSourceNode {
    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "48000", min = 8000.0, max = 192000.0)]
    pub sample_rate: u32,

    #[param(default = "1024", min = 64.0, max = 8192.0)]
    pub buffer_size: u32,

    #[serde(skip)]
    sequence: u64,
}

impl Default for AudioSourceNode {
    fn default() -> Self {
        Self {
            _output: (),
            sample_rate: 48000,
            buffer_size: 1024,
            sequence: 0,
        }
    }
}

#[async_trait]
impl ProcessingNode for AudioSourceNode {
    async fn on_create(&mut self, config: serde_json::Value) -> Result<()> {
        if let Some(sr) = config.get("sample_rate").and_then(|v| v.as_u64()) {
            self.sample_rate = sr as u32;
        }
        if let Some(bs) = config.get("buffer_size").and_then(|v| v.as_u64()) {
            self.buffer_size = bs as u32;
        }
        Ok(())
    }

    async fn process(&mut self, mut frame: DataFrame) -> Result<DataFrame> {
        // Generate silent audio for now
        let samples = vec![0.0; self.buffer_size as usize];
        frame.payload.insert(
            "main_channel".to_string(),
            std::sync::Arc::new(samples),
        );

        self.sequence += 1;
        frame.sequence_id = self.sequence;

        Ok(frame)
    }
}
```

**Step 2: Export AudioSourceNode**

Modify `src/nodes/mod.rs`:

```rust
pub mod gain;
pub mod audio_source;

pub use gain::GainNode;
pub use audio_source::AudioSourceNode;
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src/nodes/
git commit -m "feat(nodes): implement AudioSourceNode with macro"
```

### I2: Implement remaining nodes

**Files:**
- Create: `src/nodes/trigger_source.rs`
- Create: `src/nodes/debug_sink.rs`
- Create: `src/nodes/fft.rs`
- Create: `src/nodes/filter.rs`
- Modify: `src/nodes/mod.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Implement TriggerSourceNode**

Create `src/nodes/trigger_source.rs`:

```rust
use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Trigger Source", category = "Sources")]
pub struct TriggerSourceNode {
    #[output(name = "Trigger Out", data_type = "trigger")]
    _output: (),

    #[param(default = "\"periodic\"")]
    pub mode: String,

    #[param(default = "100", min = 1.0, max = 10000.0)]
    pub interval_ms: u64,
}

impl Default for TriggerSourceNode {
    fn default() -> Self {
        Self {
            _output: (),
            mode: "periodic".to_string(),
            interval_ms: 100,
        }
    }
}

#[async_trait]
impl ProcessingNode for TriggerSourceNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        // Placeholder - just pass through
        Ok(frame)
    }
}
```

**Step 2: Implement DebugSinkNode**

Create `src/nodes/debug_sink.rs`:

```rust
use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Debug Sink", category = "Sinks")]
pub struct DebugSinkNode {
    #[input(name = "Data In", data_type = "any")]
    _input: (),

    #[param(default = "\"info\"")]
    pub log_level: String,
}

impl Default for DebugSinkNode {
    fn default() -> Self {
        Self {
            _input: (),
            log_level: "info".to_string(),
        }
    }
}

#[async_trait]
impl ProcessingNode for DebugSinkNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        println!("[{}] Frame {} with {} channels",
                 self.log_level,
                 frame.sequence_id,
                 frame.payload.len());
        Ok(frame)
    }
}
```

**Step 3: Implement FFTNode**

Create `src/nodes/fft.rs`:

```rust
use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "FFT", category = "Processors")]
pub struct FFTNode {
    #[input(name = "Audio In", data_type = "audio_frame")]
    _input: (),

    #[output(name = "FFT Out", data_type = "fft_result")]
    _output: (),

    #[param(default = "\"hann\"")]
    pub window_type: String,
}

impl Default for FFTNode {
    fn default() -> Self {
        Self {
            _input: (),
            _output: (),
            window_type: "hann".to_string(),
        }
    }
}

#[async_trait]
impl ProcessingNode for FFTNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        // Placeholder - just pass through
        // Real FFT implementation will come in next phase
        Ok(frame)
    }
}
```

**Step 4: Implement FilterNode**

Create `src/nodes/filter.rs`:

```rust
use crate::core::{ProcessingNode, DataFrame};
use anyhow::Result;
use async_trait::async_trait;
use audiotab_macros::StreamNode;
use serde::{Deserialize, Serialize};

#[derive(StreamNode, Debug, Clone, Serialize, Deserialize)]
#[node_meta(name = "Filter", category = "Processors")]
pub struct FilterNode {
    #[input(name = "Audio In", data_type = "audio_frame")]
    _input: (),

    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "\"lowpass\"")]
    pub filter_type: String,

    #[param(default = "1000.0", min = 20.0, max = 20000.0)]
    pub cutoff_hz: f64,
}

impl Default for FilterNode {
    fn default() -> Self {
        Self {
            _input: (),
            _output: (),
            filter_type: "lowpass".to_string(),
            cutoff_hz: 1000.0,
        }
    }
}

#[async_trait]
impl ProcessingNode for FilterNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        // Placeholder - just pass through
        Ok(frame)
    }
}
```

**Step 5: Update nodes module**

Modify `src/nodes/mod.rs`:

```rust
pub mod gain;
pub mod audio_source;
pub mod trigger_source;
pub mod debug_sink;
pub mod fft;
pub mod filter;

pub use gain::GainNode;
pub use audio_source::AudioSourceNode;
pub use trigger_source::TriggerSourceNode;
pub use debug_sink::DebugSinkNode;
pub use fft::FFTNode;
pub use filter::FilterNode;
```

**Step 6: Update main.rs imports**

Modify `src-tauri/src/main.rs`:

```rust
mod state;
mod commands;

use state::AppState;

fn main() {
    // Import all nodes to trigger inventory registration
    use audiotab::nodes::*;
    let _ = (
        GainNode::default(),
        AudioSourceNode::default(),
        TriggerSourceNode::default(),
        DebugSinkNode::default(),
        FFTNode::default(),
        FilterNode::default(),
    );

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

**Step 7: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: SUCCESS

**Step 8: Commit**

```bash
git add src/nodes/ src-tauri/src/main.rs
git commit -m "feat(nodes): migrate all 6 nodes to StreamNode macro system"
```

---

## Task J: Test Auto-Discovery System

### J1: Create integration test

**Files:**
- Create: `tests/node_registry_test.rs`

**Step 1: Write registry test**

Create `tests/node_registry_test.rs`:

```rust
use audiotab::registry::NodeMetadata;

#[test]
fn test_inventory_collects_all_nodes() {
    // Force import of all nodes
    use audiotab::nodes::*;
    let _ = (
        GainNode::default(),
        AudioSourceNode::default(),
        TriggerSourceNode::default(),
        DebugSinkNode::default(),
        FFTNode::default(),
        FilterNode::default(),
    );

    // Collect from inventory
    let nodes: Vec<&NodeMetadata> = inventory::iter::<NodeMetadata>().collect();

    assert!(nodes.len() >= 6, "Expected at least 6 nodes, found {}", nodes.len());

    // Check specific nodes exist
    let node_ids: Vec<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    assert!(node_ids.contains(&"gainnode"), "GainNode not found");
    assert!(node_ids.contains(&"audiosourcenode"), "AudioSourceNode not found");
    assert!(node_ids.contains(&"triggersourcenode"), "TriggerSourceNode not found");
    assert!(node_ids.contains(&"debugsinknode"), "DebugSinkNode not found");
    assert!(node_ids.contains(&"fftnode"), "FFTNode not found");
    assert!(node_ids.contains(&"filternode"), "FilterNode not found");
}

#[test]
fn test_node_metadata_has_correct_structure() {
    use audiotab::nodes::GainNode;
    let _ = GainNode::default();

    let nodes: Vec<&NodeMetadata> = inventory::iter::<NodeMetadata>().collect();
    let gain_node = nodes.iter().find(|n| n.id == "gainnode").expect("GainNode not found");

    assert_eq!(gain_node.name, "Gain");
    assert_eq!(gain_node.category, "Processors");
    assert_eq!(gain_node.inputs.len(), 1);
    assert_eq!(gain_node.outputs.len(), 1);
    assert!(gain_node.parameters.len() > 0, "Expected parameters");

    // Check gain_db parameter exists
    let gain_param = gain_node.parameters.iter()
        .find(|p| p.name == "gain_db")
        .expect("gain_db parameter not found");

    assert_eq!(gain_param.param_type, "number");
    assert_eq!(gain_param.min, Some(-60.0));
    assert_eq!(gain_param.max, Some(20.0));
}

#[test]
fn test_node_factory_creates_instance() {
    use audiotab::nodes::GainNode;
    let _ = GainNode::default();

    let nodes: Vec<&NodeMetadata> = inventory::iter::<NodeMetadata>().collect();
    let gain_node = nodes.iter().find(|n| n.id == "gainnode").expect("GainNode not found");

    let instance = gain_node.create_instance();

    // Verify we can create an instance
    assert!(std::any::type_name_of_val(&*instance).contains("GainNode"));
}
```

**Step 2: Run tests**

Run: `cargo test test_inventory`
Expected: 3 tests pass

**Step 3: Commit**

```bash
git add tests/node_registry_test.rs
git commit -m "test: add integration tests for auto-discovery registry"
```

---

## Task K: Test Frontend Integration

### K1: Start dev server and verify nodes appear

**Step 1: Start Tauri dev server**

Run: `cd src-tauri && cargo tauri dev`
Expected: App launches, shows 6 nodes in palette

**Step 2: Verify node palette**

Manual verification:
- [ ] Node palette shows 6 nodes
- [ ] Nodes grouped into 3 categories (Sources, Processors, Sinks)
- [ ] AudioSource in Sources
- [ ] TriggerSource in Sources
- [ ] Gain in Processors
- [ ] FFT in Processors
- [ ] Filter in Processors
- [ ] DebugSink in Sinks

**Step 3: Test drag and drop**

Manual verification:
- [ ] Can drag Gain node to canvas
- [ ] Node shows correct ports (1 input, 1 output)
- [ ] Can connect AudioSource -> Gain -> DebugSink
- [ ] Deploy button works

**Step 4: Check browser console**

Open DevTools, check console for any errors
Expected: No errors related to node metadata

**Step 5: Document test results**

Create `docs/phase3-manual-test-results.md`:

```markdown
# Phase 3 Manual Test Results

**Date**: 2025-11-24

## Auto-Discovery System

- [x] 6 nodes registered via inventory
- [x] Node metadata correctly generated from macros
- [x] Nodes visible in frontend palette
- [x] Correct categories (Sources, Processors, Sinks)
- [x] Ports correctly defined
- [x] Parameters correctly exposed

## Macro System

- [x] `#[derive(StreamNode)]` works
- [x] `#[node_meta(...)]` correctly parses name and category
- [x] `#[param(...)]` correctly generates parameter schema
- [x] `#[input(...)]` and `#[output(...)]` generate ports
- [x] Factory function creates instances

## Known Limitations

- Parameters with string defaults need quoted strings
- Port attributes require placeholder fields (e.g., `_input: ()`)
- Node ID is auto-generated from struct name (lowercase)

## Next Steps

- Add better error messages in macro parsing
- Support more complex parameter types
- Add validation for port connections
```

**Step 6: Commit**

```bash
git add docs/phase3-manual-test-results.md
git commit -m "docs: add Phase 3 manual test results"
```

---

## Task L: Update Documentation

### L1: Create Phase 3 completion doc

**Files:**
- Create: `docs/phase3-completion.md`
- Modify: `README.md`

**Step 1: Create completion summary**

Create `docs/phase3-completion.md`:

```markdown
# Phase 3 Completion Summary

**Date**: 2025-11-24

## Implemented Features

### Auto-Discovery Registry System
- ✅ Procedural macro crate (`audiotab-macros`)
- ✅ `#[derive(StreamNode)]` macro for automatic registration
- ✅ `#[node_meta(name, category)]` attribute for node metadata
- ✅ `#[param(default, min, max)]` attribute for parameters
- ✅ `#[input(...)]` and `#[output(...)]` attributes for ports
- ✅ `inventory` crate integration for compile-time registration
- ✅ Automatic NodeMetadata generation
- ✅ Zero-overhead factory function generation

### Developer Experience

**Before (Manual Registration):**
```rust
// In metadata.rs
pub fn gain_node_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "gain".to_string(),
        name: "Gain".to_string(),
        category: "Processors".to_string(),
        inputs: vec![PortMetadata { /* ... */ }],
        outputs: vec![PortMetadata { /* ... */ }],
        parameters: json!({ /* ... */ }),
    }
}

// In state.rs
registry.register(gain_node_metadata());
```

**After (Auto-Discovery):**
```rust
#[derive(StreamNode, Serialize, Deserialize, Default)]
#[node_meta(name = "Gain", category = "Processors")]
pub struct GainNode {
    #[input(name = "Audio In", data_type = "audio_frame")]
    _input: (),

    #[output(name = "Audio Out", data_type = "audio_frame")]
    _output: (),

    #[param(default = "0.0", min = -60.0, max = 20.0)]
    pub gain_db: f64,
}

// No manual registration needed!
```

### Migrated Nodes

All 6 existing nodes migrated to new system:
1. **AudioSourceNode** - Generates audio frames
2. **TriggerSourceNode** - Emits trigger signals
3. **GainNode** - Applies gain to audio
4. **FFTNode** - Frequency analysis (placeholder)
5. **FilterNode** - Audio filtering (placeholder)
6. **DebugSinkNode** - Logs data frames

## Architecture

```
audiotab/
├── src/
│   ├── core/
│   │   └── node.rs           # ProcessingNode trait
│   ├── registry/
│   │   └── metadata.rs       # NodeMetadata types
│   └── nodes/
│       ├── gain.rs           # Using StreamNode macro
│       ├── audio_source.rs
│       └── ...
├── audiotab-macros/
│   ├── src/
│   │   ├── lib.rs            # Derive macro implementation
│   │   └── node_meta.rs      # Attribute parsing
│   └── Cargo.toml
└── src-tauri/
    └── src/
        └── state.rs          # NodeRegistry::from_inventory()
```

## Usage

### Adding a New Node

```rust
// src/nodes/my_node.rs

use audiotab_macros::StreamNode;
use crate::core::{ProcessingNode, DataFrame};

#[derive(StreamNode, Default, Serialize, Deserialize)]
#[node_meta(name = "My Custom Node", category = "Custom")]
pub struct MyNode {
    #[input(name = "Input", data_type = "audio_frame")]
    _input: (),

    #[output(name = "Output", data_type = "audio_frame")]
    _output: (),

    #[param(default = "1.0", min = 0.0, max = 10.0)]
    pub multiplier: f64,
}

#[async_trait]
impl ProcessingNode for MyNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        // Your processing logic
        Ok(frame)
    }
}
```

Node automatically appears in frontend palette!

## Testing

```bash
# Run auto-discovery tests
cargo test test_inventory

# Start dev server
cd src-tauri && cargo tauri dev
```

## Benefits

1. **Zero Boilerplate**: No manual metadata registration
2. **Compile-Time Safety**: Errors caught at build time
3. **Automatic Frontend Sync**: Metadata changes immediately reflected
4. **DRY Principle**: Single source of truth for node definition
5. **Easy Onboarding**: New developers just write annotated structs

## Limitations

- String parameter defaults must be quoted: `default = "\"hello\""`
- Port attributes need placeholder fields (technical limitation)
- Node ID auto-generated from struct name (can't customize yet)

## Next Steps (Phase 4)

- [ ] Real FFT implementation with rustfft
- [ ] Real-time visualization with shared memory
- [ ] Advanced DSP nodes (STFT, filtering)
- [ ] Python node integration (deferred from Phase 3)
```

**Step 2: Update README**

Modify `README.md`:

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

### Phase 3: Auto-Discovery Registry ✅ COMPLETE
- [x] Procedural macro system (`#[derive(StreamNode)]`)
- [x] Attribute macros for metadata (`#[node_meta]`, `#[param]`, `#[input]`, `#[output]`)
- [x] Compile-time registration with `inventory` crate
- [x] Automatic NodeMetadata generation
- [x] All 6 nodes migrated to new system
- [x] Zero-boilerplate node registration

### Phase 4: Streaming & Visualization 🚧 NEXT
- [ ] Real FFT implementation with rustfft
- [ ] Shared memory ring buffer
- [ ] WebGL plotting
- [ ] Real-time waveform display

---

## Adding New Nodes

```rust
#[derive(StreamNode, Default, Serialize, Deserialize)]
#[node_meta(name = "My Node", category = "Custom")]
pub struct MyNode {
    #[param(default = "1.0", min = 0.0, max = 10.0)]
    pub gain: f64,
}

#[async_trait]
impl ProcessingNode for MyNode {
    async fn process(&mut self, frame: DataFrame) -> Result<DataFrame> {
        // Your logic here
        Ok(frame)
    }
}
```

Node automatically appears in the frontend!
```

**Step 3: Commit**

```bash
git add docs/phase3-completion.md README.md
git commit -m "docs: document Phase 3 completion with auto-discovery system"
```

---

## Completion Checklist

- [ ] All Rust code compiles (`cargo check`)
- [ ] Macro crate compiles (`cargo check -p audiotab-macros`)
- [ ] Core library compiles with new nodes
- [ ] Tauri app compiles (`cd src-tauri && cargo check`)
- [ ] Integration tests pass (`cargo test test_inventory`)
- [ ] App launches (`cargo tauri dev`)
- [ ] 6 nodes visible in frontend palette
- [ ] Nodes correctly categorized
- [ ] Can drag/drop and connect nodes
- [ ] Deploy still works
- [ ] No console errors
- [ ] Documentation complete

## Notes for Implementation

- **Follow TDD**: Write failing tests, implement minimal code to pass
- **Frequent commits**: After each subtask completion
- **YAGNI**: Don't add features beyond spec (e.g., custom node IDs)
- **DRY**: Reuse parsing logic in macros
- **Type safety**: Use syn/quote correctly for proc macros
- **Error messages**: Macro errors should be clear and actionable

## Execution Options

This plan is ready for execution using the superpowers:executing-plans skill.
