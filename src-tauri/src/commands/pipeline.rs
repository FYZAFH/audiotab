use crate::state::{AppState, PipelineHandle};
use crate::graph::translate_graph;
use audiotab::engine::{AsyncPipeline, PipelineState};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use std::sync::{Arc, Mutex};

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

#[derive(Debug, Serialize, Clone)]
pub struct PipelineStatusEvent {
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
    app: AppHandle,
    state: State<'_, AppState>,
    graph: GraphJson,
) -> Result<String, String> {
    // Generate unique pipeline ID
    let pipeline_id = format!("pipeline_{}", uuid::Uuid::new_v4());

    println!("Deploying graph with {} nodes, {} edges",
             graph.nodes.len(), graph.edges.len());

    // Emit deploying status
    let _ = app.emit("pipeline-status", PipelineStatusEvent {
        id: pipeline_id.clone(),
        state: "Deploying".to_string(),
        error: None,
    });

    // Step 1: Translate frontend graph to backend format
    let frontend_json = serde_json::json!({
        "nodes": graph.nodes,
        "edges": graph.edges
    });

    let backend_json = match translate_graph(frontend_json) {
        Ok(json) => json,
        Err(e) => {
            let error_msg = format!("Graph translation failed: {}", e);
            println!("Translation error: {}", error_msg);

            let _ = app.emit("pipeline-status", PipelineStatusEvent {
                id: pipeline_id.clone(),
                state: "Error".to_string(),
                error: Some(error_msg.clone()),
            });

            return Err(error_msg);
        }
    };

    println!("Translated graph: {}", serde_json::to_string_pretty(&backend_json).unwrap());

    // Step 2: Create AsyncPipeline from translated graph
    let pipeline = match AsyncPipeline::from_json(backend_json).await {
        Ok(p) => p,
        Err(e) => {
            let error_msg = format!("Pipeline creation failed: {}", e);
            println!("Pipeline creation error: {}", error_msg);

            // Emit error event
            let _ = app.emit("pipeline-status", PipelineStatusEvent {
                id: pipeline_id.clone(),
                state: "Error".to_string(),
                error: Some(error_msg.clone()),
            });

            return Err(error_msg);
        }
    };

    // Step 3: Store pipeline in state
    let handle = PipelineHandle {
        id: pipeline_id.clone(),
        pipeline: Arc::new(Mutex::new(pipeline)),
        state: Arc::new(Mutex::new(PipelineState::Idle)),
    };

    {
        let mut pipelines = state.pipelines.lock().unwrap();
        pipelines.insert(pipeline_id.clone(), handle);
    }

    // Emit success status
    let _ = app.emit("pipeline-status", PipelineStatusEvent {
        id: pipeline_id.clone(),
        state: "Idle".to_string(),
        error: None,
    });

    println!("Pipeline {} created successfully", pipeline_id);

    Ok(pipeline_id)
}

#[tauri::command]
pub fn get_all_pipeline_states(state: State<AppState>) -> Vec<PipelineStatus> {
    let pipelines = state.pipelines.lock().unwrap();
    pipelines
        .values()
        .map(|handle| {
            let state = handle.state.lock().unwrap();
            PipelineStatus {
                id: handle.id.clone(),
                state: format!("{:?}", *state),
                error: None,
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_deploy_graph_creates_pipeline() {
        // Test the translation and pipeline storage logic without AppHandle
        let state = AppState::new();

        let graph = GraphJson {
            nodes: vec![
                json!({"id": "sine-1", "type": "SineGenerator", "parameters": {"frequency": 440}}),
                json!({"id": "print-2", "type": "Print", "parameters": {}})
            ],
            edges: vec![
                json!({"source": "sine-1", "target": "print-2"})
            ],
        };

        // Test graph translation
        let frontend_json = serde_json::json!({
            "nodes": graph.nodes,
            "edges": graph.edges
        });

        let backend_json = translate_graph(frontend_json).unwrap();
        assert!(backend_json["nodes"].is_array());
        assert_eq!(backend_json["nodes"].as_array().unwrap().len(), 2);

        // Test pipeline creation
        let pipeline = AsyncPipeline::from_json(backend_json).await;
        assert!(pipeline.is_ok(), "Pipeline creation should succeed");

        // Test storage
        let pipeline_id = format!("pipeline_{}", uuid::Uuid::new_v4());
        let handle = PipelineHandle {
            id: pipeline_id.clone(),
            pipeline: Arc::new(Mutex::new(pipeline.unwrap())),
            state: Arc::new(Mutex::new(PipelineState::Idle)),
        };

        {
            let mut pipelines = state.pipelines.lock().unwrap();
            pipelines.insert(pipeline_id.clone(), handle);
        }

        // Verify pipeline was stored
        let pipelines = state.pipelines.lock().unwrap();
        assert!(pipelines.contains_key(&pipeline_id));
        assert_eq!(pipelines.len(), 1);
    }

    #[tokio::test]
    async fn test_deploy_invalid_graph_returns_error() {
        // Test error handling for invalid graph
        let graph = GraphJson {
            nodes: vec![
                json!({"id": "invalid-1", "type": "NonExistentNode", "parameters": {}})
            ],
            edges: vec![],
        };

        // Test translation
        let frontend_json = serde_json::json!({
            "nodes": graph.nodes,
            "edges": graph.edges
        });

        let backend_json = translate_graph(frontend_json).unwrap();

        // Pipeline creation should fail for unknown node type
        let result = AsyncPipeline::from_json(backend_json).await;
        assert!(result.is_err(), "Should fail for unknown node type");
    }
}

#[cfg(test)]
mod manual_tests {
    use super::*;
    use serde_json::json;

    /// Manual test for deploying a sine-to-print graph
    /// Run with: cargo test manual_test_deploy_sine_to_print -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn manual_test_deploy_sine_to_print() {
        println!("\n=== Manual Test: Deploy Sine to Print ===\n");

        let graph = GraphJson {
            nodes: vec![
                json!({
                    "id": "sine-source",
                    "type": "SineGenerator",
                    "parameters": {"frequency": 440}
                }),
                json!({
                    "id": "print-sink",
                    "type": "Print",
                    "parameters": {}
                })
            ],
            edges: vec![
                json!({
                    "source": "sine-source",
                    "target": "print-sink"
                })
            ],
        };

        println!("Graph: {} nodes, {} edges", graph.nodes.len(), graph.edges.len());

        // Test translation
        let frontend_json = serde_json::json!({
            "nodes": graph.nodes,
            "edges": graph.edges
        });

        println!("\nTranslating graph...");
        let backend_json = translate_graph(frontend_json).unwrap();
        println!("Translation successful!");
        println!("Backend JSON: {}", serde_json::to_string_pretty(&backend_json).unwrap());

        // Test pipeline creation
        println!("\nCreating pipeline...");
        let pipeline = AsyncPipeline::from_json(backend_json).await;
        assert!(pipeline.is_ok(), "Pipeline creation should succeed");
        println!("Pipeline created successfully!");

        // Note: This test cannot call deploy_graph directly because it requires a Tauri AppHandle
        // In a real application, the deploy_graph command will handle all these steps
        println!("\n=== Test Complete ===\n");
    }
}
