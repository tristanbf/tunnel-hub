use serde::{Deserialize, Serialize};

// ─── Authentication ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    Password { password: String },
    KeyFile { path: String, passphrase: Option<String> },
    Agent,
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig::Agent
    }
}

// ─── Jump Host ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JumpHost {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: AuthConfig,
}

// ─── Forwarding Mode ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ForwardingMode {
    Local,
    Remote,
    Dynamic,
}

impl Default for ForwardingMode {
    fn default() -> Self {
        ForwardingMode::Local
    }
}

// ─── Tunnel Config ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub forward_mode: ForwardingMode,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub ssh_user: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub auth: AuthConfig,
    pub group_id: Option<String>,
    pub jump_hosts: Vec<JumpHost>,
    pub auto_reconnect: bool,
    pub extra_args: Option<String>,
}

impl TunnelConfig {
    pub fn new_with_id() -> Self {
        TunnelConfig {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            forward_mode: ForwardingMode::default(),
            local_port: 0,
            remote_host: String::from("127.0.0.1"),
            remote_port: 22,
            ssh_user: String::new(),
            ssh_host: String::new(),
            ssh_port: 22,
            auth: AuthConfig::default(),
            group_id: None,
            jump_hosts: Vec::new(),
            auto_reconnect: false,
            extra_args: None,
        }
    }
}

// ─── Group ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
}

impl Group {
    pub fn new(name: String, parent_id: Option<String>) -> Self {
        Group {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            parent_id,
        }
    }
}

// ─── Tunnel Status ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum TunnelStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error { message: String },
    Reconnecting,
}

impl Default for TunnelStatus {
    fn default() -> Self {
        TunnelStatus::Stopped
    }
}

// ─── Tunnel Runtime State ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelState {
    pub tunnel_id: String,
    pub status: TunnelStatus,
    pub logs: Vec<LogEntry>,
    pub started_at: Option<i64>,
    pub connected_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: i64,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl TunnelState {
    pub fn new(tunnel_id: String) -> Self {
        TunnelState {
            tunnel_id,
            status: TunnelStatus::Stopped,
            logs: Vec::new(),
            started_at: None,
            connected_at: None,
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: String) {
        let entry = LogEntry {
            timestamp: chrono::Utc::now().timestamp_millis(),
            level,
            message,
        };
        self.logs.push(entry);
        // Keep last 500 log entries
        if self.logs.len() > 500 {
            self.logs.drain(0..self.logs.len() - 500);
        }
    }
}

// ─── App Config (persisted to disk) ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub tunnels: Vec<TunnelConfig>,
    pub groups: Vec<Group>,
}

// ─── Events (Rust → Frontend) ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStatusEvent {
    pub tunnel_id: String,
    pub status: TunnelStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelLogEvent {
    pub tunnel_id: String,
    pub entry: LogEntry,
}
