use tauri::State;
use audiotab::hal::{DeviceInfo, DeviceConfig, RegisteredHardware};
use super::state::HardwareManagerState;

#[tauri::command]
pub async fn discover_hardware(
    state: State<'_, HardwareManagerState>,
) -> Result<Vec<DeviceInfo>, String> {
    state.discover_devices()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_hardware_device(
    state: State<'_, HardwareManagerState>,
    driver_id: String,
    device_id: String,
    config: DeviceConfig,
) -> Result<(), String> {
    state.create_device(&driver_id, &device_id, config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_registered_devices(
    state: State<'_, HardwareManagerState>,
) -> Result<Vec<RegisteredHardware>, String> {
    state.config_manager()
        .load()
        .await
        .map_err(|e| e.to_string())?;

    state.config_manager()
        .get_registered_devices()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn register_device(
    state: State<'_, HardwareManagerState>,
    device: RegisteredHardware,
) -> Result<(), String> {
    state.config_manager()
        .register_device(device)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_device(
    state: State<'_, HardwareManagerState>,
    registration_id: String,
    device: RegisteredHardware,
) -> Result<(), String> {
    state.config_manager()
        .update_device(&registration_id, device)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_device(
    state: State<'_, HardwareManagerState>,
    registration_id: String,
) -> Result<(), String> {
    state.config_manager()
        .remove_device(&registration_id)
        .await
        .map_err(|e| e.to_string())
}
