use crate::models::*;

/// Parse an SSH command string into a TunnelConfig.
///
/// Supported formats:
/// - `ssh -L local:remote_host:remote_port user@host -p port`
/// - `ssh -R remote_port:remote_host:local_port user@host -p port`
/// - `ssh -D local_port user@host -p port`
/// - `ssh -J jump_user@jump_host:jump_port user@host`
/// - `-o ServerAliveInterval=N` → ignored (display only)
/// - `-i keyfile` → key file auth
/// - `-fNg` combined flags → ignored (display only)
pub fn parse_ssh_command(input: &str) -> Result<TunnelConfig, String> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err("空命令".to_string());
    }

    let mut tunnel = TunnelConfig::new_with_id();
    tunnel.ssh_port = 22;
    tunnel.auth = AuthConfig::Agent;

    let mut i = 0;
    // Skip leading "ssh" if present
    if tokens[i].eq_ignore_ascii_case("ssh") {
        i += 1;
    }

    let mut forward_set = false;
    let mut jump_hosts_str: Vec<String> = Vec::new();

    while i < tokens.len() {
        let token = &tokens[i];

        if token.starts_with('-') && token.len() > 1 && !token.starts_with("--") {
            // Could be combined flags like -fNgL or individual -L, -R, -D, -p, -i, -J, -o
            let chars: Vec<char> = token[1..].chars().collect();
            let mut j = 0;
            while j < chars.len() {
                match chars[j] {
                    'L' => {
                        // -L [bind_address:]port:host:hostport
                        // Might be inline (-L13389:host:3389) or next token
                        let arg = get_flag_arg(&chars, j + 1, &tokens, &mut i)?;
                        parse_local_forward(&arg, &mut tunnel)?;
                        forward_set = true;
                        j = chars.len(); // consumed rest
                    }
                    'R' => {
                        let arg = get_flag_arg(&chars, j + 1, &tokens, &mut i)?;
                        parse_remote_forward(&arg, &mut tunnel)?;
                        forward_set = true;
                        j = chars.len();
                    }
                    'D' => {
                        let arg = get_flag_arg(&chars, j + 1, &tokens, &mut i)?;
                        parse_dynamic_forward(&arg, &mut tunnel)?;
                        forward_set = true;
                        j = chars.len();
                    }
                    'p' => {
                        let arg = get_flag_arg(&chars, j + 1, &tokens, &mut i)?;
                        tunnel.ssh_port = arg.parse::<u16>()
                            .map_err(|_| format!("无效的端口号: {}", arg))?;
                        j = chars.len();
                    }
                    'i' => {
                        let arg = get_flag_arg(&chars, j + 1, &tokens, &mut i)?;
                        tunnel.auth = AuthConfig::KeyFile {
                            path: arg,
                            passphrase: None,
                        };
                        j = chars.len();
                    }
                    'J' => {
                        let arg = get_flag_arg(&chars, j + 1, &tokens, &mut i)?;
                        // -J can have comma-separated jump hosts
                        for jh in arg.split(',') {
                            jump_hosts_str.push(jh.trim().to_string());
                        }
                        j = chars.len();
                    }
                    'o' => {
                        // -o key=value → skip (consume the argument)
                        let _arg = get_flag_arg(&chars, j + 1, &tokens, &mut i)?;
                        j = chars.len();
                    }
                    // Common no-arg flags: f, N, g, q, T, C, v, etc.
                    'f' | 'N' | 'g' | 'q' | 'T' | 'C' | 'v' | 'n' | 'x' | 'X' | 'A' => {
                        j += 1;
                    }
                    _ => {
                        // Unknown single-char flag, skip
                        j += 1;
                    }
                }
            }
        } else if token.starts_with("--") {
            // Long options, skip
        } else {
            // Positional argument: user@host or just host
            parse_destination(token, &mut tunnel)?;
        }

        i += 1;
    }

    // Parse jump hosts
    for jh_str in &jump_hosts_str {
        tunnel.jump_hosts.push(parse_jump_host_str(jh_str)?);
    }

    // Generate a name from the config
    if tunnel.name.is_empty() {
        let mode_tag = match tunnel.forward_mode {
            ForwardingMode::Local => "L",
            ForwardingMode::Remote => "R",
            ForwardingMode::Dynamic => "D",
        };
        tunnel.name = format!(
            "{} {}:{}",
            mode_tag,
            tunnel.ssh_host,
            tunnel.local_port
        );
    }

    if !forward_set {
        return Err("未找到端口转发参数 (-L, -R, -D)".to_string());
    }

    if tunnel.ssh_host.is_empty() {
        return Err("未找到 SSH 目标主机".to_string());
    }

    Ok(tunnel)
}

/// Extract the argument for a flag. It may be inline (rest of chars) or the next token.
fn get_flag_arg(chars: &[char], from: usize, tokens: &[String], i: &mut usize) -> Result<String, String> {
    if from < chars.len() {
        // Inline: e.g., -L13389:host:3389 or -p22
        Ok(chars[from..].iter().collect())
    } else {
        // Next token
        *i += 1;
        if *i < tokens.len() {
            Ok(tokens[*i].clone())
        } else {
            Err("参数缺失".to_string())
        }
    }
}

/// Parse -L [bind_address:]port:host:hostport
fn parse_local_forward(arg: &str, tunnel: &mut TunnelConfig) -> Result<(), String> {
    tunnel.forward_mode = ForwardingMode::Local;
    let parts: Vec<&str> = arg.split(':').collect();
    match parts.len() {
        3 => {
            // port:host:hostport
            tunnel.local_port = parts[0].parse().map_err(|_| format!("无效本地端口: {}", parts[0]))?;
            tunnel.remote_host = parts[1].to_string();
            tunnel.remote_port = parts[2].parse().map_err(|_| format!("无效远程端口: {}", parts[2]))?;
        }
        4 => {
            // bind_address:port:host:hostport
            tunnel.local_port = parts[1].parse().map_err(|_| format!("无效本地端口: {}", parts[1]))?;
            tunnel.remote_host = parts[2].to_string();
            tunnel.remote_port = parts[3].parse().map_err(|_| format!("无效远程端口: {}", parts[3]))?;
        }
        _ => return Err(format!("无效的 -L 参数: {}", arg)),
    }
    Ok(())
}

/// Parse -R [bind_address:]port:host:hostport
fn parse_remote_forward(arg: &str, tunnel: &mut TunnelConfig) -> Result<(), String> {
    tunnel.forward_mode = ForwardingMode::Remote;
    let parts: Vec<&str> = arg.split(':').collect();
    match parts.len() {
        3 => {
            tunnel.remote_port = parts[0].parse().map_err(|_| format!("无效远程端口: {}", parts[0]))?;
            tunnel.remote_host = parts[1].to_string();
            tunnel.local_port = parts[2].parse().map_err(|_| format!("无效本地端口: {}", parts[2]))?;
        }
        4 => {
            tunnel.remote_port = parts[1].parse().map_err(|_| format!("无效远程端口: {}", parts[1]))?;
            tunnel.remote_host = parts[2].to_string();
            tunnel.local_port = parts[3].parse().map_err(|_| format!("无效本地端口: {}", parts[3]))?;
        }
        _ => return Err(format!("无效的 -R 参数: {}", arg)),
    }
    Ok(())
}

/// Parse -D [bind_address:]port
fn parse_dynamic_forward(arg: &str, tunnel: &mut TunnelConfig) -> Result<(), String> {
    tunnel.forward_mode = ForwardingMode::Dynamic;
    let parts: Vec<&str> = arg.split(':').collect();
    let port_str = parts.last().ok_or("无效的 -D 参数")?;
    tunnel.local_port = port_str.parse().map_err(|_| format!("无效端口: {}", port_str))?;
    Ok(())
}

/// Parse user@host or just host
fn parse_destination(token: &str, tunnel: &mut TunnelConfig) -> Result<(), String> {
    if token.contains('@') {
        let parts: Vec<&str> = token.splitn(2, '@').collect();
        tunnel.ssh_user = parts[0].to_string();
        tunnel.ssh_host = parts[1].to_string();
    } else {
        tunnel.ssh_host = token.to_string();
    }
    Ok(())
}

/// Parse a jump host string like "user@host:port" or "user@host" or "host"
fn parse_jump_host_str(s: &str) -> Result<JumpHost, String> {
    let mut user = String::new();
    let host;
    let mut port: u16 = 22;

    let rest = if s.contains('@') {
        let parts: Vec<&str> = s.splitn(2, '@').collect();
        user = parts[0].to_string();
        parts[1]
    } else {
        s
    };

    // host:port or just host
    if let Some(colon_pos) = rest.rfind(':') {
        let maybe_port = &rest[colon_pos + 1..];
        if let Ok(p) = maybe_port.parse::<u16>() {
            host = rest[..colon_pos].to_string();
            port = p;
        } else {
            host = rest.to_string();
        }
    } else {
        host = rest.to_string();
    }

    Ok(JumpHost {
        host,
        port,
        user,
        auth: AuthConfig::Agent,
    })
}

/// Tokenize a command string, handling quoted strings.
fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

/// Generate an SSH command string from a TunnelConfig (for display).
pub fn generate_ssh_command(config: &TunnelConfig) -> String {
    let mut parts = vec!["ssh".to_string()];

    // Common flags
    parts.push("-fNg".to_string());
    parts.push("-o".to_string());
    parts.push("ServerAliveInterval=60".to_string());

    // Jump hosts
    if !config.jump_hosts.is_empty() {
        let jumps: Vec<String> = config.jump_hosts.iter().map(|j| {
            if j.user.is_empty() {
                if j.port == 22 {
                    j.host.clone()
                } else {
                    format!("{}:{}", j.host, j.port)
                }
            } else if j.port == 22 {
                format!("{}@{}", j.user, j.host)
            } else {
                format!("{}@{}:{}", j.user, j.host, j.port)
            }
        }).collect();
        parts.push("-J".to_string());
        parts.push(jumps.join(","));
    }

    // Auth
    match &config.auth {
        AuthConfig::KeyFile { path, .. } => {
            parts.push("-i".to_string());
            if path.contains(' ') {
                parts.push(format!("\"{}\"", path));
            } else {
                parts.push(path.clone());
            }
        }
        _ => {}
    }

    // Forwarding
    match config.forward_mode {
        ForwardingMode::Local => {
            parts.push(format!(
                "-L {}:{}:{}",
                config.local_port, config.remote_host, config.remote_port
            ));
        }
        ForwardingMode::Remote => {
            parts.push(format!(
                "-R {}:{}:{}",
                config.remote_port, config.remote_host, config.local_port
            ));
        }
        ForwardingMode::Dynamic => {
            parts.push(format!("-D {}", config.local_port));
        }
    }

    // Destination
    let dest = if config.ssh_user.is_empty() {
        config.ssh_host.clone()
    } else {
        format!("{}@{}", config.ssh_user, config.ssh_host)
    };
    parts.push(dest);

    // Port
    if config.ssh_port != 22 {
        parts.push("-p".to_string());
        parts.push(config.ssh_port.to_string());
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_forward() {
        let cmd = "ssh -fNg -o ServerAliveInterval=60 -L 13389:192.168.1.100:3389 tribf@127.0.0.1 -p 12001";
        let result = parse_ssh_command(cmd).unwrap();
        assert_eq!(result.forward_mode, ForwardingMode::Local);
        assert_eq!(result.local_port, 13389);
        assert_eq!(result.remote_host, "192.168.1.100");
        assert_eq!(result.remote_port, 3389);
        assert_eq!(result.ssh_user, "tribf");
        assert_eq!(result.ssh_host, "127.0.0.1");
        assert_eq!(result.ssh_port, 12001);
    }

    #[test]
    fn test_parse_dynamic_forward() {
        let cmd = "ssh -D 1080 user@proxy.example.com";
        let result = parse_ssh_command(cmd).unwrap();
        assert_eq!(result.forward_mode, ForwardingMode::Dynamic);
        assert_eq!(result.local_port, 1080);
        assert_eq!(result.ssh_user, "user");
        assert_eq!(result.ssh_host, "proxy.example.com");
    }

    #[test]
    fn test_roundtrip() {
        let cmd = "ssh -fNg -o ServerAliveInterval=60 -L 13389:192.168.1.100:3389 tribf@127.0.0.1 -p 12001";
        let config = parse_ssh_command(cmd).unwrap();
        let generated = generate_ssh_command(&config);
        assert!(generated.contains("-L 13389:192.168.1.100:3389"));
        assert!(generated.contains("tribf@127.0.0.1"));
        assert!(generated.contains("-p 12001"));
    }
}
