pub mod commands;
pub mod config_store;
pub mod models;
pub mod tunnel_engine;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Tunnel CRUD
            commands::list_tunnels,
            commands::create_tunnel,
            commands::update_tunnel,
            commands::delete_tunnel,
            commands::duplicate_tunnel,
            // Tunnel Control
            commands::start_tunnel,
            commands::stop_tunnel,
            commands::get_tunnel_status,
            commands::get_tunnel_state,
            commands::get_all_statuses,
            commands::get_tunnel_logs,
            // Groups
            commands::list_groups,
            commands::create_group,
            commands::update_group,
            commands::delete_group,
            commands::assign_tunnel_to_group,
            commands::start_group,
            commands::stop_group,
            // Import/Export
            commands::export_config_to_file,
            commands::import_config_from_file,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Load config synchronously on startup
            let config = config_store::load_config(&handle);

            // Use blocking lock since setup runs before the async runtime is fully started
            let state: tauri::State<'_, AppState> = app.state::<AppState>();
            {
                let mut cfg = state.config.blocking_lock();
                *cfg = config;
            }
            {
                let mut mgr = state.manager.blocking_lock();
                mgr.set_app_handle(handle);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running TunnelHub");
}
