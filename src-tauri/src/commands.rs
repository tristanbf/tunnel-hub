use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use crate::config_store;
use crate::models::*;
use crate::ssh_command_parser;
use crate::tunnel_engine::TunnelManager;

/// Shared application state accessible from Tauri commands.
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub manager: Mutex<TunnelManager>,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            config: Mutex::new(AppConfig::default()),
            manager: Mutex::new(TunnelManager::new()),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_config(config: AppConfig) -> Self {
        AppState {
            config: Mutex::new(config),
            manager: Mutex::new(TunnelManager::new()),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }
}

// ─── Tunnel CRUD ───────────────────────────────────────────────

#[tauri::command]
pub async fn list_tunnels(state: State<'_, AppState>) -> Result<Vec<TunnelConfig>, String> {
    let config = state.config.lock().await;
    Ok(config.tunnels.clone())
}

#[tauri::command]
pub async fn create_tunnel(
    config: TunnelConfig,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<TunnelConfig, String> {
    let mut app_config = state.config.lock().await;
    let mut tunnel = config;
    if tunnel.id.is_empty() {
        tunnel.id = uuid::Uuid::new_v4().to_string();
    }
    app_config.tunnels.push(tunnel.clone());
    config_store::save_config(&app, &app_config)?;
    Ok(tunnel)
}

#[tauri::command]
pub async fn update_tunnel(
    config: TunnelConfig,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut app_config = state.config.lock().await;
    if let Some(existing) = app_config.tunnels.iter_mut().find(|t| t.id == config.id) {
        *existing = config;
        config_store::save_config(&app, &app_config)?;
        Ok(())
    } else {
        Err("Tunnel not found".to_string())
    }
}

#[tauri::command]
pub async fn delete_tunnel(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Stop if running
    {
        let mut manager = state.manager.lock().await;
        let _ = manager.stop_tunnel(&id).await;
    }
    let mut app_config = state.config.lock().await;
    app_config.tunnels.retain(|t| t.id != id);
    // Also remove from any group references
    config_store::save_config(&app, &app_config)?;
    Ok(())
}

#[tauri::command]
pub async fn duplicate_tunnel(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<TunnelConfig, String> {
    let mut app_config = state.config.lock().await;
    let original = app_config
        .tunnels
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or("Tunnel not found")?;

    let mut copy = original;
    copy.id = uuid::Uuid::new_v4().to_string();
    copy.name = format!("{} (copy)", copy.name);
    copy.local_port = 0; // User must pick a new port
    app_config.tunnels.push(copy.clone());
    config_store::save_config(&app, &app_config)?;
    Ok(copy)
}

/// Reorder tunnels. `ids` is the desired order of a subset (or all) of tunnels.
/// Those ids keep their existing slots; only their relative order changes.
#[tauri::command]
pub async fn reorder_tunnels(
    ids: Vec<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }

    let mut app_config = state.config.lock().await;
    let mut seen = HashSet::new();
    for id in &ids {
        if !seen.insert(id.clone()) {
            return Err("Duplicate tunnel id in reorder list".to_string());
        }
        if !app_config.tunnels.iter().any(|t| t.id == *id) {
            return Err(format!("Unknown tunnel id: {}", id));
        }
    }

    let slots: Vec<usize> = app_config
        .tunnels
        .iter()
        .enumerate()
        .filter(|(_, t)| seen.contains(&t.id))
        .map(|(i, _)| i)
        .collect();

    let by_id: HashMap<String, TunnelConfig> = app_config
        .tunnels
        .iter()
        .cloned()
        .map(|t| (t.id.clone(), t))
        .collect();

    for (slot, id) in slots.into_iter().zip(ids.iter()) {
        if let Some(tunnel) = by_id.get(id) {
            app_config.tunnels[slot] = tunnel.clone();
        }
    }

    config_store::save_config(&app, &app_config)?;
    Ok(())
}

// ─── Tunnel Control ────────────────────────────────────────────

#[tauri::command]
pub async fn start_tunnel(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config = {
        let app_config = state.config.lock().await;
        app_config
            .tunnels
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or("Tunnel not found")?
    };

    let mut manager = state.manager.lock().await;
    manager.start_tunnel(&config).await
}

#[tauri::command]
pub async fn stop_tunnel(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.stop_tunnel(&id).await
}

#[tauri::command]
pub async fn get_tunnel_status(
    id: String,
    state: State<'_, AppState>,
) -> Result<TunnelStatus, String> {
    let manager = state.manager.lock().await;
    Ok(manager.get_status(&id))
}

#[tauri::command]
pub async fn get_tunnel_state(
    id: String,
    state: State<'_, AppState>,
) -> Result<TunnelState, String> {
    let manager = state.manager.lock().await;
    Ok(manager.get_state(&id))
}

#[tauri::command]
pub async fn get_all_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<(String, TunnelStatus)>, String> {
    let app_config = state.config.lock().await;
    let manager = state.manager.lock().await;
    let statuses = app_config
        .tunnels
        .iter()
        .map(|t| (t.id.clone(), manager.get_status(&t.id)))
        .collect();
    Ok(statuses)
}

#[tauri::command]
pub async fn get_tunnel_logs(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<LogEntry>, String> {
    let manager = state.manager.lock().await;
    let st = manager.get_state(&id);
    Ok(st.logs)
}

// ─── Group CRUD ────────────────────────────────────────────────

#[tauri::command]
pub async fn list_groups(state: State<'_, AppState>) -> Result<Vec<Group>, String> {
    let config = state.config.lock().await;
    Ok(config.groups.clone())
}

#[tauri::command]
pub async fn create_group(
    name: String,
    parent_id: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Group, String> {
    let mut app_config = state.config.lock().await;
    let group = Group::new(name, parent_id);
    app_config.groups.push(group.clone());
    config_store::save_config(&app, &app_config)?;
    Ok(group)
}

#[tauri::command]
pub async fn update_group(
    group: Group,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut app_config = state.config.lock().await;
    if let Some(existing) = app_config.groups.iter_mut().find(|g| g.id == group.id) {
        *existing = group;
        config_store::save_config(&app, &app_config)?;
        Ok(())
    } else {
        Err("Group not found".to_string())
    }
}

#[tauri::command]
pub async fn delete_group(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut app_config = state.config.lock().await;

    // Collect all descendant group IDs (recursive)
    fn collect_children(groups: &[Group], parent_id: &str) -> Vec<String> {
        let mut result = vec![];
        for g in groups {
            if g.parent_id.as_deref() == Some(parent_id) {
                result.push(g.id.clone());
                result.extend(collect_children(groups, &g.id));
            }
        }
        result
    }

    let to_delete: Vec<String> = {
        let mut ids = vec![id.clone()];
        ids.extend(collect_children(&app_config.groups, &id));
        ids
    };

    // Unassign tunnels from deleted groups
    for tunnel in &mut app_config.tunnels {
        if let Some(ref gid) = tunnel.group_id {
            if to_delete.contains(gid) {
                tunnel.group_id = None;
            }
        }
    }

    app_config.groups.retain(|g| !to_delete.contains(&g.id));
    config_store::save_config(&app, &app_config)?;
    Ok(())
}

/// Reorder groups and optionally re-parent them.
/// `items` must list every existing group exactly once; array order is the
/// persisted sibling order (same parent, earlier index = higher in the tree).
#[tauri::command]
pub async fn reorder_groups(
    items: Vec<GroupReorderItem>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }

    let mut app_config = state.config.lock().await;
    if items.len() != app_config.groups.len() {
        return Err("Group reorder list must include every group".to_string());
    }

    let mut seen = HashSet::new();
    let existing: HashSet<String> = app_config.groups.iter().map(|g| g.id.clone()).collect();
    for item in &items {
        if !seen.insert(item.id.clone()) {
            return Err("Duplicate group id in reorder list".to_string());
        }
        if !existing.contains(&item.id) {
            return Err(format!("Unknown group id: {}", item.id));
        }
        if let Some(ref parent_id) = item.parent_id {
            if parent_id == &item.id {
                return Err("Group cannot be its own parent".to_string());
            }
            if !existing.contains(parent_id) {
                return Err(format!("Unknown parent id: {}", parent_id));
            }
        }
    }

    let parent_of: HashMap<&str, Option<&str>> = items
        .iter()
        .map(|i| (i.id.as_str(), i.parent_id.as_deref()))
        .collect();
    for item in &items {
        let mut walked = HashSet::new();
        let mut current = Some(item.id.as_str());
        while let Some(id) = current {
            if !walked.insert(id) {
                return Err("Group hierarchy contains a cycle".to_string());
            }
            current = parent_of.get(id).copied().flatten();
        }
    }

    let by_id: HashMap<String, Group> = app_config
        .groups
        .iter()
        .cloned()
        .map(|g| (g.id.clone(), g))
        .collect();

    app_config.groups = items
        .into_iter()
        .map(|item| {
            let mut group = by_id.get(&item.id).cloned().expect("id checked above");
            group.parent_id = item.parent_id;
            group
        })
        .collect();

    config_store::save_config(&app, &app_config)?;
    Ok(())
}

#[tauri::command]
pub async fn assign_tunnel_to_group(
    tunnel_id: String,
    group_id: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut app_config = state.config.lock().await;
    if let Some(tunnel) = app_config.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
        tunnel.group_id = group_id;
        config_store::save_config(&app, &app_config)?;
        Ok(())
    } else {
        Err("Tunnel not found".to_string())
    }
}

// ─── Group Batch Operations ────────────────────────────────────

#[tauri::command]
pub async fn start_group(
    group_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let tunnel_ids = {
        let app_config = state.config.lock().await;

        // Collect the group and all descendant group IDs
        fn collect_ids(groups: &[Group], parent_id: &str) -> Vec<String> {
            let mut result = vec![parent_id.to_string()];
            for g in groups {
                if g.parent_id.as_deref() == Some(parent_id) {
                    result.extend(collect_ids(groups, &g.id));
                }
            }
            result
        }

        let group_ids = collect_ids(&app_config.groups, &group_id);
        app_config
            .tunnels
            .iter()
            .filter(|t| t.group_id.as_ref().map_or(false, |g| group_ids.contains(g)))
            .map(|t| t.id.clone())
            .collect::<Vec<_>>()
    };

    let mut errors = vec![];
    for tid in &tunnel_ids {
        let config = {
            let app_config = state.config.lock().await;
            app_config.tunnels.iter().find(|t| t.id == *tid).cloned()
        };
        if let Some(cfg) = config {
            let mut manager = state.manager.lock().await;
            if let Err(e) = manager.start_tunnel(&cfg).await {
                errors.push(format!("{}: {}", cfg.name, e));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[tauri::command]
pub async fn stop_group(
    group_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let tunnel_ids = {
        let app_config = state.config.lock().await;

        fn collect_ids(groups: &[Group], parent_id: &str) -> Vec<String> {
            let mut result = vec![parent_id.to_string()];
            for g in groups {
                if g.parent_id.as_deref() == Some(parent_id) {
                    result.extend(collect_ids(groups, &g.id));
                }
            }
            result
        }

        let group_ids = collect_ids(&app_config.groups, &group_id);
        app_config
            .tunnels
            .iter()
            .filter(|t| t.group_id.as_ref().map_or(false, |g| group_ids.contains(g)))
            .map(|t| t.id.clone())
            .collect::<Vec<_>>()
    };

    let mut manager = state.manager.lock().await;
    for tid in &tunnel_ids {
        let _ = manager.stop_tunnel(tid).await;
    }
    Ok(())
}

// ─── Import / Export ───────────────────────────────────────────

#[tauri::command]
pub async fn export_config_to_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_config = state.config.lock().await;
    config_store::export_config(&path, &app_config)
}

#[tauri::command]
pub async fn import_config_from_file(
    path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<AppConfig, String> {
    let imported = config_store::import_config(&path)?;
    let mut app_config = state.config.lock().await;
    *app_config = imported.clone();
    config_store::save_config(&app, &app_config)?;
    Ok(imported)
}

// ─── SSH Command Parser ────────────────────────────────────────

#[tauri::command]
pub async fn parse_ssh_command(command: String) -> Result<TunnelConfig, String> {
    ssh_command_parser::parse_ssh_command(&command)
}

// ─── Autostart ─────────────────────────────────────────────────

#[tauri::command]
pub async fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("获取开机自启状态失败: {}", e))
}

#[tauri::command]
pub async fn set_autostart_enabled(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| format!("启用开机自启失败: {}", e))
    } else {
        manager.disable().map_err(|e| format!("禁用开机自启失败: {}", e))
    }
}
