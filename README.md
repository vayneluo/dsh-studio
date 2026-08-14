# dsh-studio

DeepSeek Harness 桌面应用：把开源的 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh`）打包成 Windows 桌面端，双击即用。

- **外壳**：Tauri（Rust）——窗口、生命周期、sidecar 管理、单实例、（预留）自动更新。
- **引擎**：打包的 Node 运行时 + `dsh` web profile（原样，零改动）。
- **UI 增强**：通过 `dsh plugin` 预装 [dsh-web-ui](https://github.com/zhu1090093659/dsh-web-ui) 插件与皮肤（任务看板、Git 图谱、右侧面板、SSH 远程、鲸鱼娘、实时 token 统计、皮肤中心）。

## 状态

开发中。设计文档见 `docs/specs/`。

## 许可

本项目代码 MIT；集成的 dsh-web-ui 插件为 BSD-3-Clause（保留原作者署名）。
