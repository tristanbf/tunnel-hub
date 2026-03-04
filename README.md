# TunnelHub

> SSH 隧道管理器 —— 本地 / 远程 / 动态 SOCKS5 转发，支持跳板机链式连接

TunnelHub 是一款基于 **Tauri 2 + Rust + Vue 3** 构建的跨平台桌面应用，让你在图形界面中轻松管理多条 SSH 隧道，无需手动维护繁琐的命令行参数。

---

## 主要功能

| 功能 | 说明 |
|------|------|
| 本地端口转发 | `localhost:localPort → remoteHost:remotePort` |
| 远程端口转发 | 将远端端口映射回本地 |
| 动态 SOCKS5 代理 | 一键开启本地 SOCKS5 代理 |
| 跳板机链式连接 | 支持多级跳板机（Jump Host）串联 |
| 分组管理 | 任意层级分组，支持批量启动 / 停止 |
| 自动重连 | 隧道断线后自动重新建立连接 |
| 实时日志 | 每条隧道独立日志，最近 500 条滚动展示 |
| 开机自启 | 通过 Tauri autostart 插件实现 |
| 导入 SSH 命令 | 粘贴 `ssh -L / -R / -D` 命令自动解析参数 |

---

## 技术栈

- **前端**：Vue 3 · TypeScript · Naive UI · Pinia · Vite 6
- **后端**：Rust (edition 2021) · Tauri 2 · russh 0.57 · Tokio
- **打包**：MSI 安装包 · NSIS 安装包 · 免安装便携版 ZIP

---

## 截图

> *(截图待补充)*

---

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Tauri CLI 前置依赖](https://tauri.app/start/prerequisites/)（Windows 需要 WebView2）

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
# 仅启动 Vite 前端（无 Rust 后端）
npm run dev

# 完整 Tauri 开发模式（前端 + Rust 后端）
npm run tauri -- dev
```

### 生产构建

```bash
npm run tauri -- build
```

### Windows 发布构建（生成 MSI + 便携 ZIP）

```powershell
.\scripts\build-windows.ps1
```

构建产物位于 `release-assets/` 目录：

```
TunnelHub-v{VERSION}-Windows.msi
TunnelHub-v{VERSION}-Windows-Portable.zip
```

---

## 项目结构

```
tunnel_hub/
├── src/                        # Vue 3 前端
│   ├── components/
│   │   ├── groups/             # 分组表单
│   │   ├── layout/             # 页头、侧边栏
│   │   ├── logs/               # 日志面板
│   │   └── tunnels/            # 隧道列表、表单、状态徽章
│   ├── composables/
│   │   └── useTauri.ts         # Tauri invoke 封装
│   ├── stores/                 # Pinia 状态管理
│   │   ├── tunnelStore.ts
│   │   ├── groupStore.ts
│   │   └── uiStore.ts
│   ├── types/index.ts          # TypeScript 类型定义（与 Rust 模型同步）
│   └── views/MainView.vue      # 主视图
├── src-tauri/
│   └── src/
│       ├── commands.rs         # 全部 Tauri 命令（21 个）
│       ├── models.rs           # Rust 数据模型
│       ├── tunnel_engine.rs    # SSH 隧道核心逻辑
│       ├── config_store.rs     # JSON 配置持久化
│       └── ssh_command_parser.rs  # SSH 命令解析器
├── scripts/
│   └── build-windows.ps1      # Windows 发布脚本
└── conf/
    └── tunnelhub_config.json  # 示例配置文件
```

---

## 认证方式

| 类型 | 说明 |
|------|------|
| 密码 | 直接填写 SSH 密码 |
| 密钥文件 | 指定私钥路径，可选口令 |
| SSH Agent | 使用系统 SSH Agent |

---

## 配置文件

配置自动保存在系统应用数据目录：

- **Windows**：`%APPDATA%\TunnelHub\tunnelhub_config.json`
- **macOS**：`~/Library/Application Support/TunnelHub/tunnelhub_config.json`
- **Linux**：`~/.config/TunnelHub/tunnelhub_config.json`

---

## 安全说明

> **注意**：当前版本有以下已知安全限制，仅建议在受信任的环境中使用：

- 密码和密钥路径以**明文**存储于配置 JSON
- SSH 服务器密钥验证**已禁用**（存在中间人攻击风险）

---

## 开发说明

### 类型同步

`src/types/index.ts` 中的 TypeScript 类型与 `src-tauri/src/models.rs` 中的 Rust 结构体一一对应，修改数据结构时需**同步更新两处**。

### 类型检查

```bash
npx vue-tsc --noEmit
```

### 事件系统

后端通过 `app.emit()` 发出以下事件，前端在 `App.vue` 中订阅：

| 事件名 | 说明 |
|--------|------|
| `tunnel-status-changed` | 隧道状态变更 |
| `tunnel-log` | 隧道日志消息 |

---

## License

MIT
