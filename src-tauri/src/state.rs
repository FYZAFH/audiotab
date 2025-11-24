use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use audiotab::engine::{AsyncPipeline, PipelineState};
use crate::nodes::*;

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
        registry.register(audio_source_metadata());
        registry.register(trigger_source_metadata());
        registry.register(debug_sink_metadata());
        registry.register(fft_node_metadata());
        registry.register(gain_node_metadata());
        registry.register(filter_node_metadata());
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
