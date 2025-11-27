mod state;
mod commands;
mod nodes;
mod graph;
pub mod hardware_manager;
pub mod kernel_manager;

use state::AppState;
use hardware_manager::{
    HardwareManagerState,
    discover_hardware,
    create_hardware_device,
    get_registered_devices,
    register_device,
    update_device,
    remove_device,
};
use kernel_manager::KernelManager;
use audiotab::hal::HardwareConfig;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Import all nodes to trigger inventory registration
  use audiotab::nodes::*;
  let _ = (
      GainNode::default(),
      AudioSourceNode::default(),
      TriggerSourceNode::default(),
      DebugSinkNode::default(),
      FFTNode::default(),
      FilterNode::default(),
  );

  // Create shared HardwareManagerState which includes registry
  let hardware_state = HardwareManagerState::new();

  // Create KernelManager with shared registry from HardwareManagerState
  let kernel_manager = KernelManager::new(
    hardware_state.get_registry_arc(),
    HardwareConfig::default(),
  );

  tauri::Builder::default()
    .manage(AppState::new())
    .manage(hardware_state)
    .manage(kernel_manager)
    .invoke_handler(tauri::generate_handler![
        commands::nodes::get_node_registry,
        commands::pipeline::deploy_graph,
        commands::pipeline::get_all_pipeline_states,
        commands::pipeline::control_pipeline,
        commands::visualization::get_ringbuffer_data,
        commands::kernel::start_kernel,
        commands::kernel::stop_kernel,
        commands::kernel::get_kernel_status,
        discover_hardware,
        create_hardware_device,
        get_registered_devices,
        register_device,
        update_device,
        remove_device,
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

