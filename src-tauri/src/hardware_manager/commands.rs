use tauri::State;
use audiotab::hal::{DeviceInfo, DeviceConfig};
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
