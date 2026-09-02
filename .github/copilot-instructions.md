# TunnelHub — Project Guidelines

## Overview
TunnelHub is a Tauri 2 desktop SSH tunnel manager (local/remote/dynamic SOCKS5 forwarding) with jump host chaining. UI is in **Simplified Chinese**.

## Tech Stack
- **Frontend**: Vue 3 (`<script setup lang="ts">`), Pinia (setup stores), Naive UI, Vite 6
- **Backend**: Rust (edition 2021), Tauri 2, russh 0.57, Tokio
- **Plugins**: `tauri-plugin-dialog`, `tauri-plugin-autostart`, `tauri-plugin-single-instance` (must be registered first)
- **No router**: Single-view app — `MainView.vue` renders directly, no `<router-view>`

## Architecture

### Frontend → Backend Contract
TypeScript types in [src/types/index.ts](src/types/index.ts) mirror Rust models in [src-tauri/src/models.rs](src-tauri/src/models.rs) exactly. Both use discriminated unions with serde `tag` fields. **Keep these in sync when modifying data structures.**

### Tauri Commands
All 21 commands live in [src-tauri/src/commands.rs](src-tauri/src/commands.rs). Pattern:
```rust
#[tauri::command]
pub async fn cmd(args..., state: State<'_, AppState>, app: tauri::AppHandle) -> Result<T, String>
```
Errors are always `Result<T, String>` — use `.map_err(|e| format!(...))`.

### Frontend Invocation
[src/composables/useTauri.ts](src/composables/useTauri.ts) wraps `invoke()` with typed functions. Args use **camelCase** (Tauri auto-converts to snake_case for Rust).

### Event System
Backend emits `tunnel-status-changed` and `tunnel-log` events via `app.emit()`. Frontend subscribes in `App.vue` on mount, dispatching to Pinia stores.

### State Management
- `tunnelStore` — tunnel CRUD, statuses (Map), logs (Map, capped at 500 entries)
- `groupStore` — group CRUD, tree building, batch start/stop with recursive descendants
- `uiStore` — dark mode (persisted to `localStorage`), drawer/modal visibility flags

All stores use `defineStore('name', () => { ... })` setup function pattern — never Options API.

## Build and Test
```bash
npm run dev                    # Vite dev server (port 1420)
npm run tauri -- dev           # Full Tauri dev (frontend + Rust)
npm run tauri -- build         # Production build
.\scripts\build-windows.ps1   # Windows release: TunnelHub-v{VERSION}-Windows.exe → release-assets/
```
No test suite exists yet. Use `vue-tsc --noEmit` for type checking.

## Code Style

### TypeScript / Vue
- 2-space indent, single quotes
- `<script setup lang="ts">` exclusively
- Naive UI components imported individually: `import { NButton, NIcon } from 'naive-ui'`
- Path alias `@/` → `./src/`
- PascalCase component filenames; `<NButton>` in templates
- `TunnelList.vue` uses `h()` render functions for `NDataTable` columns
- Action column must stay on one row (`flex` + `nowrap`); do not use wrapping `NSpace`
- Minimal scoped CSS, no CSS framework

### Rust
- 4-space indent
- Section comments: `// ─── Section Name ─────`
- All command-facing types derive `Serialize, Deserialize, Debug, Clone`
- Tagged enums: `#[serde(tag = "type", rename_all = "lowercase")]`
- IDs generated via `uuid::Uuid::new_v4().to_string()`

## Project Conventions
- **UI labels are Chinese** — keep all user-facing strings in Simplified Chinese
- **AppState** uses `tokio::sync::Mutex` for async access — always `.lock().await`
- **Tunnel lifecycle**: `CancellationToken` for shutdown, `watch` channel for status broadcast, `tokio::select!` in accept loops
- **Config persistence**: JSON at Tauri `app_data_dir()/tunnelhub_config.json` — save via `config_store::save_config(&app, &config)` after mutations
- **Group hierarchy**: Arbitrary nesting via `parent_id`; batch ops recurse descendants
- **Single instance**: `tauri-plugin-single-instance` is the first plugin; second launch calls `show_main_window` (show / unminimize / focus) instead of starting another process

## Security Notes
- Passwords and key paths stored **plaintext** in config JSON (no encryption yet)
- Server key verification is **disabled** (`check_server_key` always returns `true`)
- CSP is **disabled** (`"csp": null`)
- `AssertSend` wrapper in tunnel_engine.rs uses `unsafe` for russh lifetime workaround — documented as intentional
