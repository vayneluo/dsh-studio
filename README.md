# dsh-studio

`dsh-studio` 是 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) 官方 Web UI 的 Windows 桌面封装。Tauri 负责窗口与进程生命周期，应用内置 Node.js 和 `@deepseek-ai/dsh`，终端用户无需安装 Node 或手动启动服务。

## 当前实现

- 使用官方 DSH Web profile，并为全新安装离线预装 5 个固定插件：`dsh-at-file` 0.6.0、`@liustack/modlens` 3.16.6、`dsh-better-sidebar` 0.12.1、`dshmarket` 1.2.2、`dsh-message-edit` 0.2.1。
- 不包含此前移除的 `@linxin666` 增强 UI、皮肤或 SSH 集成。
- sidecar 仅监听 `127.0.0.1`，端口由桌面壳动态分配。
- 启动页立即显示；后台收到 HTTP 成功响应后再进入 DSH Web。
- host 输出写入应用数据目录的 `dsh-host.log`；启动失败时页面显示日志路径。
- 退出时终止 sidecar 及其子进程树。
- Windows x64 runtime 在构建时剔除调试符号、source map、类型声明和非目标平台原生产物。

## 构建环境

- Windows 11 x64
- Rust stable 与 Tauri CLI 2.x
- Node.js 24、curl 与 PowerShell（仅构建机需要；无需全局 npm/pnpm）

## 开发与验证

```powershell
# 下载/安装并裁剪官方 DSH runtime
node scripts/bundle-host.mjs

# 快速门禁
node --test scripts/default-profile.test.mjs
node --test scripts/runtime-policy.test.mjs
node scripts/smoke-test.mjs
$env:DSH_SMOKE_USE_SEED='1'; node scripts/smoke-test.mjs; Remove-Item Env:DSH_SMOKE_USE_SEED
cargo test --manifest-path src-tauri/Cargo.toml

# 打开开发桌面窗口（Tauri CLI 需在该目录运行）
Push-Location src-tauri
cargo tauri dev
Pop-Location

# 生成 NSIS 安装包
Push-Location src-tauri
cargo tauri build
Pop-Location
```

默认 profile seed、完整 runtime 与 NSIS 安装包的硬门禁分别为 35 MiB、260 MiB、70 MiB。

## 用户数据

凭据、会话、设置和工作区状态保存在 Tauri 应用数据目录。仅当 `profiles/web` 完全不存在时，应用才会在启动 DSH Host 前从 EXE 内置资源原子播种默认 profile；插件无需联网下载，也无需重启即可生效。任何已有 `profiles/web`（包括升级安装，或卸载后保留了应用数据的重装）都会直接跳过，不补装、不覆盖。

由旧版升级时，应用只移除曾经自动预装的增强 UI profile 条目；用户数据与自行安装的其他插件会保留。插件本身可离线启用，但模型服务、市场浏览等在线功能仍取决于网络和相应服务可用性。

## 许可

本项目代码采用 MIT 许可；DeepSeek Harness 按其上游许可分发。
