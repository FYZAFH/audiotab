mod state;
mod commands;
mod nodes;
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

  tauri::Builder::default()
    .manage(AppState::new())
    .manage(HardwareManagerState::new())
    .invoke_handler(tauri::generate_handler![
        commands::nodes::get_node_registry,
        commands::pipeline::deploy_graph,
        commands::pipeline::get_all_pipeline_states,
        commands::pipeline::control_pipeline,
        commands::visualization::get_ringbuffer_data,
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
