# 0.1.4 发布检查清单

本清单解决迭代（网页版 / `cargo tauri dev`）与打包 exe 之间的差异问题。

## 每次迭代（dev）

1. `node --test scripts/*.test.mjs`
2. `cargo test --manifest-path src-tauri/Cargo.toml`
3. 改插件或 harness 版本时，同步更新：
   - `scripts/default-profile.mjs`（唯一事实源）
   - `src-tauri/src/profile_seed.rs`（目录一致性测试会拦住遗漏）
   - `runtime/profile-seed/profiles/web/package.json` 与对应 `node_modules`
4. 涉及会话格式/插件写入路径的改动，用 dev 建一个新会话，再用旧版 exe 打开验证跨版本兼容。

## 构建环境

- `bundle-host.mjs` 依赖 pnpm；pnpm 10/11 默认拦截 `node-pty` 构建脚本会导致失败。
  固定 pnpm 9，或给 profile 的 `package.json` 加 `onlyBuiltDependencies: ["node-pty"]`。
- `runtime/host/package.json` 的 `@deepseek-ai/dsh` 必须保持精确版本（不要 `^` 范围），
  避免打包时解析到新 rc。如要锁定传递依赖，把 `runtime/host/package-lock.json` 提交进仓库
  （需在 `.gitignore` 中为它加例外）并让 `bundle-host.mjs` 使用 `npm ci`。

## 发布门禁（按顺序）

1. `node scripts/bundle-host.mjs`
2. `$env:DSH_SMOKE_USE_SEED='1'; node scripts/smoke-test.mjs`
3. `node --test scripts/*.test.mjs` + `node --test landing/tests/*.test.mjs`
4. `cargo test --manifest-path src-tauri/Cargo.toml`
5. `cargo tauri build`
6. 安装新 exe 后：
   - 打开一个 dev 中创建的会话，确认跨读正常；
   - 走查一次 WebView2 渲染；
   - 确认内置插件自动同步（`profiles/web` 的 `.dsh-studio-managed.json` 指纹已更新）。
7. 更新 `README.md` 与 `landing/` 的下载链接，打 tag `v0.1.4` 并发布 release。

## 已知约束

- 会话日志有事件词汇表：旧 exe 无法读取含未知事件的会话。dev 与 exe 必须使用完全相同的
  harness（`0.1.0-rc.6`）与插件版本（`dsh-message-edit 0.2.2`）。
- `dsh-message-edit` 请固定 0.2.2：0.2.1 缺少 `ignorable` 标记导致会话无法加载，
  0.2.3 又移除了该标记。
