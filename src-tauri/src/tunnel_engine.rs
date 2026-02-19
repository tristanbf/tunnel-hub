use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use russh::client;
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use tauri::Emitter;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::models::*;

// ─── AssertSend wrapper ────────────────────────────────────────
// Workaround for a known Rust compiler limitation where async methods
// on russh::client::Handle return futures that the compiler cannot prove
// are Send for all lifetimes (higher-ranked lifetime error), even though
// the concrete types (Handle, PublicKey, channels) are all actually Send.
struct AssertSend<F>(F);
// SAFETY: Handle<SshHandler> and all russh types involved are Send.
// The compiler's failure is purely with higher-ranked lifetime bounds
// in the generated async state machine, not with actual thread safety.
unsafe impl<F> Send for AssertSend<F> {}
impl<F: std::future::Future> std::future::Future for AssertSend<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { self.map_unchecked_mut(|s| &mut s.0).poll(cx) }
    }
}

// ─── SSH Client Handler ────────────────────────────────────────

struct SshHandler {
    /// For remote forwarding: the local port to connect to when receiving
    /// forwarded connections from the SSH server. `None` for non-remote modes.
    remote_forward_local_port: Arc<std::sync::Mutex<Option<u16>>>,
}

impl SshHandler {
    fn new() -> Self {
        SshHandler {
            remote_forward_local_port: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    fn with_remote_forward_port(port: u16) -> Self {
        SshHandler {
            remote_forward_local_port: Arc::new(std::sync::Mutex::new(Some(port))),
        }
    }
}

impl client::Handler for SshHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: Implement known_hosts verification in production
        Ok(true)
    }

    /// Handle incoming forwarded-tcpip channels for remote port forwarding.
    /// When the SSH server receives a connection on a remotely-forwarded port,
    /// it opens this channel. We connect to the local target and bridge data.
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<client::Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let local_port = {
            let guard = self.remote_forward_local_port.lock().unwrap();
            *guard
        };

        if let Some(port) = local_port {
            tokio::spawn(async move {
                match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                    Ok(mut tcp) => {
                        let mut stream = channel.into_stream();
                        if let Err(e) = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await {
                            log::debug!("Remote forward connection ended: {}", e);
                        }
                    }
                    Err(e) => {
                        log::warn!("Remote forward: failed to connect to 127.0.0.1:{}: {}", port, e);
                    }
                }
            });
        }

        Ok(())
    }
}

// ─── Tunnel Handle ─────────────────────────────────────────────

pub struct TunnelHandle {
    pub cancel_token: CancellationToken,
    pub status_rx: watch::Receiver<TunnelStatus>,
    task: tokio::task::JoinHandle<()>,
}

impl TunnelHandle {
    pub fn status(&self) -> TunnelStatus {
        self.status_rx.borrow().clone()
    }

    pub async fn stop(self) {
        self.cancel_token.cancel();
        let _ = self.task.await;
    }
}

// ─── Tunnel Manager ────────────────────────────────────────────

pub struct TunnelManager {
    pub handles: HashMap<String, TunnelHandle>,
    pub states: HashMap<String, TunnelState>,
    app_handle: Option<tauri::AppHandle>,
}

impl TunnelManager {
    pub fn new() -> Self {
        TunnelManager {
            handles: HashMap::new(),
            states: HashMap::new(),
            app_handle: None,
        }
    }

    pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle) {
        self.app_handle = Some(app_handle);
    }

    fn emit_status(&self, tunnel_id: &str, status: &TunnelStatus, message: Option<String>) {
        if let Some(ref app) = self.app_handle {
            let event = TunnelStatusEvent {
                tunnel_id: tunnel_id.to_string(),
                status: status.clone(),
                message,
            };
            let _ = app.emit("tunnel-status-changed", &event);
        }
    }

    fn emit_log(&self, tunnel_id: &str, level: LogLevel, message: &str) {
        if let Some(ref app) = self.app_handle {
            let entry = LogEntry {
                timestamp: chrono::Utc::now().timestamp_millis(),
                level,
                message: message.to_string(),
            };
            let event = TunnelLogEvent {
                tunnel_id: tunnel_id.to_string(),
                entry,
            };
            let _ = app.emit("tunnel-log", &event);
        }
    }

    /// Start a tunnel based on its configuration.
    pub async fn start_tunnel(&mut self, config: &TunnelConfig) -> Result<(), String> {
        let id = config.id.clone();

        // Don't start if already running
        if let Some(handle) = self.handles.get(&id) {
            if matches!(handle.status(), TunnelStatus::Running | TunnelStatus::Starting) {
                return Err("Tunnel is already running".to_string());
            }
        }

        let (status_tx, status_rx) = watch::channel(TunnelStatus::Starting);
        let cancel_token = CancellationToken::new();

        // Initialize state
        let mut state = TunnelState::new(id.clone());
        state.status = TunnelStatus::Starting;
        state.started_at = Some(chrono::Utc::now().timestamp_millis());
        state.add_log(LogLevel::Info, "Starting tunnel...".to_string());
        self.states.insert(id.clone(), state);

        // Emit starting status
        self.emit_status(&id, &TunnelStatus::Starting, None);
        self.emit_log(&id, LogLevel::Info, "Starting tunnel...");

        let config = config.clone();
        let cancel = cancel_token.clone();
        let app_handle = self.app_handle.clone();

        let task = tokio::spawn(run_tunnel_task(config, cancel, app_handle, status_tx));

        let handle = TunnelHandle {
            cancel_token,
            status_rx,
            task,
        };

        self.handles.insert(id, handle);
        Ok(())
    }

    /// Stop a running tunnel.
    pub async fn stop_tunnel(&mut self, id: &str) -> Result<(), String> {
        if let Some(handle) = self.handles.remove(id) {
            self.emit_status(id, &TunnelStatus::Stopping, None);
            self.emit_log(id, LogLevel::Info, "Stopping tunnel...");

            handle.stop().await;

            if let Some(state) = self.states.get_mut(id) {
                state.status = TunnelStatus::Stopped;
                state.add_log(LogLevel::Info, "Tunnel stopped".to_string());
            }

            self.emit_status(id, &TunnelStatus::Stopped, None);
            self.emit_log(id, LogLevel::Info, "Tunnel stopped");
            Ok(())
        } else {
            Err("Tunnel not found or not running".to_string())
        }
    }

    /// Get the current status of a tunnel.
    pub fn get_status(&self, id: &str) -> TunnelStatus {
        if let Some(handle) = self.handles.get(id) {
            handle.status()
        } else {
            self.states
                .get(id)
                .map(|s| s.status.clone())
                .unwrap_or(TunnelStatus::Stopped)
        }
    }

    /// Get the full state (status + logs) of a tunnel.
    pub fn get_state(&self, id: &str) -> TunnelState {
        self.states
            .get(id)
            .cloned()
            .unwrap_or_else(|| TunnelState::new(id.to_string()))
    }

    /// Stop all tunnels.
    pub async fn stop_all(&mut self) {
        let ids: Vec<String> = self.handles.keys().cloned().collect();
        for id in ids {
            let _ = self.stop_tunnel(&id).await;
        }
    }
}

// ─── Core Tunnel Runner ────────────────────────────────────────

/// Helper to emit a log event to the frontend.
fn do_emit_log(app: &Option<tauri::AppHandle>, tunnel_id: &str, level: LogLevel, msg: String) {
    if let Some(ref a) = app {
        let _ = a.emit("tunnel-log", &TunnelLogEvent {
            tunnel_id: tunnel_id.to_string(),
            entry: LogEntry {
                timestamp: chrono::Utc::now().timestamp_millis(),
                level,
                message: msg,
            },
        });
    }
}

/// Wrapper that is `Send + 'static` for `tokio::spawn`.
async fn run_tunnel_task(
    config: TunnelConfig,
    cancel: CancellationToken,
    app_handle: Option<tauri::AppHandle>,
    status_tx: watch::Sender<TunnelStatus>,
) {
    let tunnel_id = config.id.clone();
    let app = app_handle;

    let result = run_tunnel(config, cancel, app.clone()).await;

    match result {
        Ok(()) => {
            let _ = status_tx.send(TunnelStatus::Stopped);
            if let Some(ref a) = app {
                let _ = a.emit("tunnel-status-changed", &TunnelStatusEvent {
                    tunnel_id: tunnel_id.clone(),
                    status: TunnelStatus::Stopped,
                    message: Some("Tunnel stopped".to_string()),
                });
            }
        }
        Err(e) => {
            let err_status = TunnelStatus::Error { message: e.clone() };
            let _ = status_tx.send(err_status.clone());
            if let Some(ref a) = app {
                let _ = a.emit("tunnel-status-changed", &TunnelStatusEvent {
                    tunnel_id: tunnel_id.clone(),
                    status: err_status,
                    message: Some(e),
                });
            }
        }
    }
}

async fn run_tunnel(
    config: TunnelConfig,
    cancel: CancellationToken,
    app: Option<tauri::AppHandle>,
) -> Result<(), String> {
    let tunnel_id = config.id.clone();

    let session = establish_ssh_chain(
        config.clone(),
        app.clone(),
        tunnel_id.clone(),
    ).await?;

    do_emit_log(&app, &tunnel_id, LogLevel::Info, format!(
        "SSH connection established to {}@{}:{}",
        config.ssh_user, config.ssh_host, config.ssh_port
    ));

    match config.forward_mode {
        ForwardingMode::Local => run_local_forward(config, session, cancel, app).await,
        ForwardingMode::Remote => run_remote_forward(config, session, cancel, app).await,
        ForwardingMode::Dynamic => run_dynamic_forward(config, session, cancel, app).await,
    }
}

/// Local port forwarding: bind locally, forward each connection to remote via SSH.
async fn run_local_forward(
    config: TunnelConfig,
    session: client::Handle<SshHandler>,
    cancel: CancellationToken,
    app: Option<tauri::AppHandle>,
) -> Result<(), String> {
    let tunnel_id = config.id.clone();

    // Bind local listener
    let bind_addr: SocketAddr = format!("127.0.0.1:{}", config.local_port)
        .parse()
        .map_err(|e| format!("Invalid local address: {e}"))?;

    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|e| format!("Failed to bind port {}: {e}", config.local_port))?;

    do_emit_log(&app, &tunnel_id, LogLevel::Info, format!(
        "[Local] Listening on 127.0.0.1:{} → {}:{}",
        config.local_port, config.remote_host, config.remote_port
    ));

    // Notify running status
    if let Some(ref a) = app {
        let _ = a.emit("tunnel-status-changed", &TunnelStatusEvent {
            tunnel_id: config.id.clone(),
            status: TunnelStatus::Running,
            message: None,
        });
    }

    let session = Arc::new(session);

    // Accept connections loop
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                do_emit_log(&app, &tunnel_id, LogLevel::Info, "Cancellation requested".to_string());
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((tcp_stream, peer_addr)) => {
                        do_emit_log(&app, &tunnel_id, LogLevel::Debug, format!("Connection from {}", peer_addr));

                        let session = session.clone();
                        let remote_host = config.remote_host.clone();
                        let remote_port = config.remote_port;
                        let cancel_inner = cancel.clone();
                        let app_inner = app.clone();
                        let tid = config.id.clone();

                        tokio::spawn(handle_connection_task(
                            tcp_stream,
                            session,
                            remote_host,
                            remote_port,
                            cancel_inner,
                            app_inner,
                            tid,
                        ));
                    }
                    Err(e) => {
                        do_emit_log(&app, &tunnel_id, LogLevel::Error, format!("Accept error: {e}"));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Remote port forwarding: ask the SSH server to listen on remote_port,
/// forward incoming connections to local_host:local_port.
async fn run_remote_forward(
    config: TunnelConfig,
    mut session: client::Handle<SshHandler>,
    cancel: CancellationToken,
    app: Option<tauri::AppHandle>,
) -> Result<(), String> {
    let tunnel_id = config.id.clone();

    // Request remote forwarding from the SSH server
    let listen_port = session
        .tcpip_forward(config.remote_host.clone(), config.remote_port.into())
        .await
        .map_err(|e| format!("Remote forward request failed: {e}"))?;

    do_emit_log(&app, &tunnel_id, LogLevel::Info, format!(
        "[Remote] Server listening on {}:{} → 127.0.0.1:{}",
        config.remote_host, listen_port, config.local_port
    ));

    // Notify running status
    if let Some(ref a) = app {
        let _ = a.emit("tunnel-status-changed", &TunnelStatusEvent {
            tunnel_id: config.id.clone(),
            status: TunnelStatus::Running,
            message: None,
        });
    }

    // Wait for cancellation — the actual forwarding is handled by the SSH handler
    // (forwarded_tcpip channel requests from the server)
    cancel.cancelled().await;
    do_emit_log(&app, &tunnel_id, LogLevel::Info, "Cancellation requested".to_string());

    // Cancel remote forwarding
    let _ = session
        .cancel_tcpip_forward(config.remote_host.clone(), config.remote_port.into())
        .await;

    Ok(())
}

/// Dynamic port forwarding (SOCKS5 proxy): bind locally, parse SOCKS5 handshake,
/// open direct-tcpip to the requested target via SSH.
async fn run_dynamic_forward(
    config: TunnelConfig,
    session: client::Handle<SshHandler>,
    cancel: CancellationToken,
    app: Option<tauri::AppHandle>,
) -> Result<(), String> {
    let tunnel_id = config.id.clone();

    let bind_addr: SocketAddr = format!("127.0.0.1:{}", config.local_port)
        .parse()
        .map_err(|e| format!("Invalid local address: {e}"))?;

    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|e| format!("Failed to bind SOCKS5 port {}: {e}", config.local_port))?;

    do_emit_log(&app, &tunnel_id, LogLevel::Info, format!(
        "[Dynamic] SOCKS5 proxy listening on 127.0.0.1:{}",
        config.local_port
    ));

    // Notify running status
    if let Some(ref a) = app {
        let _ = a.emit("tunnel-status-changed", &TunnelStatusEvent {
            tunnel_id: config.id.clone(),
            status: TunnelStatus::Running,
            message: None,
        });
    }

    let session = Arc::new(session);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                do_emit_log(&app, &tunnel_id, LogLevel::Info, "Cancellation requested".to_string());
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((tcp_stream, peer_addr)) => {
                        do_emit_log(&app, &tunnel_id, LogLevel::Debug, format!("SOCKS5 connection from {}", peer_addr));
                        let session = session.clone();
                        let cancel_inner = cancel.clone();
                        let app_inner = app.clone();
                        let tid = config.id.clone();

                        tokio::spawn(handle_socks5_connection(
                            tcp_stream, session, cancel_inner, app_inner, tid,
                        ));
                    }
                    Err(e) => {
                        do_emit_log(&app, &tunnel_id, LogLevel::Error, format!("Accept error: {e}"));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Handle a single SOCKS5 connection.
async fn handle_socks5_connection(
    mut tcp: tokio::net::TcpStream,
    session: Arc<client::Handle<SshHandler>>,
    cancel: CancellationToken,
    app: Option<tauri::AppHandle>,
    tunnel_id: String,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let result: Result<(), String> = async {
        // ─── SOCKS5 greeting ───
        let mut buf = [0u8; 258];
        let n = tcp.read(&mut buf).await.map_err(|e| format!("SOCKS5 read greeting: {e}"))?;
        if n < 2 || buf[0] != 0x05 {
            return Err("Not a SOCKS5 request".to_string());
        }
        // No auth required
        tcp.write_all(&[0x05, 0x00]).await.map_err(|e| format!("SOCKS5 write auth: {e}"))?;

        // ─── SOCKS5 request ───
        let n = tcp.read(&mut buf).await.map_err(|e| format!("SOCKS5 read request: {e}"))?;
        if n < 4 || buf[0] != 0x05 || buf[1] != 0x01 {
            // Only CONNECT (0x01) supported
            let _ = tcp.write_all(&[0x05, 0x07, 0x00, 0x01, 0,0,0,0, 0,0]).await;
            return Err("Unsupported SOCKS5 command".to_string());
        }

        let (target_host, target_port) = match buf[3] {
            0x01 => {
                // IPv4
                if n < 10 { return Err("SOCKS5 request too short for IPv4".to_string()); }
                let ip = format!("{}.{}.{}.{}", buf[4], buf[5], buf[6], buf[7]);
                let port = u16::from_be_bytes([buf[8], buf[9]]);
                (ip, port)
            }
            0x03 => {
                // Domain name
                let domain_len = buf[4] as usize;
                if n < 5 + domain_len + 2 {
                    return Err("SOCKS5 request too short for domain".to_string());
                }
                let domain = String::from_utf8_lossy(&buf[5..5 + domain_len]).to_string();
                let port = u16::from_be_bytes([buf[5 + domain_len], buf[6 + domain_len]]);
                (domain, port)
            }
            0x04 => {
                // IPv6
                if n < 22 { return Err("SOCKS5 request too short for IPv6".to_string()); }
                let mut parts = Vec::new();
                for i in 0..8 {
                    parts.push(format!("{:02x}{:02x}", buf[4 + i * 2], buf[5 + i * 2]));
                }
                let ip = parts.join(":");
                let port = u16::from_be_bytes([buf[20], buf[21]]);
                (ip, port)
            }
            _ => {
                let _ = tcp.write_all(&[0x05, 0x08, 0x00, 0x01, 0,0,0,0, 0,0]).await;
                return Err("Unknown SOCKS5 address type".to_string());
            }
        };

        do_emit_log(&app, &tunnel_id, LogLevel::Debug, format!(
            "SOCKS5 CONNECT {}:{}", target_host, target_port
        ));

        // Open SSH channel to target
        let channel = session
            .channel_open_direct_tcpip(
                target_host.clone(),
                target_port.into(),
                "127.0.0.1",
                0,
            )
            .await
            .map_err(|e| format!("direct-tcpip to {}:{} error: {e}", target_host, target_port))?;

        // Send SOCKS5 success reply
        tcp.write_all(&[0x05, 0x00, 0x00, 0x01, 0,0,0,0, 0,0])
            .await
            .map_err(|e| format!("SOCKS5 write reply: {e}"))?;

        // Bridge
        let mut stream = channel.into_stream();
        let (mut tcp_read, mut tcp_write) = tcp.split();
        let (mut ssh_read, mut ssh_write) = tokio::io::split(&mut stream);

        tokio::select! {
            _ = cancel.cancelled() => {}
            result = async {
                let t2s = tokio::io::copy(&mut tcp_read, &mut ssh_write);
                let s2t = tokio::io::copy(&mut ssh_read, &mut tcp_write);
                tokio::try_join!(t2s, s2t)
            } => {
                if let Err(e) = result {
                    log::debug!("SOCKS5 stream ended: {}", e);
                }
            }
        }

        Ok(())
    }.await;

    if let Err(e) = result {
        do_emit_log(&app, &tunnel_id, LogLevel::Warn, format!("SOCKS5 error: {e}"));
    }
}

/// Establish an SSH session, potentially through a chain of jump hosts.
/// Returns a boxed future to help the compiler prove Send for tokio::spawn.
fn establish_ssh_chain(
    config: TunnelConfig,
    app: Option<tauri::AppHandle>,
    tunnel_id: String,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<client::Handle<SshHandler>, String>> + Send>>
{
    Box::pin(async move {
    let ssh_config = Arc::new(client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(30)),
        keepalive_interval: Some(std::time::Duration::from_secs(15)),
        ..<_>::default()
    });

    if config.jump_hosts.is_empty() {
        // Direct connection
        do_emit_log(&app, &tunnel_id, LogLevel::Info, format!(
            "Connecting to {}@{}:{}...",
            config.ssh_user, config.ssh_host, config.ssh_port
        ));

        let handler = if matches!(config.forward_mode, ForwardingMode::Remote) {
            SshHandler::with_remote_forward_port(config.local_port)
        } else {
            SshHandler::new()
        };
        let session = client::connect(
            ssh_config,
            (config.ssh_host.as_str(), config.ssh_port),
            handler,
        )
        .await
        .map_err(|e| format!("SSH connect error: {e}"))?;

        let session = authenticate(
            session,
            config.ssh_user.clone(),
            config.auth.clone(),
            app.clone(),
            tunnel_id.clone(),
        ).await?;

        Ok(session)
    } else {
        // Chain through jump hosts
        let mut current_session: Option<client::Handle<SshHandler>> = None;

        for (i, jump) in config.jump_hosts.iter().enumerate() {
            do_emit_log(&app, &tunnel_id, LogLevel::Info, format!(
                "Connecting to jump host {}/{}: {}@{}:{}...",
                i + 1, config.jump_hosts.len(), jump.user, jump.host, jump.port
            ));

            if let Some(prev_session) = current_session {
                let channel = prev_session
                    .channel_open_direct_tcpip(
                        jump.host.clone(),
                        jump.port.into(),
                        "127.0.0.1",
                        0,
                    )
                    .await
                    .map_err(|e| format!("Jump channel error: {e}"))?;

                let stream = channel.into_stream();
                let session = client::connect_stream(
                    ssh_config.clone(),
                    stream,
                    SshHandler::new(),
                )
                .await
                .map_err(|e| format!("SSH connect via jump error: {e}"))?;

                let session = authenticate(
                    session,
                    jump.user.clone(),
                    jump.auth.clone(),
                    app.clone(),
                    tunnel_id.clone(),
                ).await?;
                current_session = Some(session);
            } else {
                let session = client::connect(
                    ssh_config.clone(),
                    (jump.host.as_str(), jump.port),
                    SshHandler::new(),
                )
                .await
                .map_err(|e| format!("SSH connect error (jump {}): {e}", i + 1))?;

                let session = authenticate(
                    session,
                    jump.user.clone(),
                    jump.auth.clone(),
                    app.clone(),
                    tunnel_id.clone(),
                ).await?;
                current_session = Some(session);
            }

            do_emit_log(&app, &tunnel_id, LogLevel::Info, format!(
                "Jump host {} connected", i + 1
            ));
        }

        // Final hop
        let prev = current_session.ok_or("No jump sessions established")?;

        do_emit_log(&app, &tunnel_id, LogLevel::Info, format!(
            "Connecting to target {}@{}:{} through jump chain...",
            config.ssh_user, config.ssh_host, config.ssh_port
        ));

        let channel = prev
            .channel_open_direct_tcpip(
                config.ssh_host.clone(),
                config.ssh_port.into(),
                "127.0.0.1",
                0,
            )
            .await
            .map_err(|e| format!("Final hop channel error: {e}"))?;

        let stream = channel.into_stream();
        let handler = if matches!(config.forward_mode, ForwardingMode::Remote) {
            SshHandler::with_remote_forward_port(config.local_port)
        } else {
            SshHandler::new()
        };
        let session = client::connect_stream(
            ssh_config,
            stream,
            handler,
        )
        .await
        .map_err(|e| format!("SSH connect to target error: {e}"))?;

        let session = authenticate(
            session,
            config.ssh_user.clone(),
            config.auth.clone(),
            app.clone(),
            tunnel_id.clone(),
        ).await?;

        Ok(session)
    }
    }) // Box::pin
}

/// Authenticate an SSH session with the given auth config.
/// Takes ownership of the Handle and returns it after successful authentication.
/// Returns a boxed future to help the compiler prove Send for tokio::spawn.
fn authenticate(
    mut session: client::Handle<SshHandler>,
    user: String,
    auth: AuthConfig,
    app: Option<tauri::AppHandle>,
    tunnel_id: String,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<client::Handle<SshHandler>, String>> + Send>>
{
    Box::pin(async move {
    match auth {
        AuthConfig::Password { password } => {
            do_emit_log(&app, &tunnel_id, LogLevel::Debug, "Authenticating with password...".to_string());
            let result = session
                .authenticate_password(user.clone(), password)
                .await
                .map_err(|e| format!("Password auth error: {e}"))?;
            if !result.success() {
                return Err("Password authentication failed".to_string());
            }
        }
        AuthConfig::KeyFile { path, passphrase } => {
            do_emit_log(&app, &tunnel_id, LogLevel::Debug, format!("Authenticating with key: {}", path));
            let key_data = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read key file {}: {e}", path))?;
            let private_key = if let Some(pp) = passphrase {
                PrivateKey::from_openssh(key_data.as_bytes())
                    .and_then(|k| k.decrypt(pp))
                    .map_err(|e| format!("Failed to parse/decrypt key {}: {e}", path))?
            } else {
                PrivateKey::from_openssh(key_data.as_bytes())
                    .map_err(|e| format!("Failed to parse key {}: {e}", path))?
            };

            let hash_alg = if private_key.algorithm().is_rsa() {
                match session.best_supported_rsa_hash().await {
                    Ok(Some(alg)) => alg,
                    Ok(None) => None,
                    Err(_) => None,
                }
            } else {
                None
            };

            let key_with_hash = PrivateKeyWithHashAlg::new(
                Arc::new(private_key),
                hash_alg,
            );

            let result = session
                .authenticate_publickey(user.clone(), key_with_hash)
                .await
                .map_err(|e| format!("Key auth error: {e}"))?;
            if !result.success() {
                return Err("Public key authentication failed".to_string());
            }
        }
        AuthConfig::Agent => {
            do_emit_log(&app, &tunnel_id, LogLevel::Debug, "Authenticating with SSH agent...".to_string());
            session = authenticate_with_agent(session, user.clone(), app.clone(), tunnel_id.clone()).await?;
        }
    }

    do_emit_log(&app, &tunnel_id, LogLevel::Info, format!("Authenticated as {}", user));
    Ok(session)
    }) // Box::pin
}

/// Perform SSH agent authentication.
/// Returns a boxed Send future. The agent client types may not be Send on all platforms,
/// but the function itself only holds owned data and should be safe to use from Send contexts.
fn authenticate_with_agent(
    mut session: client::Handle<SshHandler>,
    user: String,
    app: Option<tauri::AppHandle>,
    tunnel_id: String,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<client::Handle<SshHandler>, String>> + Send>>
{
    Box::pin(AssertSend(async move {
    let mut agent = {
        let pipe_path = r"\\.\pipe\openssh-ssh-agent";
        match russh::keys::agent::client::AgentClient::connect_named_pipe(pipe_path).await {
            Ok(client) => client.dynamic(),
            Err(_) => {
                russh::keys::agent::client::AgentClient::connect_pageant()
                    .await
                    .map_err(|e| format!("SSH agent connect error (Pageant): {e}"))?
                    .dynamic()
            }
        }
    };

    #[cfg(unix)]
    let mut agent = {
        russh::keys::agent::client::AgentClient::connect_env()
            .await
            .map_err(|e| format!("SSH agent connect error: {e}"))?
    };

    let identities = agent
        .request_identities()
        .await
        .map_err(|e| format!("SSH agent identities error: {e}"))?;

    if identities.is_empty() {
        return Err("No identities found in SSH agent".to_string());
    }

    let mut authenticated = false;
    for identity in identities {
        let hash_alg = if identity.algorithm().is_rsa() {
            match session.best_supported_rsa_hash().await {
                Ok(Some(alg)) => alg,
                _ => None,
            }
        } else {
            None
        };

        match session
            .authenticate_publickey_with(user.clone(), identity, hash_alg, &mut agent)
            .await
        {
            Ok(russh::client::AuthResult::Success) => {
                authenticated = true;
                break;
            }
            _ => continue,
        }
    }

    if !authenticated {
        return Err("SSH agent authentication failed with all keys".to_string());
    }

    do_emit_log(&app, &tunnel_id, LogLevel::Info, "SSH agent authentication successful".to_string());
    Ok(session)
    })) // Box::pin(AssertSend(...))
}

/// Handle a single TCP connection by bridging it to an SSH channel.
async fn handle_connection_task(
    mut tcp_stream: tokio::net::TcpStream,
    session: Arc<client::Handle<SshHandler>>,
    remote_host: String,
    remote_port: u16,
    cancel: CancellationToken,
    app: Option<tauri::AppHandle>,
    tunnel_id: String,
) {
    if let Err(e) = handle_connection(&mut tcp_stream, &session, &remote_host, remote_port, cancel).await {
        log::warn!("Connection error: {}", e);
        if let Some(ref a) = app {
            let _ = a.emit("tunnel-log", &TunnelLogEvent {
                tunnel_id,
                entry: LogEntry {
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    level: LogLevel::Warn,
                    message: format!("Connection error: {e}"),
                },
            });
        }
    }
}

async fn handle_connection(
    tcp_stream: &mut tokio::net::TcpStream,
    session: &client::Handle<SshHandler>,
    remote_host: &str,
    remote_port: u16,
    cancel: CancellationToken,
) -> Result<(), String> {
    let channel = session
        .channel_open_direct_tcpip(
            remote_host.to_string(),
            remote_port.into(),
            "127.0.0.1",
            0,
        )
        .await
        .map_err(|e| format!("direct-tcpip error: {e}"))?;

    let mut stream = channel.into_stream();
    let (mut tcp_read, mut tcp_write) = tcp_stream.split();
    let (mut ssh_read, mut ssh_write) = tokio::io::split(&mut stream);

    tokio::select! {
        _ = cancel.cancelled() => {}
        result = async {
            let t2s = tokio::io::copy(&mut tcp_read, &mut ssh_write);
            let s2t = tokio::io::copy(&mut ssh_read, &mut tcp_write);
            tokio::try_join!(t2s, s2t)
        } => {
            if let Err(e) = result {
                log::debug!("Connection stream ended: {}", e);
            }
        }
    }

    Ok(())
}
