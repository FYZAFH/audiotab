use crate::state::{AppState, NodeMetadata};
use tauri::State;

#[tauri::command]
pub fn get_node_registry(state: State<AppState>) -> Vec<NodeMetadata> {
    state.registry.list_nodes()
}
