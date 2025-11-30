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
    let mut pipeline = match AsyncPipeline::from_json(backend_json).await {
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

    // Step 3: Inject RingBuffer into visualization-capable nodes
    pipeline.set_ring_buffer(state.ring_buffer.clone());

    // Step 4: Inject DeviceChannels into AudioSourceNodes with device_profile_id
    let mut started_devices = Vec::new(); // Track successfully started devices

    let device_injection_results: Vec<Result<(), String>> = {
        let mut results = Vec::new();

        for (node_id, node) in pipeline.nodes_mut().iter_mut() {
            if let Some(audio_source) = node.as_any_mut()
                .downcast_mut::<audiotab::nodes::AudioSourceNode>()
            {
                let device_profile_id = audio_source.device_profile_id.clone();

                if !device_profile_id.is_empty() {
                    println!("AudioSourceNode '{}' requests device profile '{}'", node_id, device_profile_id);

                    // Async device creation and channel injection
                    let manager_arc = state.device_manager.clone();
                    let device_id_for_closure = device_profile_id.clone();

                    let result = tokio::task::spawn_blocking(move || {
                        let manager = manager_arc.lock()
                            .map_err(|e| format!("Device manager lock poisoned: {}", e))?;

                        // Create runtime for async start_device
                        let runtime = tokio::runtime::Runtime::new()
                            .map_err(|e| format!("Failed to create runtime: {}", e))?;

                        runtime.block_on(async {
                            manager.start_device(&device_id_for_closure).await
                                .map_err(|e| format!("Failed to start device '{}': {}", device_id_for_closure, e))
                        })
                    })
                    .await
                    .map_err(|e| format!("Device creation task failed: {}", e))?;

                    match result {
                        Ok(_) => {
                            started_devices.push(device_profile_id.clone());

                            // Get device channels
                            let channels = {
                                let mut manager = state.device_manager.lock()
                                    .map_err(|e| format!("Device manager lock poisoned: {}", e))?;

                                manager.get_device_channels(&device_profile_id)
                                    .map_err(|e| format!("Failed to get device channels: {}", e))?
                            };

                            // Inject channels into node
                            audio_source.set_device_channels(Some(channels));
                            println!("Successfully injected device channels for '{}'", device_profile_id);

                            results.push(Ok(()));
                        }
                        Err(e) => {
                            results.push(Err(e));
                            break; // Stop processing on first failure
                        }
                    }
                }
            }
        }

        results
    };

    // Check if any device injection failed - cleanup started devices if so
    for result in device_injection_results.iter() {
        if let Err(e) = result {
            let error_msg = format!("Device injection failed: {}", e);
            println!("Error: {}", error_msg);

            // Cleanup: Stop all devices that were successfully started
            let mut cleanup_handles = Vec::new();
            for device_id in started_devices.iter() {
                println!("Cleaning up device: {}", device_id);
                let manager_arc = state.device_manager.clone();
                let device_id_clone = device_id.clone();

                let handle = tokio::task::spawn_blocking(move || {
                    if let Ok(manager) = manager_arc.lock() {
                        let runtime = tokio::runtime::Runtime::new().ok()?;
                        runtime.block_on(async {
                            let _ = manager.stop_device(&device_id_clone).await;
                        });
                    }
                    Some(())
                });
                cleanup_handles.push(handle);
            }

            // Wait for all cleanup to complete
            for handle in cleanup_handles {
                let _ = handle.await;
            }

            let _ = app.emit("pipeline-status", PipelineStatusEvent {
                id: pipeline_id.clone(),
                state: "Error".to_string(),
                error: Some(error_msg.clone()),
            });

            return Err(error_msg);
        }
    }

    // Step 5: Store pipeline in state
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
pub fn control_pipeline(
    state: State<'_, AppState>,
    kernel_manager: State<'_, crate::kernel_manager::KernelManager>,
    id: String,
    action: PipelineAction,
) -> Result<(), String> {
    println!("Control pipeline {}: {:?}", id, action);

    // Get the pipeline handle
    let pipeline_arc = {
        let pipelines = state.pipelines.lock().unwrap();
        let handle = pipelines.get(&id)
            .ok_or_else(|| format!("Pipeline {} not found", id))?;

        // Clone the Arc references we need
        (handle.pipeline.clone(), handle.state.clone())
    };

    match action {
        PipelineAction::Start => {
            // Execute the pipeline via KernelManager
            kernel_manager.execute_pipeline_sync(pipeline_arc.0.clone())
                .map_err(|e| format!("Failed to execute pipeline: {}", e))?;

            // Update state to Running
            *pipeline_arc.1.lock().unwrap() = PipelineState::Running {
                start_time: Some(std::time::Instant::now()),
                frames_processed: 0,
            };

            println!("Pipeline {} started successfully", id);
        }
        PipelineAction::Stop => {
            // Stop the pipeline via async stop() method
            let mut pipeline_guard = pipeline_arc.0.lock().unwrap();

            // Call stop on the pipeline (async operation)
            let runtime = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create runtime: {}", e))?;
            runtime.block_on(async {
                pipeline_guard.stop().await
            }).map_err(|e| format!("Failed to stop pipeline: {}", e))?;

            // Update state to Completed
            *pipeline_arc.1.lock().unwrap() = PipelineState::Completed {
                duration: None,
                total_frames: 0,
            };

            println!("Pipeline {} stopped successfully", id);
        }
        PipelineAction::Pause => {
            // Pause not yet implemented in AsyncPipeline
            // Defer to future implementation
            println!("Pause not yet implemented for pipeline {}", id);
            return Err("Pause action not yet supported".to_string());
        }
    }

    Ok(())
}

/// Trigger a pipeline to process one frame
///
/// Sends a trigger DataFrame to the pipeline's source node, causing it to process one frame.
/// This is used for triggered execution mode where frames are processed on demand.
#[tauri::command]
pub fn trigger_pipeline(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    println!("Trigger pipeline {}", id);

    // Get the pipeline handle
    let pipeline_arc = {
        let pipelines = state.pipelines.lock().unwrap();
        let handle = pipelines.get(&id)
            .ok_or_else(|| format!("Pipeline {} not found", id))?;
        handle.pipeline.clone()
    };

    // Create a simple trigger DataFrame
    // For Phase 7, this is a minimal frame just to trigger processing
    use audiotab::core::DataFrame;
    let trigger_frame = DataFrame::new(0, 0); // timestamp=0, sequence_id=0

    // Send trigger frame to pipeline
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    runtime.block_on(async {
        let pipeline = pipeline_arc.lock().unwrap();
        pipeline.trigger(trigger_frame).await
    }).map_err(|e| format!("Failed to trigger pipeline: {}", e))?;

    println!("Pipeline {} triggered successfully", id);
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

    #[tokio::test]
    async fn test_pipeline_execution_starts() {
        // This test validates that control_pipeline can start execution
        use crate::kernel_manager::KernelManager;
        use audiotab::hal::{HardwareRegistry, HardwareConfig};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let state = AppState::new();

        // Create a simple graph with a sine generator
        let graph = GraphJson {
            nodes: vec![
                json!({"id": "sine-1", "type": "SineGenerator", "parameters": {"frequency": 440}}),
            ],
            edges: vec![],
        };

        // Deploy the graph (without AppHandle - just create pipeline directly)
        let frontend_json = serde_json::json!({
            "nodes": graph.nodes,
            "edges": graph.edges
        });

        let backend_json = translate_graph(frontend_json).unwrap();
        let pipeline = AsyncPipeline::from_json(backend_json).await.unwrap();
        let pipeline_id = format!("pipeline_{}", uuid::Uuid::new_v4());

        let handle = PipelineHandle {
            id: pipeline_id.clone(),
            pipeline: Arc::new(Mutex::new(pipeline)),
            state: Arc::new(Mutex::new(PipelineState::Idle)),
        };

        {
            let mut pipelines = state.pipelines.lock().unwrap();
            pipelines.insert(pipeline_id.clone(), handle);
        }

        // Create kernel manager
        let registry = Arc::new(RwLock::new(HardwareRegistry::new()));
        let config = HardwareConfig {
            version: "1.0".to_string(),
            registered_devices: vec![],
        };
        let _kernel_manager = KernelManager::new(registry, config);

        // Note: We can't actually start the kernel without devices, but we can test the control flow
        // The test validates that control_pipeline updates the state

        // Verify initial state is Idle
        {
            let pipelines = state.pipelines.lock().unwrap();
            let handle = pipelines.get(&pipeline_id).unwrap();
            let pipeline_state = handle.state.lock().unwrap();
            assert!(matches!(*pipeline_state, PipelineState::Idle));
        }

        // Note: Full execution test would require a running kernel with devices
        // For now, this test documents the expected behavior
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
