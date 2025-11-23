use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use crate::core::{ProcessingNode, DataFrame};
use crate::nodes::{SineGenerator, Gain, Print};

pub struct Pipeline {
    nodes: HashMap<String, Box<dyn ProcessingNode>>,
    connections: Vec<(String, String)>,
}

impl Pipeline {
    pub async fn from_json(config: Value) -> Result<Self> {
        let mut nodes: HashMap<String, Box<dyn ProcessingNode>> = HashMap::new();
        let mut connections = Vec::new();

        // Parse nodes
        if let Some(nodes_array) = config["nodes"].as_array() {
            for node_config in nodes_array {
                let id = node_config["id"].as_str()
                    .ok_or(anyhow!("Node missing id"))?
                    .to_string();
                let node_type = node_config["type"].as_str()
                    .ok_or(anyhow!("Node missing type"))?;
                let node_cfg = node_config["config"].clone();

                let mut node: Box<dyn ProcessingNode> = match node_type {
                    "SineGenerator" => Box::new(SineGenerator::new()),
                    "Gain" => Box::new(Gain::new()),
                    "Print" => Box::new(Print::new()),
                    _ => return Err(anyhow!("Unknown node type: {}", node_type)),
                };

                node.on_create(node_cfg).await?;
                nodes.insert(id, node);
            }
        }

        // Parse connections
        if let Some(conns_array) = config["connections"].as_array() {
            for conn in conns_array {
                let from = conn["from"].as_str()
                    .ok_or(anyhow!("Connection missing from"))?
                    .to_string();
                let to = conn["to"].as_str()
                    .ok_or(anyhow!("Connection missing to"))?
                    .to_string();
                connections.push((from, to));
            }
        }

        Ok(Self { nodes, connections })
    }

    pub async fn execute_once(&mut self) -> Result<()> {
        // Simple linear execution for now (no parallelism)
        // Start with empty frame
        let mut current_frame = DataFrame::new(0, 0);

        // Build execution order (simple topological sort for linear pipeline)
        let mut executed = std::collections::HashSet::new();
        let mut execution_order = Vec::new();

        // Find source node (no incoming connections)
        for id in self.nodes.keys() {
            let has_incoming = self.connections.iter()
                .any(|(_, to)| to == id);
            if !has_incoming {
                execution_order.push(id.clone());
                executed.insert(id.clone());
                break;
            }
        }

        // Follow connections to build order
        while execution_order.len() < self.nodes.len() {
            let last = execution_order.last().unwrap();
            if let Some((_, next)) = self.connections.iter()
                .find(|(from, _)| from == last) {
                if !executed.contains(next) {
                    execution_order.push(next.clone());
                    executed.insert(next.clone());
                }
            } else {
                break;
            }
        }

        // Execute in order
        for node_id in execution_order {
            if let Some(node) = self.nodes.get(&node_id) {
                current_frame = node.process(current_frame).await?;
            }
        }

        Ok(())
    }
}
