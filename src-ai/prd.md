# TunnelHub — SSH 隧道管理器

> **这份文档是应用程序的完整规格说明。任何 AI 模型拿到这份文档，都应该能重新生成前端 Vue 代码和后端 Rust 代码，并最终编译成可运行的桌面程序。**

---

## 1. 项目概述

TunnelHub 是一个跨平台桌面应用，用来管理 SSH 隧道。用户可以：

- 创建、编辑、删除、复制 SSH 隧道配置
- 把隧道归入分组（支持嵌套分组）
- 一键启动/停止单个隧道或整个分组
- 实时看到每条隧道的运行状态和日志
- 通过粘贴一条 ssh 命令来快速导入配置
- 开机自启、最小化到系统托盘

支持三种 SSH 转发模式：
- **本地端口转发**（`-L`）：把本机某个端口的流量，通过 SSH 服务器转发到指定目标
- **远程端口转发**（`-R`）：让 SSH 服务器监听某个端口，把流量转发回本机
- **动态端口转发**（`-D`）：在本机开一个 SOCKS5 代理端口，所有流量通过 SSH 服务器出去

支持多跳（Jump Host）：可以配置多个跳板机，逐级建立 SSH 连接。

---

## 2. 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 桌面框架 | Tauri v2 | `^2.2.0` |
| 前端框架 | Vue 3 (Composition API + `<script setup>`) | `^3.5.13` |
| 前端语言 | TypeScript | `^5.7.3` |
| 前端状态管理 | Pinia | `^2.3.0` |
| 前端构建 | Vite | `^6.0.5` |
| 前端 UI 组件库 | Naive UI | `^2.40.1` |
| 前端图标 | `@vicons/ionicons5` | `^0.12.0` |
| 路由 | vue-router v4 | `^4.5.0` |
| 后端语言 | Rust 2021 edition | stable |
| SSH 协议库 | russh | 纯 Rust 实现，支持客户端、agent |
| 异步运行时 | tokio | full features |
| 序列化 | serde + serde_json | |
| ID 生成 | uuid v1 | features = ["v4"] |
| 时间 | chrono | features = ["serde"] |
| 错误处理 | anyhow | |
| 日志 | log + env_logger 或 tauri 内建日志 | |
| Tauri 插件 | tauri-plugin-dialog, tauri-plugin-autostart, tauri-plugin-fs | `^2.x` |

**Cargo 路径（Windows 开发机）：** `C:\Users\doudou\.cargo\bin\cargo.exe`

---

## 3. 目录结构

```
tunnel_hub/
├── src/                        # Vue 前端
│   ├── main.ts                 # 入口，创建 app、注册 Pinia、Router、Naive UI
│   ├── App.vue                 # 根组件，设置 NConfigProvider（主题/暗色模式）
│   ├── vite-env.d.ts
│   ├── types/                  # TypeScript 类型定义（与 Rust 模型完全对应）
│   │   └── index.ts
│   ├── stores/                 # Pinia stores
│   │   ├── tunnelStore.ts      # 隧道配置 + 运行状态
│   │   ├── groupStore.ts       # 分组树
│   │   └── uiStore.ts          # UI 状态（暗色模式、当前选中分组/隧道等）
│   ├── composables/            # 可复用逻辑
│   │   └── useTunnelEvents.ts  # 监听 Tauri 事件（tunnel-status-changed, tunnel-log）
│   ├── views/
│   │   └── MainView.vue        # 唯一的主页面（左侧分组树 + 右侧隧道列表）
│   └── components/
│       ├── layout/
│       │   ├── AppSidebar.vue  # 左侧分组树面板
│       │   └── AppHeader.vue   # 顶部工具栏（搜索、暗色切换、设置）
│       ├── groups/
│       │   ├── GroupTree.vue   # 递归渲染分组树，支持展开/折叠
│       │   └── GroupFormModal.vue  # 创建/编辑分组的模态框
│       ├── tunnels/
│       │   ├── TunnelList.vue      # 右侧隧道列表（表格或卡片）
│       │   ├── TunnelRow.vue       # 单行隧道：状态灯、名称、端口、操作按钮
│       │   ├── TunnelFormDrawer.vue # 创建/编辑隧道的抽屉组件（主表单）
│       │   └── TunnelImportModal.vue # 通过 SSH 命令导入的模态框
│       └── logs/
│           └── TunnelLogPanel.vue  # 某条隧道的实时日志面板
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             # 程序入口，调用 lib::run()
│   │   ├── lib.rs              # Tauri Builder 配置、系统托盘、插件注册
│   │   ├── models.rs           # 所有数据结构定义
│   │   ├── commands.rs         # 所有 #[tauri::command] 函数 + AppState
│   │   ├── tunnel_engine.rs    # SSH 隧道核心逻辑（russh 实现）
│   │   ├── config_store.rs     # 配置文件读写（JSON）
│   │   └── ssh_command_parser.rs # SSH 命令字符串解析器
│   ├── Cargo.toml
│   ├── tauri.conf.json         # Tauri 配置
│   ├── icons/                  # 应用图标（多尺寸）
│   │   ├── icon.ico            # Windows 图标（必须是真实多尺寸 ICO，不能是纯色）
│   │   ├── icon.png
│   │   └── ...（各平台尺寸）
│   └── capabilities/           # Tauri v2 权限配置
├── src-ai/
│   └── prd.md                  # 本文件
├── package.json
├── vite.config.ts
├── tsconfig.json
└── README.md
```

---

## 4. 数据模型

所有模型在 Rust 里定义（`models.rs`），同时在前端 TypeScript 里有完全对应的类型（`types/index.ts`）。JSON 序列化用 serde，字段名保持 camelCase（serde 用 `rename_all = "camelCase"` 或前端直接用 snake_case 均可，整个项目统一一种）。

### 4.1 认证方式 `AuthConfig`

```
AuthConfig 是一个枚举，有三种：
- Password：密码字符串
- KeyFile：密钥文件路径（string）+ 可选的密钥密码（passphrase, 可为 null）
- Agent：使用系统 SSH Agent（Windows 上连 OpenSSH Agent 命名管道或 Pageant）

JSON 序列化格式：用 tag 字段区分类型
  { "type": "password", "password": "xxx" }
  { "type": "keyfile", "path": "/path/to/key", "passphrase": null }
  { "type": "agent" }

默认值：Agent
```

### 4.2 跳板机 `JumpHost`

```
JumpHost {
  host: string        // 跳板机主机名或 IP
  port: number        // SSH 端口，默认 22
  user: string        // SSH 用户名
  auth: AuthConfig    // 认证方式
}
```

### 4.3 转发模式 `ForwardingMode`

```
ForwardingMode 枚举：
- "local"   → 本地端口转发 (-L)
- "remote"  → 远程端口转发 (-R)
- "dynamic" → 动态端口转发 / SOCKS5 代理 (-D)

默认：local
```

### 4.4 隧道配置 `TunnelConfig`

这是核心数据结构，保存到配置文件，由用户创建和编辑。

```
TunnelConfig {
  id: string              // UUID v4，创建时自动生成
  name: string            // 用户起的名字，显示在列表里
  forward_mode: ForwardingMode  // 转发模式，默认 local

  // 本地端口（-L 和 -D 模式：本机监听端口；-R 模式：本机目标服务端口）
  local_port: number

  // 远程目标（-L 模式：要访问的最终目标地址）
  // -R 模式：SSH 服务器上监听的绑定地址（通常 "0.0.0.0" 或 "127.0.0.1"）
  // -D 模式：不使用这两个字段
  remote_host: string     // 默认 "127.0.0.1"
  remote_port: number     // 默认 22

  // SSH 服务器（最终连接的 SSH 目标，如果有跳板机则是最后一跳）
  ssh_user: string
  ssh_host: string
  ssh_port: number        // 默认 22

  auth: AuthConfig        // 认证方式
  group_id: string | null // 所属分组 ID，null 表示未分组
  jump_hosts: JumpHost[]  // 跳板机列表，按顺序排列（第一个是第一跳）

  auto_reconnect: bool    // 断线后是否自动重连（功能预留，当前版本可不实现重连逻辑）
  extra_args: string | null  // 额外的 SSH 参数字符串（仅用于 SSH 命令显示，不影响 russh 内部实现）
}
```

### 4.5 分组 `Group`

```
Group {
  id: string              // UUID v4
  name: string
  parent_id: string | null  // 父分组 ID，null 表示顶级分组
}
```

分组是无限嵌套的树形结构。前端把 `Vec<Group>` 构建成树来渲染。

### 4.6 隧道状态 `TunnelStatus`

```
TunnelStatus 枚举（运行时状态，不持久化）：
- "stopped"      → 已停止
- "starting"     → 正在连接中
- "running"      → 运行中（连接已建立，正在监听端口）
- "stopping"     → 正在停止中
- "error"        → 出错，附带 message 字段说明错误原因
- "reconnecting" → 正在重连（auto_reconnect 为 true 时）

JSON：{ "status": "error", "message": "连接拒绝" }
```

### 4.7 日志条目 `LogEntry`

```
LogEntry {
  timestamp: number   // Unix 毫秒时间戳
  level: "info" | "warn" | "error" | "debug"
  message: string
}
```

### 4.8 运行时状态 `TunnelState`

```
TunnelState {
  tunnel_id: string
  status: TunnelStatus
  logs: LogEntry[]      // 最多保留最近 500 条
  started_at: number | null    // 开始启动的时间戳（毫秒）
  connected_at: number | null  // 连接成功的时间戳（毫秒）
}
```

### 4.9 应用配置 `AppConfig`（持久化到磁盘）

```
AppConfig {
  tunnels: TunnelConfig[]
  groups: Group[]
}
```

配置文件路径：Tauri 的 `app_data_dir()`，文件名 `config.json`。即：
- Windows：`C:\Users\<用户名>\AppData\Roaming\tunnel-hub\config.json`
- macOS：`~/Library/Application Support/tunnel-hub/config.json`
- Linux：`~/.local/share/tunnel-hub/config.json`

### 4.10 事件（Rust → 前端）

Rust 通过 `app.emit()` 向前端推送两种事件：

```
事件名 "tunnel-status-changed"，负载：
{
  tunnel_id: string
  status: TunnelStatus
  message: string | null
}

事件名 "tunnel-log"，负载：
{
  tunnel_id: string
  entry: LogEntry
}
```

---

## 5. 后端（Rust）

### 5.1 应用状态 `AppState`

全局唯一的 Tauri managed state，包含：
- `config: Mutex<AppConfig>`：当前内存中的所有配置
- `manager: Mutex<TunnelManager>`：所有正在运行的隧道句柄
- `app_handle: Arc<Mutex<Option<tauri::AppHandle>>>`：用于发射事件（实际上 TunnelManager 初始化后直接持有 AppHandle）

### 5.2 所有 Tauri Commands

#### 隧道 CRUD

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_tunnels` | 无 | `Vec<TunnelConfig>` | 返回所有隧道配置 |
| `create_tunnel` | `config: TunnelConfig` | `TunnelConfig` | 新建隧道，自动生成 ID，保存配置文件 |
| `update_tunnel` | `config: TunnelConfig` | `()` | 更新已有隧道（按 id 匹配），保存配置文件 |
| `delete_tunnel` | `id: String` | `()` | 先停止隧道（如果在运行），再删除，保存配置文件 |
| `duplicate_tunnel` | `id: String` | `TunnelConfig` | 复制隧道，新 ID，名字加 " (copy)"，local_port 置为 0 |

#### 隧道控制

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `start_tunnel` | `id: String` | `()` | 启动隧道，在后台 tokio 任务中运行 |
| `stop_tunnel` | `id: String` | `()` | 取消后台任务，等待任务结束 |
| `get_tunnel_status` | `id: String` | `TunnelStatus` | 查询单条隧道状态 |
| `get_tunnel_state` | `id: String` | `TunnelState` | 查询单条隧道完整状态（含日志） |
| `get_all_statuses` | 无 | `Vec<(String, TunnelStatus)>` | 查询所有隧道状态，返回 (id, status) 数组 |
| `get_tunnel_logs` | `id: String` | `Vec<LogEntry>` | 查询某条隧道的历史日志 |

#### 分组 CRUD

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_groups` | 无 | `Vec<Group>` | 返回所有分组 |
| `create_group` | `name: String, parent_id: Option<String>` | `Group` | 新建分组 |
| `update_group` | `group: Group` | `()` | 更新分组（重命名或改父级） |
| `delete_group` | `id: String` | `()` | 递归删除该分组及所有子分组，隧道的 group_id 置 null |
| `assign_tunnel_to_group` | `tunnel_id: String, group_id: Option<String>` | `()` | 把隧道分配到某分组，null 表示移出分组 |

#### 分组批量操作

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `start_group` | `group_id: String` | `()` | 启动该分组及所有子分组内的全部隧道，有失败则返回错误（但继续启动其余的） |
| `stop_group` | `group_id: String` | `()` | 停止该分组及所有子分组内的全部隧道 |

#### 导入/导出

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `export_config_to_file` | `path: String` | `()` | 把当前 AppConfig 序列化为 JSON 写入指定路径 |
| `import_config_from_file` | `path: String` | `AppConfig` | 从指定 JSON 文件读取，完全替换当前配置 |
| `parse_ssh_command` | `command: String` | `TunnelConfig` | 解析 SSH 命令字符串，返回填好的 TunnelConfig |

#### 开机自启

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_autostart_enabled` | 无 | `bool` | 查询开机自启是否已启用 |
| `set_autostart_enabled` | `enabled: bool` | `()` | 启用或禁用开机自启 |

### 5.3 SSH 隧道引擎（`tunnel_engine.rs`）

使用 **russh** 库实现纯 Rust 的 SSH 客户端，不调用系统 `ssh` 命令行工具。

#### TunnelManager

```
TunnelManager {
  handles: HashMap<String, TunnelHandle>   // tunnel_id → 后台任务句柄
  states: HashMap<String, TunnelState>     // tunnel_id → 运行时状态（含日志）
  app_handle: Option<AppHandle>            // 用于向前端发射事件
}
```

**start_tunnel(config)**：
1. 如果该 id 已经处于 Running/Starting 状态，直接返回错误
2. 创建 `CancellationToken` 和 `watch::channel(TunnelStatus::Starting)`
3. 初始化 TunnelState，状态置为 Starting，写入 started_at 时间
4. 向前端发射 `tunnel-status-changed`（Starting）
5. 用 `tokio::spawn` 启动后台任务 `run_tunnel_task`
6. 把 `TunnelHandle` 存入 handles

**stop_tunnel(id)**：
1. 从 handles 取出 TunnelHandle
2. 发射 Stopping 事件
3. 调用 `cancel_token.cancel()`，await 任务结束
4. 更新 state 为 Stopped，发射 Stopped 事件

#### 建立 SSH 连接链（`establish_ssh_chain`）

```
输入：TunnelConfig（含 jump_hosts 列表）
输出：russh::client::Handle<SshHandler>

逻辑：
1. 如果 jump_hosts 为空：
   直接 client::connect(ssh_host, ssh_port) → authenticate()

2. 如果有跳板机（jump_hosts = [J1, J2, ...], target = ssh_host:ssh_port）：
   a. 连接 J1，认证
   b. 通过 J1 的 channel_open_direct_tcpip 打开到 J2 的通道
   c. 用 client::connect_stream(通道流) 连接 J2，认证
   d. 以此类推，最后通过最后一个跳板的通道连接 target，认证
   返回 target 的 session Handle
```

#### 认证（`authenticate`）

```
三种方式：
- Password → session.authenticate_password(user, password)
- KeyFile  → 读取文件 → PrivateKey::from_openssh → 可选解密 passphrase
             → session.authenticate_publickey(user, key_with_hash)
- Agent    → 连接 Windows 命名管道 \\.\pipe\openssh-ssh-agent（失败则连 Pageant）
             → 枚举 agent 中的所有 key，逐个尝试 authenticate_publickey_with
             → Linux/macOS 用 connect_env()
```

#### 三种转发模式的实现

**本地转发（Local，-L local_port:remote_host:remote_port）**：
```
1. 在本机 127.0.0.1:local_port 上 TcpListener::bind
2. 发射 Running 状态
3. 循环 accept：
   - 每接到一个 TCP 连接，tokio::spawn 一个任务
   - 任务内：session.channel_open_direct_tcpip(remote_host, remote_port)
   - 双向 copy：TCP 流 ↔ SSH 通道流（tokio::io::copy_bidirectional）
4. 监听 CancellationToken，收到取消信号时退出循环
```

**远程转发（Remote，-R remote_port:remote_host:local_port）**：
```
1. 调用 session.tcpip_forward(remote_host, remote_port)
   让 SSH 服务器在 remote_host:remote_port 上开始监听
2. 发射 Running 状态
3. 等待 CancellationToken 取消
4. 取消时调用 session.cancel_tcpip_forward()
5. 当 SSH 服务器收到连接时，会回调 SshHandler::server_channel_open_forwarded_tcpip
   Handler 里把 SSH 通道 ↔ 127.0.0.1:local_port 的 TCP 连接 双向 copy
```

**动态转发（Dynamic，-D local_port，SOCKS5 代理）**：
```
1. 在本机 127.0.0.1:local_port 上 TcpListener::bind
2. 发射 Running 状态
3. 循环 accept：
   - 每接到一个 TCP 连接，tokio::spawn 一个 SOCKS5 处理任务
   - SOCKS5 握手流程：
     a. 读客户端问候（版本=5，方法列表）
     b. 回复无需认证（0x05, 0x00）
     c. 读 CONNECT 请求（版本=5，命令=0x01）
     d. 解析目标地址（IPv4/域名/IPv6）和端口
     e. session.channel_open_direct_tcpip(target_host, target_port)
     f. 回复成功（0x05, 0x00, 0x00, 0x01, 0,0,0,0, 0,0）
     g. 双向 copy：客户端 TCP ↔ SSH 通道
4. 监听 CancellationToken
```

### 5.4 SSH 命令解析器（`ssh_command_parser.rs`）

**解析**（`parse_ssh_command(input: &str) -> Result<TunnelConfig, String>`）：

把一条 SSH 命令字符串转成 TunnelConfig。支持：
- `-L [bind:]local_port:remote_host:remote_port` → local 模式
- `-R [bind:]remote_port:remote_host:local_port` → remote 模式
- `-D [bind:]local_port` → dynamic 模式
- `-p port` → ssh_port
- `-i keyfile` → AuthConfig::KeyFile
- `-J user@host:port[,user@host:port,...]` → jump_hosts
- `-o key=value` → 忽略（用于解析 ServerAliveInterval 等）
- `-fNgqTCvnxXA` 等无参数 flag → 忽略
- `user@host` 或 `host` → ssh_user + ssh_host
- flag 可以连写，如 `-fNgL 13389:host:3389`；也支持 flag 后紧跟参数，如 `-p22`

**生成**（`generate_ssh_command(config: &TunnelConfig) -> String`）：

把 TunnelConfig 生成一条用于显示的 SSH 命令，格式固定为：
```
ssh -fNg -o ServerAliveInterval=60 [-J jumps] [-i keyfile] -L/-R/-D <forward> user@host [-p port]
```

示例：
```
ssh -fNg -o ServerAliveInterval=60 -L 13389:192.168.1.100:3389 tribf@127.0.0.1 -p 12001
```

### 5.5 配置存储（`config_store.rs`）

```
load_config(app_handle) → AppConfig
  从 app_data_dir()/config.json 读取 JSON，反序列化
  文件不存在或解析失败则返回 AppConfig::default()（空配置）

save_config(app_handle, config) → Result<(), String>
  序列化为 JSON（pretty print），写入 app_data_dir()/config.json
  目录不存在时自动创建

export_config(path, config) → Result<(), String>
  写入用户指定的路径

import_config(path) → Result<AppConfig, String>
  从用户指定的路径读取并反序列化
```

### 5.6 系统托盘与生命周期（`lib.rs`）

```
Tauri 启动时（setup 回调）：
1. 调用 config_store::load_config，加载配置到 AppState
2. 调用 manager.set_app_handle(handle)，让 TunnelManager 持有 AppHandle 以发射事件
3. 创建系统托盘：
   - 图标：app.default_window_icon()
   - 菜单：[打开主界面] [分割线] [退出]
   - 双击托盘图标 → 显示主窗口并聚焦
   - 点击"打开主界面" → 同上
   - 点击"退出" → app.exit(0)

窗口关闭事件（on_window_event CloseRequested）：
  - 拦截关闭，改为 window.hide()（最小化到托盘）
  - 不退出程序
  - 只有点托盘菜单"退出"才真正退出

开机自启：
  - 使用 tauri-plugin-autostart
  - macOS 用 LaunchAgent
  - Windows/Linux 用各自的系统机制
```

---

## 6. 前端（Vue）

### 6.1 TypeScript 类型（`types/index.ts`）

完全对应 Rust 的数据模型，所有类型用 TypeScript interface 定义：

```typescript
// 认证方式
type AuthConfig =
  | { type: 'password'; password: string }
  | { type: 'keyfile'; path: string; passphrase: string | null }
  | { type: 'agent' }

// 跳板机
interface JumpHost {
  host: string
  port: number
  user: string
  auth: AuthConfig
}

// 转发模式
type ForwardingMode = 'local' | 'remote' | 'dynamic'

// 隧道配置
interface TunnelConfig {
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

// 分组
interface Group {
  id: string
  name: string
  parent_id: string | null
}

// 运行时状态
type TunnelStatus =
  | { status: 'stopped' }
  | { status: 'starting' }
  | { status: 'running' }
  | { status: 'stopping' }
  | { status: 'error'; message: string }
  | { status: 'reconnecting' }

interface LogEntry {
  timestamp: number
  level: 'info' | 'warn' | 'error' | 'debug'
  message: string
}

interface TunnelState {
  tunnel_id: string
  status: TunnelStatus
  logs: LogEntry[]
  started_at: number | null
  connected_at: number | null
}

interface AppConfig {
  tunnels: TunnelConfig[]
  groups: Group[]
}

// 事件负载
interface TunnelStatusEvent {
  tunnel_id: string
  status: TunnelStatus
  message: string | null
}

interface TunnelLogEvent {
  tunnel_id: string
  entry: LogEntry
}
```

### 6.2 Pinia Stores

#### `tunnelStore.ts`

```
state：
  tunnels: TunnelConfig[]           // 所有隧道配置
  statuses: Record<string, TunnelStatus>  // id → 当前状态
  logs: Record<string, LogEntry[]>  // id → 日志数组（前端缓存最新 500 条）

actions：
  loadTunnels()           → invoke('list_tunnels')，更新 tunnels
  createTunnel(config)    → invoke('create_tunnel')，push 到 tunnels
  updateTunnel(config)    → invoke('update_tunnel')，替换 tunnels 中的对应项
  deleteTunnel(id)        → invoke('delete_tunnel')，从 tunnels 移除
  duplicateTunnel(id)     → invoke('duplicate_tunnel')，push 新配置到 tunnels

  startTunnel(id)         → invoke('start_tunnel')
  stopTunnel(id)          → invoke('stop_tunnel')

  loadAllStatuses()       → invoke('get_all_statuses')，批量更新 statuses
  loadTunnelLogs(id)      → invoke('get_tunnel_logs')，更新 logs[id]

  // 接收来自 Tauri 事件的更新（由 useTunnelEvents composable 调用）
  onStatusEvent(event: TunnelStatusEvent) → 更新 statuses[tunnel_id]
  onLogEvent(event: TunnelLogEvent)       → 追加到 logs[tunnel_id]

getters：
  getStatus(id) → statuses[id] 或 { status: 'stopped' }
  getLogs(id)   → logs[id] 或 []
  getStatusText(id) → 中文状态文字（"运行中"/"已停止"/"连接中"/"出错"等）
  getStatusColor(id) → Naive UI 颜色 token（"success"/"error"/"warning"/"default"）
```

#### `groupStore.ts`

```
state：
  groups: Group[]

actions：
  loadGroups()                      → invoke('list_groups')
  createGroup(name, parent_id?)     → invoke('create_group')
  updateGroup(group)                → invoke('update_group')
  deleteGroup(id)                   → invoke('delete_group')
  assignTunnelToGroup(tunnelId, groupId?) → invoke('assign_tunnel_to_group')
  startGroup(groupId)               → invoke('start_group')
  stopGroup(groupId)                → invoke('stop_group')

getters：
  tree → 把 groups 数组转换成树形结构 TreeNode[]，供 Naive UI NTree 使用
  每个 TreeNode：{ key: group.id, label: group.name, children: [...] }
  根节点之上加一个虚拟的"全部隧道"节点（key: '__all__'）

  tunnelCountForGroup(groupId, tunnels) → 递归统计该分组及所有子分组内的隧道数量
```

#### `uiStore.ts`

```
state：
  darkMode: boolean           // 暗色模式，持久化到 localStorage
  selectedGroupId: string | null  // 当前选中的分组（null 表示"全部"）
  selectedTunnelId: string | null // 当前选中的隧道（用于显示日志面板）

actions：
  toggleDarkMode()
  selectGroup(id?)
  selectTunnel(id?)
```

### 6.3 Composable：`useTunnelEvents.ts`

在 App.vue 或 MainView.vue 的 `onMounted` 里调用一次，注册 Tauri 事件监听器。

```typescript
import { listen } from '@tauri-apps/api/event'

export function useTunnelEvents() {
  onMounted(async () => {
    // 监听状态变化
    await listen<TunnelStatusEvent>('tunnel-status-changed', ({ payload }) => {
      tunnelStore.onStatusEvent(payload)
    })
    // 监听日志
    await listen<TunnelLogEvent>('tunnel-log', ({ payload }) => {
      tunnelStore.onLogEvent(payload)
    })
  })
}
```

### 6.4 UI 布局

主窗口（唯一路由 `/`，对应 `MainView.vue`）：

```
┌─────────────────────────────────────────────────────────────────┐
│  AppHeader（顶部栏）                                             │
│  左：应用名称 "TunnelHub"                                        │
│  右：搜索框、暗色模式切换按钮、设置按钮（开机自启等）             │
├──────────────────┬──────────────────────────────────────────────┤
│  AppSidebar      │  TunnelList（右侧主内容区）                   │
│  （左侧，固定宽） │                                              │
│                  │  工具栏行：                                   │
│  分组树          │  [新建隧道] [导入SSH命令] [批量操作▼]         │
│  ├ 全部隧道 (N)  │                                              │
│  ├ 分组A (n)     │  隧道列表（NDataTable 或 NList）             │
│  │ ├ 子分组A1    │  每行：状态灯 | 名称 | 转发模式 | 本地端口   │
│  │ └ 子分组A2    │       | 目标 | 分组 | 操作按钮               │
│  └ 分组B (n)     │                                              │
│                  │  点击某行 → 底部展开日志面板（TunnelLogPanel）│
│  底部：          │                                              │
│  [+ 新建分组]    │                                              │
└──────────────────┴──────────────────────────────────────────────┘
```

**左侧分组树（AppSidebar + GroupTree）**：
- 使用 Naive UI `NTree` 或手写递归组件
- 点击分组 → 右侧只显示该分组（及子分组）内的隧道
- 点击"全部隧道" → 显示所有隧道
- 分组节点右键菜单或 hover 时显示操作按钮：编辑分组、删除分组、启动全部、停止全部
- 分组名旁边显示隧道数量（该分组 + 所有子分组的隧道总数）
- 支持折叠/展开
- "未分组"作为特殊节点，显示没有 group_id 的隧道

**右侧隧道列表（TunnelList + TunnelRow）**：

表格列：
1. 状态指示灯：圆形小图标，颜色：绿色=running，红色=error，灰色=stopped，黄色=starting/stopping/reconnecting
2. 名称
3. 模式标签：`LOCAL` / `REMOTE` / `DYNAMIC`（Naive UI NTag 不同颜色）
4. 本地端口
5. 目标（`remote_host:remote_port`，dynamic 模式显示 SOCKS5）
6. SSH 服务器（`ssh_user@ssh_host:ssh_port`）
7. 操作列（鼠标悬停时显示，hover 有 tooltip）：
   - 启动/停止（根据状态切换，tooltip："启动隧道"/"停止隧道"）
   - 编辑（tooltip："编辑"）
   - 复制（tooltip："复制"）
   - 删除（tooltip："删除"，需确认）

点击某行（非操作按钮区域）→ 展开底部日志面板显示该隧道日志

**TunnelFormDrawer（隧道创建/编辑抽屉）**：

表单字段（从上到下）：

```
[名称]  文本输入框，必填

[转发模式]  单选 Radio Group：本地端口转发 | 远程端口转发 | 动态端口转发(SOCKS5)

--- 根据转发模式动态显示 ---

Local 模式：
  [本地端口]  数字输入，必填
  [远程目标主机]  文本，必填（如 192.168.1.100）
  [远程目标端口]  数字，必填（如 3389）

Remote 模式：
  [服务器监听地址]  文本（如 0.0.0.0 或 127.0.0.1）
  [服务器监听端口]  数字，必填
  [本地目标主机]   文本（通常 127.0.0.1）
  [本地目标端口]   数字，必填

Dynamic 模式：
  [本地 SOCKS5 端口]  数字，必填（如 1080）

--- SSH 服务器 ---
[SSH 用户名]  文本
[SSH 主机]    文本，必填
[SSH 端口]    数字，默认 22

--- 认证方式 ---
[认证方式]  单选：SSH Agent | 密码 | 密钥文件
  → Agent：无额外字段
  → 密码：[密码] 密码输入框
  → 密钥文件：[密钥文件路径] 文本 + [选择文件] 按钮（调用 tauri-plugin-dialog 打开文件选择器）
              [密钥密码(可选)] 密码输入框

--- 跳板机 ---
[跳板机列表]
  每个跳板机显示：主机、端口、用户名、认证方式（同上）
  [+ 添加跳板机] 按钮，[删除] 按钮
  跳板机顺序就是连接顺序（第一个是第一跳）

--- 其他 ---
[自动重连]  Switch 开关
[额外参数]  文本输入（可选，仅用于 SSH 命令显示）
[所属分组]  下拉选择（NSelect，列出所有分组，可选空）

--- 底部只读区域 ---
[等效 SSH 命令]  代码块，显示 generate_ssh_command(config) 的结果，实时更新

--- 按钮 ---
[取消]  [保存]
```

**TunnelImportModal（SSH 命令导入模态框）**：
```
标题：通过 SSH 命令导入

文本域（多行）：粘贴 SSH 命令，如：
  ssh -fNg -o ServerAliveInterval=60 -L 13389:192.168.1.100:3389 tribf@127.0.0.1 -p 12001

[解析] 按钮 → 调用 invoke('parse_ssh_command')
  成功：预览解析结果（名称、模式、端口等），[确认导入] 按钮打开编辑抽屉并填入数据
  失败：显示错误信息（红色）
```

**TunnelLogPanel（日志面板）**：
```
显示在列表底部（或侧边抽屉），显示选中隧道的日志
- 按时间顺序列出 LogEntry
- 不同级别用不同颜色：info=灰白，warn=黄色，error=红色，debug=暗灰
- 自动滚动到最新一条
- [清空] 按钮（仅清前端缓存）
- [关闭] 按钮
```

**GroupFormModal（分组创建/编辑模态框）**：
```
字段：
  [分组名称]  文本，必填
  [父分组]    下拉选择（可选，选择后成为子分组）
[取消] [保存]
```

### 6.5 主题与样式

- 使用 Naive UI 的 `NConfigProvider` 包裹整个应用
- 通过 `darkTheme` 属性切换暗色/亮色主题
- 暗色模式偏好保存到 `localStorage`，下次启动自动恢复
- 默认跟随系统暗色偏好（`prefers-color-scheme: dark`）

### 6.6 初始化流程（App 启动时）

```
1. App.vue onMounted：
   a. groupStore.loadGroups()
   b. tunnelStore.loadTunnels()
   c. tunnelStore.loadAllStatuses()
   d. useTunnelEvents()（注册事件监听器）
```

---

## 7. 打包与发布

### 7.1 Windows

目标产物：
1. **MSI 安装包**（`TunnelHub_x.x.x_x64_en-US.msi`）
2. **Portable 压缩包**（`TunnelHub-Windows-Portable.zip`，包含 `.exe` 文件，解压即用）

参考打包方式：[farion1231/cc-switch](https://github.com/farion1231/cc-switch) 的 GitHub Actions。

`tauri.conf.json` 关键配置：
```json
{
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.ico"],
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    }
  }
}
```

### 7.2 图标要求

`src-tauri/icons/icon.ico` 必须是包含多种尺寸的真实 ICO 文件（16x16, 32x32, 48x48, 256x256），不能是纯色占位文件。

生成方式：用 `gen_icons.py` 脚本，或 `tauri icon` CLI 命令从一张 1024x1024 PNG 自动生成所有平台图标。

### 7.3 构建命令

```bash
# 开发模式
npm run tauri dev

# 生产构建
npm run tauri build

# 仅前端构建
npm run build
```

---

## 8. 已知问题与修复要求

以下是历史迭代中发现的 bug 和新增需求，新生成代码时必须直接做好，不要留下这些问题：

| # | 问题/需求 | 要求 |
|---|-----------|------|
| 1 | 分组展开后隧道列表为空 | 点击分组时，右侧隧道列表必须正确过滤显示该分组内的隧道（含子分组递归） |
| 2 | 操作列按钮没有 tooltip | 所有操作按钮（启动/停止/编辑/复制/删除）鼠标悬停时必须显示 NTooltip 提示文字 |
| 3 | 分组隧道数量统计不对 | 分组节点显示的数量必须包含所有子分组内的隧道，不能只算直接子隧道 |
| 4 | 只支持本地转发 | 必须支持三种模式：Local / Remote / Dynamic(SOCKS5) |
| 5 | icon.ico 是纯色文件 | 必须使用真实多尺寸 ICO 文件，通过 `tauri icon` 或 `gen_icons.py` 生成 |
| 6 | 无系统托盘 | 已实现：托盘图标、右键菜单（打开主界面、退出）、双击打开、关闭窗口最小化到托盘 |
| 7 | 无开机自启 | 已实现：通过 tauri-plugin-autostart，前端设置页面提供开关 |
| 8 | 编辑页无 SSH 命令预览 | 已实现：表单底部实时显示等效 SSH 命令 |
| 9 | 无 SSH 命令导入 | 已实现：通过 TunnelImportModal + parse_ssh_command 命令 |
| 10 | SSH Key 认证无文件选择 | 已实现：使用 tauri-plugin-dialog 的 open() 打开文件选择器 |

---

## 9. 不实现的功能（范围外）

- **SSH known_hosts 验证**：当前 `check_server_key` 直接返回 true，不做主机指纹验证（已有 TODO 注释，留待后续）
- **自动重连逻辑**：`auto_reconnect` 字段保留但不实现实际的重连循环
- **拖拽隧道到分组**：通过下拉选择分配分组即可，不需要 drag-and-drop
- **多语言（i18n）**：全中文界面，不做国际化

---

## 10. 附录：关键数据流示例

**用户点击"启动隧道"的完整流程**：

```
前端 TunnelRow.vue
  → tunnelStore.startTunnel(id)
  → invoke('start_tunnel', { id })
  → Rust commands::start_tunnel()
    → 从 AppConfig 查找 TunnelConfig
    → TunnelManager::start_tunnel(&config)
      → 状态置 Starting
      → 发射 tunnel-status-changed { status: "starting" }
      → tokio::spawn(run_tunnel_task)
        → establish_ssh_chain()  // 建立 SSH 连接（可能经过多跳）
        → 根据 forward_mode 运行对应转发逻辑
        → 发射 tunnel-status-changed { status: "running" }
        → 监听 CancellationToken（stop 时触发）
  ← Ok(())
前端收到 Ok → 无需轮询，状态更新依赖事件
前端监听 tunnel-status-changed → tunnelStore.onStatusEvent() → 更新 statuses[id] → 界面自动反应
```

**SSH 命令导入流程**：

```
用户在 TunnelImportModal 粘贴命令：
  ssh -fNg -o ServerAliveInterval=60 -L 13389:192.168.1.100:3389 tribf@127.0.0.1 -p 12001

→ invoke('parse_ssh_command', { command })
→ Rust ssh_command_parser::parse_ssh_command()
← TunnelConfig {
    forward_mode: Local,
    local_port: 13389,
    remote_host: "192.168.1.100",
    remote_port: 3389,
    ssh_user: "tribf",
    ssh_host: "127.0.0.1",
    ssh_port: 12001,
    auth: Agent,
    ...
  }
→ 打开 TunnelFormDrawer，填入解析结果，用户确认后保存
```
