pub mod commands;
pub mod config_store;
pub mod models;
pub mod ssh_command_parser;
pub mod tunnel_engine;

use commands::AppState;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri_plugin_autostart::MacosLauncher;

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin so a second launch exits before other plugins run
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
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
            // SSH command import
            commands::parse_ssh_command,
            // Autostart
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
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
                mgr.set_app_handle(handle.clone());
            }

            // ─── System Tray ─────────────────────────────────
            let show_item = MenuItemBuilder::with_id("show", "打开主界面").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .tooltip("TunnelHub - SSH 隧道管理")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => show_main_window(app),
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Intercept close → hide to tray instead of quitting
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running TunnelHub");
}
