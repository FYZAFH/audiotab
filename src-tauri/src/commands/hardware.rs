use tauri::State;
use crate::state::AppState;
use audiotab::hal::{DeviceInfo, DeviceProfile};

#[tauri::command]
pub async fn discover_devices(
    state: State<'_, AppState>,
) -> Result<Vec<DeviceInfo>, String> {
    // We need to avoid holding the lock across the await point
    // The manager.discover_all() needs a &self reference, so we need to restructure

    // Get the Arc reference
    let manager_arc = state.device_manager.clone();

    // Lock, call discover_all (which is async), and the lock will be held
    // This is a problem for Send. We need tokio::spawn with a block_in_place or
    // restructure to not hold the lock.

    // For now, use tokio::task::spawn_blocking workaround
    tokio::task::spawn_blocking(move || {
        let manager = manager_arc.lock().unwrap();
        // This is still async, so we need to block on it
        tokio::runtime::Handle::current().block_on(manager.discover_all())
    })
    .await
    .map_err(|e| format!("Task join failed: {}", e))?
    .map_err(|e| format!("Device discovery failed: {}", e))
}

#[tauri::command]
pub fn list_device_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<DeviceProfile>, String> {
    let manager = state.device_manager.lock().unwrap();
    Ok(manager.list_profiles().into_iter().cloned().collect())
}

#[tauri::command]
pub fn get_device_profile(
    state: State<'_, AppState>,
    id: String,
) -> Result<DeviceProfile, String> {
    let manager = state.device_manager.lock().unwrap();

    manager.get_profile(&id)
        .cloned()
        .ok_or_else(|| format!("Profile {} not found", id))
}

#[tauri::command]
pub fn add_device_profile(
    state: State<'_, AppState>,
    profile: DeviceProfile,
) -> Result<(), String> {
    let mut manager = state.device_manager.lock().unwrap();

    manager.add_profile(profile)
        .map_err(|e| format!("Failed to add profile: {}", e))
}

#[tauri::command]
pub fn update_device_profile(
    state: State<'_, AppState>,
    profile: DeviceProfile,
) -> Result<(), String> {
    let mut manager = state.device_manager.lock().unwrap();

    manager.update_profile(profile)
        .map_err(|e| format!("Failed to update profile: {}", e))
}

#[tauri::command]
pub fn delete_device_profile(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut manager = state.device_manager.lock().unwrap();

    manager.delete_profile(&id)
        .map_err(|e| format!("Failed to delete profile: {}", e))
}
