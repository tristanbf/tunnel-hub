// ─── Auth Config ───────────────────────────────────────────────

export interface PasswordAuth {
  type: 'password'
  password: string
}

export interface KeyFileAuth {
  type: 'keyfile'
  path: string
  passphrase?: string
}

export interface AgentAuth {
  type: 'agent'
}

export type AuthConfig = PasswordAuth | KeyFileAuth | AgentAuth

// ─── Jump Host ─────────────────────────────────────────────────

export interface JumpHost {
  host: string
  port: number
  user: string
  auth: AuthConfig
}

// ─── Forwarding Mode ───────────────────────────────────────────

export type ForwardingMode = 'local' | 'remote' | 'dynamic'

// ─── Tunnel Config ─────────────────────────────────────────────

export interface TunnelConfig {
  id: string
  name: string
  forward_mode: ForwardingMode
  local_port: number
  remote_host: string
  remote_port: number
  ssh_user: string
  ssh_host: string
  ssh_port: number
  auth: AuthConfig
  group_id: string | null
  jump_hosts: JumpHost[]
  auto_reconnect: boolean
  extra_args: string | null
}

// ─── Group ─────────────────────────────────────────────────────

export interface Group {
  id: string
  name: string
  parent_id: string | null
}

// ─── Tunnel Status ─────────────────────────────────────────────

export type TunnelStatusType = 'stopped' | 'starting' | 'running' | 'stopping' | 'error' | 'reconnecting'

export interface TunnelStatusStopped {
  status: 'stopped'
}

export interface TunnelStatusStarting {
  status: 'starting'
}

export interface TunnelStatusRunning {
  status: 'running'
}

export interface TunnelStatusStopping {
  status: 'stopping'
}

export interface TunnelStatusError {
  status: 'error'
  message: string
}

export interface TunnelStatusReconnecting {
  status: 'reconnecting'
}

export type TunnelStatus =
  | TunnelStatusStopped
  | TunnelStatusStarting
  | TunnelStatusRunning
  | TunnelStatusStopping
  | TunnelStatusError
  | TunnelStatusReconnecting

// ─── Log Entry ─────────────────────────────────────────────────

export type LogLevel = 'info' | 'warn' | 'error' | 'debug'

export interface LogEntry {
  timestamp: number
  level: LogLevel
  message: string
}

// ─── Tunnel State ──────────────────────────────────────────────

export interface TunnelState {
  tunnel_id: string
  status: TunnelStatus
  logs: LogEntry[]
  started_at: number | null
  connected_at: number | null
}

// ─── Events ────────────────────────────────────────────────────

export interface TunnelStatusEvent {
  tunnel_id: string
  status: TunnelStatus
  message?: string
}

export interface TunnelLogEvent {
  tunnel_id: string
  entry: LogEntry
}

// ─── App Config (for import/export) ────────────────────────────

export interface AppConfig {
  tunnels: TunnelConfig[]
  groups: Group[]
}

// ─── UI Helpers ────────────────────────────────────────────────

export interface GroupTreeNode {
  key: string
  label: string
  children?: GroupTreeNode[]
  isGroup: boolean
}

export function getStatusType(status: TunnelStatus): TunnelStatusType {
  return status.status
}

export function getForwardingModeLabel(mode: ForwardingMode): string {
  switch (mode) {
    case 'local': return '本地转发 (L)'
    case 'remote': return '远程转发 (R)'
    case 'dynamic': return '动态代理 (D)'
    default: return '本地转发 (L)'
  }
}

export function getForwardingModeTag(mode: ForwardingMode): string {
  switch (mode) {
    case 'local': return 'L'
    case 'remote': return 'R'
    case 'dynamic': return 'D'
    default: return 'L'
  }
}

export function getStatusColor(status: TunnelStatus): string {
  switch (status.status) {
    case 'running': return '#18a058'
    case 'starting': case 'reconnecting': return '#f0a020'
    case 'error': return '#d03050'
    case 'stopping': return '#f0a020'
    default: return '#909399'
  }
}

export function getStatusLabel(status: TunnelStatus): string {
  switch (status.status) {
    case 'running': return '运行中'
    case 'starting': return '启动中'
    case 'stopped': return '已停止'
    case 'stopping': return '停止中'
    case 'error': return '错误'
    case 'reconnecting': return '重连中'
    default: return '未知'
  }
}

export function createDefaultTunnel(): TunnelConfig {
  return {
    id: '',
    name: '',
    forward_mode: 'local',
    local_port: 0,
    remote_host: '127.0.0.1',
    remote_port: 22,
    ssh_user: '',
    ssh_host: '',
    ssh_port: 22,
    auth: { type: 'agent' },
    group_id: null,
    jump_hosts: [],
    auto_reconnect: false,
    extra_args: null,
  }
}

export function createDefaultJumpHost(): JumpHost {
  return {
    host: '',
    port: 22,
    user: '',
    auth: { type: 'agent' },
  }
}

// ─── SSH Command Generator ─────────────────────────────────────

export function generateSshCommand(config: TunnelConfig): string {
  const parts: string[] = ['ssh', '-fNg', '-o', 'ServerAliveInterval=60']

  // Jump hosts
  if (config.jump_hosts.length > 0) {
    const jumps = config.jump_hosts.map(j => {
      const userPart = j.user ? `${j.user}@` : ''
      const portPart = j.port !== 22 ? `:${j.port}` : ''
      return `${userPart}${j.host}${portPart}`
    })
    parts.push('-J', jumps.join(','))
  }

  // Auth
  if (config.auth.type === 'keyfile') {
    const path = (config.auth as KeyFileAuth).path
    parts.push('-i', path.includes(' ') ? `"${path}"` : path)
  }

  // Forwarding
  switch (config.forward_mode) {
    case 'local':
      parts.push(`-L ${config.local_port}:${config.remote_host}:${config.remote_port}`)
      break
    case 'remote':
      parts.push(`-R ${config.remote_port}:${config.remote_host}:${config.local_port}`)
      break
    case 'dynamic':
      parts.push(`-D ${config.local_port}`)
      break
  }

  // Destination
  const dest = config.ssh_user ? `${config.ssh_user}@${config.ssh_host}` : config.ssh_host
  parts.push(dest)

  // Port
  if (config.ssh_port !== 22) {
    parts.push('-p', String(config.ssh_port))
  }

  return parts.join(' ')
}
