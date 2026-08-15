# 纯 DSH Web 桌面版设计

- 日期：2026-08-15
- 状态：已确认
- 决策：移除 `@linxin666/dsh-web-ui-all` 及其 profile/runtime 集成，只分发 DeepSeek Harness 官方 Web UI。

## 目标与验收标准

应用继续采用 Tauri + Node sidecar，但桌面包只包含 Windows x64 所需的 Node 与 `@deepseek-ai/dsh` 生产运行时。用户双击后先看到可响应的启动页，host 就绪后进入官方 DSH Web；失败时启动页显示错误与日志路径。

验收门槛：

- runtime 不包含 `runtime/home`、`@linxin666`、PDB、source map 或 TypeScript 声明文件。
- runtime 展开体积不超过 230 MiB；NSIS 安装包不超过 60 MiB。
- 冷启动允许最长 90 秒，但 Tauri 主线程不等待 host，启动页必须立即可见。
- 就绪判断使用 HTTP 2xx/3xx 响应，不以 TCP 端口已监听代替页面可用。
- 关闭应用后 sidecar 进程树退出；后台进程不弹出控制台窗口。
- 开发模式、sidecar 冒烟、Rust 单测、release/NSIS 构建均通过。

## 架构

```text
Tauri 主线程
  ├─ 立即显示本地启动页
  ├─ 定位开发态或安装态 runtime
  ├─ 清理旧版预装增强 UI 的 web profile（保留会话、凭据和用户自装插件）
  ├─ 启动 node.exe + dsh web，并把 stdout/stderr 写入 app data 日志
  └─ 后台线程轮询 HTTP
       ├─ ready -> WebView 导航到 http://127.0.0.1:<port>/
       └─ timeout/error -> 启动页展示错误与日志路径
```

DSH 自己会在空的 `$DSH_HOME` 中生成官方 `web` profile，因此不再构建或播种 `runtime/home`。`$DSH_HOME` 仍使用 Tauri 的 app data 目录，使凭据、设置、会话和工作区状态跨版本保留。

## 组件边界

### `scripts/runtime-policy.mjs`

定义可测试的 Windows x64 runtime 裁剪与审计策略。裁剪仅删除运行时不读取的资产：`*.pdb`、`*.map`、`*.d.ts`/`*.d.mts`/`*.d.cts`，以及 `node-pty` 的非 Windows x64 prebuild 和构建源码目录。许可证文件保留。

### `scripts/bundle-host.mjs`

只下载 Node、安装固定版本的 `@deepseek-ai/dsh`、执行裁剪并运行审计。不安装 pnpm、不安装第三方插件、不生成 `runtime/home`。

### `src-tauri/src/profile.rs`

负责一次性迁移旧版自动预装的 `web` profile。仅当 manifest 中存在已知的 `@linxin666` 依赖时删除这些依赖与 bundle；其他用户依赖与 patch 保留。没有其他外部依赖时才清理旧 profile 的安装缓存，避免误删用户插件。

### `src-tauri/src/sidecar.rs`

负责无控制台启动、日志重定向、HTTP 就绪探测和同步进程树终止。该模块不负责窗口 UI。

### `src-tauri/src/lib.rs`

只做桌面装配：路径解析、状态注册、异步启动任务、成功导航和失败展示。开发态允许回退到仓库根目录的 `runtime`，安装态只使用资源目录。

## 数据与迁移

旧版在 app data 中播种的会话、凭据、设置和 workspace 数据继续保留。迁移只触碰 `profiles/web/package.json` 中明确命名的 `@linxin666/dsh-web-ui-all`、`@linxin666/dsh-skins` 及其旧安装缓存，不清空整个 DSH home。

若用户在同一 profile 中另行安装过其他插件，迁移只停用已知增强 UI，保留其他依赖；可能遗留的共享包缓存不会再被加载，可由后续插件管理操作回收。

## 错误处理

- runtime 缺失：启动页显示缺失路径，进程不 panic。
- sidecar 启动失败：显示系统错误与日志路径。
- 90 秒内没有 HTTP ready：显示超时、日志路径并终止 sidecar。
- sidecar 提前退出：HTTP 探测失败后落入同一错误路径。
- 窗口关闭或应用退出：等待 `taskkill /T /F` 完成，再回收根进程句柄。

## 测试策略

- Node 单测：裁剪规则、第三方包禁入、体积/文件门禁。
- Rust 单测：HTTP ready 正反例、profile manifest 迁移、runtime 路径选择。
- sidecar 冒烟：使用临时空 DSH home 启动官方 Web，断言 HTTP 200 和配置中没有 `@linxin666`。
- 桌面开发态：`cargo tauri dev` 启动真实窗口，验证官方页面可见。
- 发布态：`cargo tauri build`，记录 runtime、主程序和安装包实际体积。

## 非目标

- 不重写 DeepSeek Harness 官方前端或引擎。
- 不保留第三方任务看板、SSH、皮肤、鲸鱼或 Git 图谱集成。
- 不在本轮加入自动更新、代码签名或跨平台包。
