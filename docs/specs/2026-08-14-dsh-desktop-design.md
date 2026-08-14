# dsh 桌面端设计（deepseek-harness → Tauri 桌面应用）

- 日期：2026-08-14
- 状态：待评审
- 目标：把开源的 DeepSeek Harness（`dsh`）从"Web UI + 命令行启动"打包成 Windows 桌面应用，双击即可使用，可分发给他人。

## 1. 背景与核心发现

DeepSeek Harness（`deepseek-harness`）不是"一个 Web UI"，而是**前后端分离的客户端/服务端架构**：

- **前端（瘦客户端）**：`@deepseek-ai/dsh-web-frontend`，React 18 + Vite 构建，只负责渲染，不执行任何 agent 逻辑。它由 host 同源服务并注入 `window.__DSH_BOOT__` 启动，无法脱离 host 独立运行（`apps/web/vite.config.ts` 明确拒绝 standalone serve）。
- **Host（真正的引擎）**：Node.js 进程，运行 `dsh web`（即 `dsh --profile web`）。包含会话、工具、LLM provider、shell/fs/subprocess、插件系统（Cordis）等全部能力，并通过 HTTP/WebSocket + JSON-RPC 向前端暴露 `/api` 网关。

因此，"复刻成桌面端"的本质不是重写 UI，而是**给现有的 Node 引擎 + React 前端套一个桌面外壳**。这带来零重写的好处：所有 agent 能力原样保留。

关键源码证据：
- web profile 原生支持 `--host 127.0.0.1 --port <N>`，且 `--port 0` 表示由 OS 分配空闲端口（`packages/bundle/web-app/src/startup.ts`）。
- `dsh-host-webserver` 的注释明确提及桌面形态："Electron loads dist over file:// and carries fetch over an IPC bridge"，说明桌面化本来就是被考虑过的 seam。
- `/api` 信任栅栏是 loopback same-origin，WebView 加载 `127.0.0.1` 恰好满足可信场景，无需改动。

## 2. 需求（已确认）

| 维度 | 决策 |
|------|------|
| 用途 | 分发给别人（团队/客户） |
| 目标平台 | 仅 Windows |
| API Key | 用户自带，在设置界面填写，存本机 |
| 更新方式 | 先手动分发安装包，架构上预留自动更新 |
| 技术选型 | Tauri（Rust 外壳）+ Node 引擎 sidecar |

## 3. 方案选型

| 方案 | 说明 | 结论 |
|------|------|------|
| **A. Tauri + Node sidecar** | Rust 外壳 + 打包 Node 引擎 | ✅ 采用（小体积、符合预期、预留更新方便） |
| B. Electron | 主进程复用 Node，开发最快 | ❌ 安装包 150MB+，不符合 Rust 预期 |
| C. 极简壳 | WebView 直接开 127.0.0.1:3080 | ❌ 仍需用户自装 Node、手动跑 `dsh web` |

## 4. 架构

```
用户双击 dsh-desktop.exe
   │
   ▼
┌─────────────────────────────── Tauri 外壳 (Rust) ───────────────────────────────┐
│  · 找一个空闲端口 N（自绑定测试 socket）                                          │
│  · spawn sidecar：node <bundled>/dsh web --host 127.0.0.1 --port N                 │
│  · 轮询 GET http://127.0.0.1:N/ 直到就绪 → 显示加载中 → 打开窗口                    │
│  · WebView 窗口导航到 http://127.0.0.1:N/                                        │
│  · 退出时用 Windows Job Object 杀干净整个子进程树                                  │
└──────────────────────────────────────────────────────────────────────────────────┘
   │  （本地 loopback HTTP，无 CORS、无信任问题）
   ▼
┌─────────────────────────── Node 引擎（原样，零改动）───────────────────────────┐
│  dsh --profile web → webserver + apiproxy + 前端 dist + JSON-RPC/WS 网关         │
│  · agent 引擎（会话/工具/LLM/shell/fs/subprocess）全在这里                        │
│  · 前端同源服务，loopback 信任栅栏天然满足                                         │
│  · 用户 API Key 走现有 Settings → Models 界面 → credentials 存本机                │
└──────────────────────────────────────────────────────────────────────────────────┘
```

**隔离原则**：Tauri 只负责"进程生命周期 + 窗口"，引擎与前端完全不改。改引擎内部不影响 Tauri，反之亦然。

## 5. 组件划分

| 组件 | 职责 | 技术 |
|------|------|------|
| Tauri 壳 | 窗口/生命周期/端口分配/sidecar 管理/单实例/（预留）自动更新 | Rust + Tauri v2 |
| Node 引擎 sidecar | 完整 agent harness（原样，不修改） | 打包的 Node 运行时 + 构建好的 `dsh` web profile |
| 前端 | 默认 React UI + 预装的 dsh-web-ui 插件，由 host 同源服务 | 已有 `@deepseek-ai/dsh-web-frontend` + `dsh plugin` 安装（零源码改动） |
| 打包脚本 | 构建 host → 收集 node_modules → 生成 NSIS 安装包 | CI/脚本 + Tauri bundler |

## 6. 启动时序（数据流）

1. Tauri 启动，自绑定测试 socket，拿到空闲端口 N。
2. spawn sidecar：`node <bundled>/dsh web --host 127.0.0.1 --port N`（`web` 是 `--profile web` 的别名；`<bundled>/dsh` 指打包后的 CLI 入口，其具体路径在实现阶段确定）。
3. 轮询 `http://127.0.0.1:N/` 直到 200（超时则显示带日志的错误页）。
4. 显示 WebView，导航到 `127.0.0.1:N`。
5. 用户首次启动 → Settings → Models 填 API Key（复用现有 UI，存本机）。
6. 用户关闭窗口 → Tauri 触发 Job Object 终止整个子进程树 → 退出。

## 7. 关键技术决策

- **端口**：Tauri 自选空闲端口再显式传给 host（`--port N`）。比固定 3080 抗冲突，比 `--port 0` + 解析 stdout 更稳。
- **Node 打包**：bundle 一个 Node 运行时（`node.exe`）+ 构建好的 host（`pnpm build` 产物 + node_modules），sidecar 直接 `node <bundled>/dsh web`。用户无需装 Node。
- **API Key**：复用现有 Settings UI（`settings.*`/`credentials.*` wire 域），零新增界面；仅需首次启动引导用户去设置填 key。
- **单实例 + 退出清理**：单实例锁 + Windows Job Object，确保 agent 启动的子进程（bash/pwsh/tool）一并被杀，不留孤儿进程。
- **自动更新预留**：在 `tauri.conf.json` 保留 updater 字段占位 + 版本号约定，不实际启用。
- **dsh-web-ui 插件集成**：通过 `dsh plugin --profile web add @linxin666/dsh-web-ui-all`（+ 皮肤 `@linxin666/dsh-skins`）在打包时预装进 web profile，**依赖引用、不搬源码**。它是 BSD-3-Clause，保留 LICENSE 与作者署名。打包脚本在构建时执行插件安装并固化进 profile。

## 8. 错误处理

- host 启动失败（端口被抢、构建产物缺失）→ 显示带日志的错误页，不白屏。
- host 崩溃 → Tauri 捕获 sidecar exit → 弹提示 + 可一键重启。
- 无 API Key → 引擎能起，发消息时报 provider 错，UI 已有引导，无需 Tauri 处理。

## 9. 测试策略

- 打包产物冒烟：安装后启动 → host 就绪 → 能打开 UI、能填 key。
- 生命周期：关闭窗口后无残留 node 子进程（`tasklist` 验证）。
- 端口冲突：占用 3080 后启动仍能自动换端口跑起来。
- 引擎侧：不改，沿用仓库原有测试门禁。

## 10. 范围与非目标

**范围内**：Tauri 外壳、sidecar 生命周期管理、Node 引擎打包、dsh-web-ui 插件预装（`dsh plugin add`，依赖引用不搬源码）、NSIS 安装包、单实例与进程树清理。

**非目标（本期不做）**：自动更新（仅预留接口）、代码签名证书、跨平台（macOS/Linux）、引擎或前端功能改动。
