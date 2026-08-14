# dsh-studio 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 DeepSeek Harness（`dsh`）打包成 Windows 桌面应用 `dsh-studio`，双击即用，引擎与前端零源码改动，预装 dsh-web-ui 插件。

**Architecture:** Tauri v2（Rust）做外壳，spawn 打包好的 Node 运行时 + `@deepseek-ai/dsh` 作为 sidecar，WebView 导航到 `http://127.0.0.1:<port>`。端口由外壳动态分配；退出时用 `taskkill /T` 杀干净子进程树。

**Tech Stack:** Rust + Tauri v2、Node 24（打包）、`@deepseek-ai/dsh@0.1.0-rc.6`（npm）、`@linxin666/dsh-web-ui-all` + `@linxin666/dsh-skins`（插件）、NSIS 安装包。

## Global Constraints

- 仅 Windows（本机 Win11）。
- 引擎运行时：Node ^22.19 || >=24（随应用打包，用户无需装 Node）。
- 宿主：`@deepseek-ai/dsh@0.1.0-rc.6`，bin 入口 `lib/bin.js`，启动命令 `node <path>/lib/bin.js web --host 127.0.0.1 --port <N>`。
- 插件：`@linxin666/dsh-web-ui-all@0.1.11` + `@linxin666/dsh-skins@0.1.11`（BSD-3-Clause，保留 LICENSE 与作者署名；依赖引用，不搬源码）。
- 只监听 loopback（`127.0.0.1`），不暴露到局域网。
- 不改 `dsh` 或 `dsh-web-ui` 任何源码。
- 应用代码许可 MIT；不实际启用自动更新，仅保留版本号约定与 updater 字段占位。
- 所有 Rust 逻辑先写测试再实现（TDD）。

---

## 文件结构

```
dsh-studio/
├── ui/index.html                 # 静态启动页（splash），Tauri frontendDist
├── scripts/
│   ├── bundle-host.mjs           # 打包脚本：npm 装 dsh + 下载 node.exe + 装插件
│   └── smoke-test.mjs            # 端到端冒烟：启动→宿主就绪→UI 可达
├── src-tauri/
│   ├── Cargo.toml                # 依赖：tauri v2、serde、serde_json
│   ├── build.rs                  # tauri_build::build()
│   ├── tauri.conf.json           # 窗口/安全/打包配置
│   ├── icons/icon.ico            # 应用图标
│   └── src/
│       ├── main.rs               # 调用 lib::run()
│       ├── lib.rs                # Tauri 装配：setup 里起 sidecar、导航窗口
│       ├── port.rs               # 找空闲端口（纯逻辑，单测）
│       └── sidecar.rs            # sidecar spawn/就绪探测/杀进程树（就绪逻辑单测）
└── runtime/                      # 构建产物，不提交 git（见 .gitignore）
    ├── node/node.exe             # 下载的 Node 运行时
    └── host/                     # npm install 的 @deepseek-ai/dsh + node_modules
```

`runtime/` 由 `bundle-host.mjs` 生成，加入 `.gitignore`（已在初始提交中忽略 `node_modules/`、`dist/`、`target/`；需补 `runtime/`）。

---

## Task 1: 脚手架 Tauri v2 项目 + 工具链

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `ui/index.html`
- Modify: `.gitignore`（追加 `runtime/`）

**Interfaces:**
- Produces: 可编译的空 Tauri 应用（`cargo check` 通过），后续任务往里填 `port`/`sidecar` 模块。

- [ ] **Step 1: 安装 tauri-cli**

```bash
cargo install tauri-cli --version "^2"
cargo tauri --version   # 确认输出 2.x
```

- [ ] **Step 2: 写 `src-tauri/Cargo.toml`**

```toml
[package]
name = "dsh-studio"
version = "0.1.0"
description = "DeepSeek Harness desktop app"
edition = "2021"

[lib]
name = "dsh_studio_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 3: 写 `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 4: 写 `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "dsh-studio",
  "version": "0.1.0",
  "identifier": "com.dsh.studio",
  "build": { "frontendDist": "../ui" },
  "app": {
    "windows": [
      { "label": "main", "title": "dsh-studio", "width": 1280, "height": 800 }
    ],
    "security": { "csp": null }
  },
  "bundle": { "active": false }
}
```

- [ ] **Step 5: 写 `ui/index.html`（启动页）**

```html
<!doctype html>
<html><head><meta charset="utf-8"><title>dsh-studio</title>
<style>body{margin:0;height:100vh;display:flex;align-items:center;justify-content:center;font-family:system-ui;background:#0d1117;color:#e6edf3}.spinner{width:24px;height:24px;border:3px solid #30363d;border-top-color:#58a6ff;border-radius:50%;animation:s 1s linear infinite}@keyframes s{to{transform:rotate(360deg)}}</style>
</head><body><div style="text-align:center"><div class="spinner" style="margin:0 auto 12px"></div><div>正在启动 DeepSeek Harness…</div></div></body></html>
```

- [ ] **Step 6: 写 `src-tauri/src/main.rs`（先留空壳）**

```rust
fn main() {
    dsh_studio_lib::run()
}
```

- [ ] **Step 7: 写 `src-tauri/src/lib.rs`（空壳，Task 4 填充）**

```rust
pub fn run() {
    tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {});
}
```

- [ ] **Step 8: 编译验证**

```bash
cd src-tauri && cargo check
```
Expected: 无错误编译通过。

- [ ] **Step 9: 提交**

```bash
git add -A
git commit -m "chore: scaffold tauri v2 project"
```

---

## Task 2: 端口分配（port.rs）

**Files:**
- Create: `src-tauri/src/port.rs`
- Modify: `src-tauri/src/lib.rs`（`mod port;`）

**Interfaces:**
- Produces: `pub fn find_free_port() -> std::io::Result<u16>` —— 绑定 `127.0.0.1:0`，读取 OS 分配端口后立即释放并返回。

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/port.rs` 末尾写：

```rust
#[cfg(test)]
mod tests {
    use super::find_free_port;

    #[test]
    fn returns_port_in_valid_range() {
        let port = find_free_port().expect("should find a free port");
        assert!((1..=65535).contains(&port));
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cd src-tauri && cargo test port::tests
```
Expected: FAIL（`find_free_port` 未定义）。

- [ ] **Step 3: 实现**

```rust
use std::net::TcpListener;

/// 绑定 127.0.0.1:0 让 OS 分配空闲端口，读取后立即释放返回。
/// 释放到 sidecar 绑定时存在极小竞争窗口；若 bind 失败，调用方重试即可。
pub fn find_free_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::find_free_port;

    #[test]
    fn returns_port_in_valid_range() {
        let port = find_free_port().expect("should find a free port");
        assert!((1..=65535).contains(&port));
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cd src-tauri && cargo test port::tests
```
Expected: PASS。

- [ ] **Step 5: 在 `lib.rs` 顶部加 `mod port;` 并提交**

```bash
git add src-tauri/src/port.rs src-tauri/src/lib.rs
git commit -m "feat: free-port allocator"
```

---

## Task 3: sidecar 进程管理（sidecar.rs）

**Files:**
- Create: `src-tauri/src/sidecar.rs`
- Modify: `src-tauri/src/lib.rs`（`mod sidecar;`）

**Interfaces:**
- Produces:
  - `pub struct SidecarHandle { child: Child, port: u16 }`
  - `pub fn spawn(node: &str, dsh_bin: &str, port: u16, dsh_home: &str) -> std::io::Result<SidecarHandle>`
  - `pub fn wait_ready(port: u16, timeout: Duration) -> bool`（TCP 连接探测，供单测）
  - `pub fn kill(self)`（`taskkill /F /T /PID` 杀整个进程树）

- [ ] **Step 1: 写失败测试（就绪探测逻辑）**

```rust
#[cfg(test)]
mod tests {
    use super::wait_ready;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn wait_ready_true_when_server_listens() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                if let Ok((_s, _)) = listener.accept() { break }
            }
        });
        assert!(wait_ready(port, Duration::from_secs(2)));
        handle.join().unwrap();
    }

    #[test]
    fn wait_ready_false_when_nothing_listens() {
        // 拿一个刚释放的端口，不监听。
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        assert!(!wait_ready(port, Duration::from_millis(300)));
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cd src-tauri && cargo test sidecar::tests
```
Expected: FAIL（`wait_ready` 未定义）。

- [ ] **Step 3: 实现**

```rust
use std::net::TcpStream;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

pub struct SidecarHandle {
    child: Child,
    port: u16,
}

/// 启动 node <dsh_bin> web，绑定 loopback，设置 DSH_HOME。
pub fn spawn(node: &str, dsh_bin: &str, port: u16, dsh_home: &str) -> std::io::Result<SidecarHandle> {
    let child = Command::new(node)
        .arg(dsh_bin)
        .arg("web")
        .arg("--host").arg("127.0.0.1")
        .arg("--port").arg(port.to_string())
        .env("DSH_HOME", dsh_home)
        .spawn()?;
    Ok(SidecarHandle { child, port })
}

/// TCP 连接探测：端口开始接受连接即认为就绪。
pub fn wait_ready(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

impl SidecarHandle {
    pub fn port(&self) -> u16 { self.port }

    /// 终止 sidecar 及其整棵子进程树（agent 会 spawn bash/pwsh/工具子进程）。
    pub fn kill(self) {
        let pid = self.child.id();
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .spawn();
        let mut child = self.child;
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::wait_ready;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn wait_ready_true_when_server_listens() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                if let Ok((_s, _)) = listener.accept() { break }
            }
        });
        assert!(wait_ready(port, Duration::from_secs(2)));
        handle.join().unwrap();
    }

    #[test]
    fn wait_ready_false_when_nothing_listens() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        assert!(!wait_ready(port, Duration::from_millis(300)));
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cd src-tauri && cargo test sidecar::tests
```
Expected: PASS。

- [ ] **Step 5: `lib.rs` 加 `mod sidecar;` 并提交**

```bash
git add src-tauri/src/sidecar.rs src-tauri/src/lib.rs
git commit -m "feat: sidecar spawn + readiness probe + tree kill"
```

---

## Task 4: 装配到 Tauri 应用（lib.rs）

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`（无新依赖）

**Interfaces:**
- Consumes: `port::find_free_port`、`sidecar::{spawn, wait_ready, SidecarHandle}`。
- Produces: 完整 `run()` —— setup 阶段解析资源路径、起 sidecar、导航窗口；退出时 kill。

- [ ] **Step 1: 实现 `lib.rs`**

```rust
mod port;
mod sidecar;

use std::sync::Mutex;
use tauri::Manager;

struct SidecarState(Mutex<Option<sidecar::SidecarHandle>>);

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let resource_dir = app.path().resource_dir()?;
            let node = resource_dir.join("runtime/node/node.exe");
            let dsh_bin = resource_dir
                .join("runtime/host/node_modules/@deepseek-ai/dsh/lib/bin.js");
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            let port = port::find_free_port()?;
            let handle = sidecar::spawn(
                node.to_str().ok_or("node path not utf8")?,
                dsh_bin.to_str().ok_or("dsh path not utf8")?,
                port,
                data_dir.to_str().ok_or("data dir not utf8")?,
            )?;

            if !sidecar::wait_ready(port, std::time::Duration::from_secs(30)) {
                return Err("harness 未在 30s 内就绪".into());
            }

            let url = tauri::Url::parse(&format!("http://127.0.0.1:{}/", port))?;
            if let Some(window) = app.get_webview_window("main") {
                window.navigate(&url)?;
            }

            app.manage(SidecarState(Mutex::new(Some(handle))));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<SidecarState>() {
                    if let Some(handle) = state.0.lock().unwrap().take() {
                        handle.kill();
                    }
                }
            }
        });
}
```

- [ ] **Step 2: 编译验证**

```bash
cd src-tauri && cargo check
```
Expected: 通过。若 Tauri v2 API 名有出入（如 `resource_dir()` 返回类型），按编译器提示修正为等价调用。

- [ ] **Step 3: 手动运行验证（需先有 runtime，见 Task 5；本步骤可延后到 Task 5 后执行）**

```bash
cd src-tauri && cargo run
```
Expected: 弹出窗口显示 splash，然后跳到 `http://127.0.0.1:<port>` 加载 dsh UI。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire sidecar into tauri app"
```

---

## Task 5: 打包脚本（bundle-host.mjs）

**Files:**
- Create: `scripts/bundle-host.mjs`
- Modify: `.gitignore`（追加 `runtime/`，若 Task 1 未加）

**Interfaces:**
- Produces: `runtime/node/node.exe`、`runtime/host/node_modules/...`（含 dsh 与已装插件的 profile）。
- Consumes: 无（独立脚本，用 `node scripts/bundle-host.mjs` 运行）。

- [ ] **Step 1: 写脚本**

```js
#!/usr/bin/env node
// 打包 runtime：下载 Node、npm 安装 @deepseek-ai/dsh、安装 dsh-web-ui 插件。
import { execSync } from 'node:child_process'
import { mkdirSync, existsSync, rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')
const runtime = join(root, 'runtime')
const NODE_VERSION = '24.13.0'
const DSH_VERSION = '0.1.0-rc.6'
const PLUGIN_VERSION = '0.1.11'

function run(cmd, cwd) {
  console.log('>', cmd)
  execSync(cmd, { cwd, stdio: 'inherit' })
}

// 1. 下载并解包 Node 运行时（win-x64）
if (!existsSync(join(runtime, 'node', 'node.exe'))) {
  mkdirSync(join(runtime, 'node'), { recursive: true })
  const zip = join(runtime, 'node.zip')
  run(`curl -sSL -o "${zip}" https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-win-x64.zip`)
  run(`unzip -o -q "${zip}" -d "${join(runtime, 'node')}"`)
  // 解包后是 node-v<ver>-win-x64/node.exe，挪到 node/ 根下
  const inner = join(runtime, 'node', `node-v${NODE_VERSION}-win-x64`)
  run(`mv "${join(inner, 'node.exe')}" "${join(runtime, 'node', 'node.exe')}"`)
  rmSync(inner, { recursive: true, force: true })
  rmSync(zip, { force: true })
}

// 2. npm 安装 @deepseek-ai/dsh 到 runtime/host
if (!existsSync(join(runtime, 'host', 'node_modules', '@deepseek-ai', 'dsh'))) {
  mkdirSync(join(runtime, 'host'), { recursive: true })
  run(`npm install --prefix "${join(runtime, 'host')}" "@deepseek-ai/dsh@${DSH_VERSION}"`)
}

// 3. 在受控 DSH_HOME 下安装插件（需要本机 pnpm）
const home = join(runtime, 'home')
const dshBin = join(runtime, 'host', 'node_modules', '.bin', 'dsh')
if (!existsSync(join(home, 'profiles'))) {
  mkdirSync(home, { recursive: true })
  run(`DSH_HOME="${home}" "${dshBin}" plugin --profile web add "@linxin666/dsh-web-ui-all@${PLUGIN_VERSION}" "@linxin666/dsh-skins@${PLUGIN_VERSION}"`)
}

// 4. 校验：dump-config 应包含插件层
run(`DSH_HOME="${home}" "${dshBin}" --profile web --dump-config`)
console.log('bundle-host done')
```

- [ ] **Step 2: 运行脚本**

```bash
node scripts/bundle-host.mjs
```
Expected: 依次完成下载、安装、`dsh plugin add`、`--dump-config` 输出包含 `dsh-web-ui-all` 与 `dsh-skins` 的 layer。

- [ ] **Step 3: 记录实际 profile 布局（写进脚本注释）**

运行后 `ls -la runtime/home`，把 profile 实际落点（例如 `runtime/home/profiles/web/...`）与插件 node_modules 位置记进脚本顶部注释，供 Task 4 的 `data_dir` 首次播种参考。

- [ ] **Step 4: 提交**

```bash
git add scripts/bundle-host.mjs .gitignore
git commit -m "feat: runtime bundling script"
```

> 说明：首次运行播种（把 `runtime/home` 复制到 `%APPDATA%/dsh-studio`）属于 Task 7 冒烟阶段一并验证；若冒烟发现插件未生效，在 Task 4 的 `setup` 里补一段"首次运行复制 runtime/home → data_dir"的逻辑。

---

## Task 6: 安装包与元数据

**Files:**
- Modify: `src-tauri/tauri.conf.json`（`bundle.active=true`、NSIS、图标、updater 占位）
- Create: `src-tauri/icons/icon.ico`（生成，见 Step 1）
- Modify: `src-tauri/Cargo.toml`（加 `tauri-plugin-single-instance`，可选）

**Interfaces:**
- Consumes: 前序任务的 `run()`。
- Produces: `cargo tauri build` 产出 NSIS 安装包。

- [ ] **Step 1: 生成图标**

```bash
cd src-tauri
cargo tauri icon ../../ui/favicon.svg 2>/dev/null || echo "无源图，先用占位：后续用一张 512x512 PNG 作为图标源"
```
Expected: 生成 `src-tauri/icons/*` 全套尺寸。若无源图，先放一张任意 512x512 PNG，命名 `icon.png` 再跑 `cargo tauri icon icon.png`。

- [ ] **Step 2: 更新 `tauri.conf.json` 的 bundle 段**

```json
{
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.ico"],
    "publisher": "dsh-studio",
    "windows": {
      "nsis": {
        "installMode": "currentUser"
      }
    },
    "createUpdaterArtifacts": false
  }
}
```

- [ ] **Step 3: 构建安装包**

```bash
cd src-tauri && cargo tauri build
```
Expected: 产出 `src-tauri/target/release/dsh-studio.exe` 与 `.../bundle/nsis/dsh-studio_0.1.0_x64-setup.exe`。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/tauri.conf.json src-tauri/icons
git commit -m "feat: nsis installer config + icons"
```

---

## Task 7: 端到端冒烟（smoke-test.mjs）

**Files:**
- Create: `scripts/smoke-test.mjs`

**Interfaces:**
- Consumes: `cargo tauri build` 产物 exe + `runtime/`。
- Produces: 可重复的冒烟脚本，验证"双击→宿主就绪→UI 可达→无残留进程"。

- [ ] **Step 1: 写冒烟脚本**

```js
#!/usr/bin/env node
// 冒烟：直接以 sidecar 方式起宿主，验证就绪与 UI 可达（不依赖 GUI 自动化）。
import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')
const node = join(root, 'runtime', 'node', 'node.exe')
const dshBin = join(root, 'runtime', 'host', 'node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js')
const home = join(root, 'runtime', 'home')

const child = spawn(node, [dshBin, 'web', '--host', '127.0.0.1', '--port', '0'], {
  env: { ...process.env, DSH_HOME: home },
  stdio: ['ignore', 'pipe', 'pipe'],
})
let out = ''
child.stdout.on('data', (d) => { out += d; process.stdout.write(d) })
child.stderr.on('data', (d) => { out += d; process.stderr.write(d) })

const deadline = Date.now() + 30_000
const urlMatch = out.match(/http:\/\/127\.0\.0\.1:(\d+)/)
let ok = false
const timer = setInterval(() => {
  const m = out.match(/http:\/\/127\.0\.0\.1:(\d+)/)
  if (m) {
    clearInterval(timer)
    ok = true
    console.log('\nREADY at', m[0])
    child.kill()
    process.exit(0)
  }
  if (Date.now() > deadline) {
    clearInterval(timer)
    console.error('\nTIMEOUT: 未在 30s 内就绪')
    child.kill()
    process.exit(1)
  }
}, 500)
```

- [ ] **Step 2: 运行冒烟**

```bash
node scripts/smoke-test.mjs
```
Expected: 输出 `READY at http://127.0.0.1:<port>`，退出码 0。

- [ ] **Step 3: 验证无残留进程**

```bash
tasklist | grep -i node || echo "无残留 node 进程"
```
Expected: 无残留。

- [ ] **Step 4: 提交**

```bash
git add scripts/smoke-test.mjs
git commit -m "test: end-to-end smoke script"
```

---

## Self-Review 结论

- **Spec 覆盖**：端口策略（Task 2）、sidecar 生命周期/树清理（Task 3）、Node 打包（Task 5）、dsh-web-ui 插件预装（Task 5）、NSIS 安装包（Task 6）、冒烟验证（Task 7）、单实例与 updater 预留（Task 6，`createUpdaterArtifacts:false` + 版本号约定，single-instance 作为可选步骤）。
- **占位符扫描**：无 TBD/TODO；Task 4 Step 3 与 Task 5 Step 3 是显式"运行后按实际结果修正"的步骤，非占位。
- **类型一致性**：`find_free_port` / `spawn(node,dsh_bin,port,dsh_home)` / `wait_ready(port,timeout)` / `SidecarHandle::kill()` 在 Task 2/3/4/7 中命名与签名一致。

## 已知风险（执行时确认）

1. `dsh plugin add` 依赖 pnpm 与 profile 布局细节 —— Task 5 通过"运行后记录实际落点"化解。
2. Tauri v2 的 `resource_dir()` / `navigate()` API 名 —— Task 4 通过 `cargo check` 按编译器提示修正。
3. `runtime/home` 首次播种到用户 AppData —— 在 Task 7 冒烟里验证插件是否生效，必要时在 Task 4 补播种逻辑。
