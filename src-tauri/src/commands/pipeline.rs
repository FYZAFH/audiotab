use crate::state::{AppState, PipelineHandle};
use audiotab::engine::PipelineState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

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
    // For now, just create a placeholder pipeline ID
    let pipeline_id = format!("pipeline_{}", uuid::Uuid::new_v4());

    // TODO: Parse graph and create actual pipeline in Task F
    println!("Deploying graph with {} nodes, {} edges",
             graph.nodes.len(), graph.edges.len());

    // Emit status event
    let _ = app.emit("pipeline-status", PipelineStatusEvent {
        id: pipeline_id.clone(),
        state: "Deploying".to_string(),
        error: None,
    });

    // Simulate deployment delay
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let _ = app.emit("pipeline-status", PipelineStatusEvent {
        id: pipeline_id.clone(),
        state: "Running".to_string(),
        error: None,
    });

    Ok(pipeline_id)
}

#[tauri::command]
pub fn get_all_pipeline_states(state: State<AppState>) -> Vec<PipelineStatus> {
    let pipelines = state.pipelines.lock().unwrap();
    pipelines
        .values()
        .map(|handle| PipelineStatus {
            id: handle.id.clone(),
            state: format!("{:?}", handle.state),
            error: None,
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
