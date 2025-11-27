use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use audiotab::engine::{AsyncPipeline, PipelineState};
use audiotab::visualization::RingBufferWriter;
use crate::nodes::*;

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<NodeRegistry>,
    pub pipelines: Arc<Mutex<HashMap<String, PipelineHandle>>>,
    pub ring_buffer: Arc<Mutex<RingBufferWriter>>,
}

pub struct PipelineHandle {
    pub id: String,
    pub pipeline: Arc<Mutex<AsyncPipeline>>,
    pub state: Arc<Mutex<PipelineState>>,
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
        registry.register(audio_source_metadata());
        registry.register(trigger_source_metadata());
        registry.register(debug_sink_metadata());
        registry.register(fft_node_metadata());
        registry.register(gain_node_metadata());
        registry.register(filter_node_metadata());
        registry
    }

    pub fn from_inventory() -> Self {
        let mut registry = Self::new();
        for wrapper in inventory::iter::<audiotab::registry::NodeMetadataFactoryWrapper> {
            // Call the factory to create metadata at runtime
            let meta = (wrapper.0)();
            // Convert to serializable metadata
            let serializable_meta = NodeMetadata {
                id: meta.id.clone(),
                name: meta.name.clone(),
                category: meta.category.clone(),
                inputs: meta.inputs.iter().map(|p| PortMetadata {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    data_type: p.data_type.clone(),
                }).collect(),
                outputs: meta.outputs.iter().map(|p| PortMetadata {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    data_type: p.data_type.clone(),
                }).collect(),
                parameters: serde_json::to_value(&meta.parameters).unwrap_or(serde_json::json!([])),
            };
            registry.register(serializable_meta);
        }
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
        // Initialize ring buffer (48kHz, 1 channel for now, 30 seconds)
        let ring_buffer = RingBufferWriter::new(
            "/tmp/audiotab_ringbuf",
            48000,
            1,
            30,
        ).expect("Failed to create ring buffer");

        Self {
            registry: Arc::new(NodeRegistry::with_defaults()),
            pipelines: Arc::new(Mutex::new(HashMap::new())),
            ring_buffer: Arc::new(Mutex::new(ring_buffer)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
