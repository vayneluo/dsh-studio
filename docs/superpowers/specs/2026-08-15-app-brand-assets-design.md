# DS Studio 应用品牌资产设计

## 目标

让 DS Studio 0.1.2 桌面应用真正使用仓库中的 `ui/logo.png` 与 `ui/brand.png`，而不只是在 README 和落地页展示。

## 设计

- `ui/logo.png` 是 Windows 应用图标的唯一来源。用 Tauri 图标生成器重建 `src-tauri/icons/`，覆盖 EXE、任务栏、桌面快捷方式和 NSIS 安装包使用的 PNG/ICO 资产。
- `ui/brand.png` 用于 DSH Web 左上角品牌按钮。Rust 在编译时包含图片字节，运行时生成 `data:image/png;base64` 地址，再由已有页面注入脚本创建 `<img alt="DS Studio">` 替换上游 SVG。
- 图片在 182×24 像素容器内使用居中裁切，保留原始品牌图形和文字；白色底色作为品牌锁定区，在深浅主题下均保持可读。
- 品牌图片随 EXE 编译，不访问网络、不写临时文件，也不改变 DSH Host 或用户 profile。

## 验证

- Node 测试验证输入品牌资产哈希固定，并确保生成图标不再是旧 `D` 图标。
- Rust 测试验证注入脚本创建图片、包含品牌 data URL、保留 MutationObserver 重应用行为。
- 运行既有脚本门禁与 `cargo test`。
- 构建 v0.1.2 后启动本地可执行程序，人工查看桌面窗口图标与左上角品牌。

