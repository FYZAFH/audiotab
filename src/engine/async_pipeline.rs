use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use crate::core::{ProcessingNode, DataFrame};
use crate::nodes::{SineGenerator, Gain, Print};

pub struct AsyncPipeline {
    nodes: HashMap<String, Box<dyn ProcessingNode>>,
    connections: Vec<(String, String)>,
    channels: HashMap<String, mpsc::Sender<DataFrame>>,
    handles: Vec<JoinHandle<Result<()>>>,
    source_node_id: Option<String>,
    channel_capacity: usize,
}

impl AsyncPipeline {
    pub async fn from_json(config: Value) -> Result<Self> {
        // Parse channel capacity from config
        let channel_capacity = config["pipeline_config"]["channel_capacity"]
            .as_u64()
            .unwrap_or(100) as usize;

        let mut nodes: HashMap<String, Box<dyn ProcessingNode>> = HashMap::new();
        let mut connections = Vec::new();

        // Parse nodes
        if let Some(nodes_array) = config["nodes"].as_array() {
            for node_config in nodes_array {
                let id = node_config["id"]
                    .as_str()
                    .ok_or(anyhow!("Node missing id"))?
                    .to_string();
                let node_type = node_config["type"].as_str().ok_or(anyhow!("Node missing type"))?;
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
                let from = conn["from"]
                    .as_str()
                    .ok_or(anyhow!("Connection missing from"))?
                    .to_string();
                let to = conn["to"]
                    .as_str()
                    .ok_or(anyhow!("Connection missing to"))?
                    .to_string();
                connections.push((from, to));
            }
        }

        // Find source node (no incoming connections)
        let source_node_id = nodes.keys().find(|id| {
            !connections.iter().any(|(_, to)| to == *id)
        }).map(|s| s.clone());

        Ok(Self {
            nodes,
            connections,
            channels: HashMap::new(),
            handles: Vec::new(),
            source_node_id,
            channel_capacity,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let channel_capacity = self.channel_capacity;
        let mut node_channels: HashMap<String, (mpsc::Sender<DataFrame>, mpsc::Receiver<DataFrame>)> = HashMap::new();

        // Create channels for each node
        for node_id in self.nodes.keys() {
            let (tx, rx) = mpsc::channel(channel_capacity);
            node_channels.insert(node_id.clone(), (tx, rx));
        }

        // Save source node channel before spawning
        if let Some(source_id) = &self.source_node_id {
            if let Some((tx, _)) = node_channels.get(source_id) {
                self.channels.insert(source_id.clone(), tx.clone());
            }
        }

        // Build output channel map (which nodes send to which channels)
        let mut output_channels: HashMap<String, Vec<mpsc::Sender<DataFrame>>> = HashMap::new();
        for (from, to) in &self.connections {
            output_channels
                .entry(from.clone())
                .or_insert_with(Vec::new)
                .push(node_channels.get(to).unwrap().0.clone());
        }

        // Spawn task for each node
        for (node_id, node) in self.nodes.drain() {
            let (_tx, rx) = node_channels.remove(&node_id).unwrap();
            let outputs = output_channels.remove(&node_id).unwrap_or_default();

            let handle = tokio::spawn(async move {
                let (fanout_tx, mut fanout_rx) = mpsc::channel(channel_capacity);

                // Spawn node processing
                let node_task = tokio::spawn(async move {
                    node.run(rx, fanout_tx).await
                });

                // Spawn fanout (send to multiple outputs)
                let fanout_task = tokio::spawn(async move {
                    while let Some(frame) = fanout_rx.recv().await {
                        for output in &outputs {
                            let _ = output.send(frame.clone()).await;
                        }
                    }
                });

                node_task.await??;
                fanout_task.await?;
                Ok(())
            });

            self.handles.push(handle);
        }

        Ok(())
    }

    pub async fn trigger(&self, frame: DataFrame) -> Result<()> {
        if let Some(source_id) = &self.source_node_id {
            if let Some(tx) = self.channels.get(source_id) {
                tx.send(frame).await.map_err(|_| anyhow!("Failed to send trigger frame"))?;
            }
        }
        Ok(())
    }

    pub async fn stop(self) -> Result<()> {
        // Drop channels to signal nodes to shut down
        drop(self.channels);

        // Wait for all node tasks to complete
        for handle in self.handles {
            handle.await??;
        }

        Ok(())
    }
}
