use crate::core::ProcessingNode;
use serde::{Deserialize, Serialize};

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

// Factory type for creating node metadata at runtime
pub type NodeMetadataFactory = fn() -> NodeMetadata;

// Wrapper for inventory collection
pub struct NodeMetadataFactoryWrapper(pub NodeMetadataFactory);

// Inventory submission type
inventory::collect!(NodeMetadataFactoryWrapper);
