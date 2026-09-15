import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  TunnelConfig,
  TunnelStatus,
  TunnelState,
  Group,
  GroupReorderItem,
  AppConfig,
  LogEntry,
  TunnelStatusEvent,
  TunnelLogEvent,
} from '@/types'

// ─── Tunnel CRUD ───────────────────────────────────────────────

export async function listTunnels(): Promise<TunnelConfig[]> {
  return invoke<TunnelConfig[]>('list_tunnels')
}

export async function createTunnel(config: TunnelConfig): Promise<TunnelConfig> {
  return invoke<TunnelConfig>('create_tunnel', { config })
}

export async function updateTunnel(config: TunnelConfig): Promise<void> {
  return invoke('update_tunnel', { config })
}

export async function deleteTunnel(id: string): Promise<void> {
  return invoke('delete_tunnel', { id })
}

export async function duplicateTunnel(id: string): Promise<TunnelConfig> {
  return invoke<TunnelConfig>('duplicate_tunnel', { id })
}

export async function reorderTunnels(ids: string[]): Promise<void> {
  return invoke('reorder_tunnels', { ids })
}

// ─── Tunnel Control ────────────────────────────────────────────

export async function startTunnel(id: string): Promise<void> {
  return invoke('start_tunnel', { id })
}

export async function stopTunnel(id: string): Promise<void> {
  return invoke('stop_tunnel', { id })
}

export async function getTunnelStatus(id: string): Promise<TunnelStatus> {
  return invoke<TunnelStatus>('get_tunnel_status', { id })
}

export async function getTunnelState(id: string): Promise<TunnelState> {
  return invoke<TunnelState>('get_tunnel_state', { id })
}

export async function getAllStatuses(): Promise<[string, TunnelStatus][]> {
  return invoke<[string, TunnelStatus][]>('get_all_statuses')
}

export async function getTunnelLogs(id: string): Promise<LogEntry[]> {
  return invoke<LogEntry[]>('get_tunnel_logs', { id })
}

// ─── Groups ────────────────────────────────────────────────────

export async function listGroups(): Promise<Group[]> {
  return invoke<Group[]>('list_groups')
}

export async function createGroup(name: string, parentId?: string | null): Promise<Group> {
  return invoke<Group>('create_group', { name, parentId: parentId ?? null })
}

export async function updateGroup(group: Group): Promise<void> {
  return invoke('update_group', { group })
}

export async function deleteGroup(id: string): Promise<void> {
  return invoke('delete_group', { id })
}

export async function reorderGroups(items: GroupReorderItem[]): Promise<void> {
  return invoke('reorder_groups', { items })
}

export async function assignTunnelToGroup(tunnelId: string, groupId: string | null): Promise<void> {
  return invoke('assign_tunnel_to_group', { tunnelId, groupId })
}

export async function startGroup(groupId: string): Promise<void> {
  return invoke('start_group', { groupId })
}

export async function stopGroup(groupId: string): Promise<void> {
  return invoke('stop_group', { groupId })
}

// ─── Import / Export ───────────────────────────────────────────

export async function exportConfigToFile(path: string): Promise<void> {
  return invoke('export_config_to_file', { path })
}

export async function importConfigFromFile(path: string): Promise<AppConfig> {
  return invoke<AppConfig>('import_config_from_file', { path })
}

// ─── SSH Command Parser ────────────────────────────────────────

export async function parseSshCommand(command: string): Promise<TunnelConfig> {
  return invoke<TunnelConfig>('parse_ssh_command', { command })
}

// ─── Autostart ─────────────────────────────────────────────────

export async function getAutostartEnabled(): Promise<boolean> {
  return invoke<boolean>('get_autostart_enabled')
}

export async function setAutostartEnabled(enabled: boolean): Promise<void> {
  return invoke('set_autostart_enabled', { enabled })
}

// ─── Event Listeners ───────────────────────────────────────────

export async function onTunnelStatusChanged(
  callback: (event: TunnelStatusEvent) => void
): Promise<UnlistenFn> {
  return listen<TunnelStatusEvent>('tunnel-status-changed', (e) => callback(e.payload))
}

export async function onTunnelLog(
  callback: (event: TunnelLogEvent) => void
): Promise<UnlistenFn> {
  return listen<TunnelLogEvent>('tunnel-log', (e) => callback(e.payload))
}
