use anyhow::{anyhow, Result};
use serde_json::{json, Value};

/// Translates frontend graph format to backend AsyncPipeline format
///
/// Frontend format:
/// {
///   "nodes": [{"id": "...", "type": "...", "position": {...}, "parameters": {...}}],
///   "edges": [{"id": "...", "source": "...", "target": "...", ...}]
/// }
///
/// Backend format:
/// {
///   "nodes": [{"id": "...", "type": "...", "config": {...}}],
///   "connections": [{"from": "...", "to": "..."}],
///   "pipeline_config": {"channel_capacity": 100, "priority": "Normal"}
/// }
pub fn translate_graph(frontend_graph: Value) -> Result<Value> {
    let nodes_array = frontend_graph["nodes"]
        .as_array()
        .ok_or_else(|| anyhow!("Missing or invalid 'nodes' array"))?;

    let edges_array = frontend_graph["edges"]
        .as_array()
        .ok_or_else(|| anyhow!("Missing or invalid 'edges' array"))?;

    // Transform nodes
    let backend_nodes: Vec<Value> = nodes_array
        .iter()
        .map(|node| {
            json!({
                "id": node["id"],
                "type": map_node_type(node["type"].as_str().unwrap_or("")),
                "config": node["parameters"]
            })
        })
        .collect();

    // Transform edges to connections
    let connections: Vec<Value> = edges_array
        .iter()
        .map(|edge| {
            json!({
                "from": edge["source"],
                "to": edge["target"]
            })
        })
        .collect();

    Ok(json!({
        "nodes": backend_nodes,
        "connections": connections,
        "pipeline_config": {
            "channel_capacity": 100,
            "priority": "Normal"
        }
    }))
}

/// Maps frontend node type names to backend node type names
fn map_node_type(frontend_type: &str) -> &str {
    match frontend_type {
        "SineGenerator" => "AudioSourceNode",
        "Gain" => "GainNode",
        "Print" => "DebugSinkNode",
        "FFT" => "FFTNode",
        "Filter" => "FilterNode",
        _ => frontend_type, // Pass through if unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_translate_simple_graph() {
        // Frontend format
        let frontend_graph = json!({
            "nodes": [
                {
                    "id": "sine-1",
                    "type": "SineGenerator",
                    "position": {"x": 100, "y": 200},
                    "parameters": {"frequency": 440, "amplitude": 1.0}
                },
                {
                    "id": "gain-2",
                    "type": "Gain",
                    "position": {"x": 300, "y": 200},
                    "parameters": {"gain": 0.5}
                }
            ],
            "edges": [
                {
                    "id": "e1",
                    "source": "sine-1",
                    "target": "gain-2",
                    "sourceHandle": null,
                    "targetHandle": null
                }
            ]
        });

        let result = translate_graph(frontend_graph).unwrap();

        // Backend format should have:
        // - nodes array with id, type, config
        // - connections array with from, to
        // - pipeline_config with defaults
        assert!(result["nodes"].is_array());
        assert_eq!(result["nodes"].as_array().unwrap().len(), 2);

        assert!(result["connections"].is_array());
        assert_eq!(result["connections"].as_array().unwrap().len(), 1);

        let conn = &result["connections"][0];
        assert_eq!(conn["from"], "sine-1");
        assert_eq!(conn["to"], "gain-2");
    }

    #[test]
    fn test_translate_empty_graph() {
        let frontend_graph = json!({
            "nodes": [],
            "edges": []
        });

        let result = translate_graph(frontend_graph).unwrap();

        assert_eq!(result["nodes"].as_array().unwrap().len(), 0);
        assert_eq!(result["connections"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_translate_missing_nodes() {
        let frontend_graph = json!({
            "edges": []
        });

        let result = translate_graph(frontend_graph);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing or invalid 'nodes'"));
    }
}
