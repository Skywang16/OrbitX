# AI Tool Shell Adapter

本目录提供“AI 工具”在需要时对终端进行操控的命令与适配层，典型场景包括：创建临时终端、写入命令、调整大小、读取缓冲区等，使 AI 任务能够以受控方式与终端交互。

- 职责
  - 作为 AI 工具的“终端适配器”，暴露 Tauri 命令供前端/AI 任务调用
  - 依赖 `mux::TerminalMux` 和上下文服务，在应用侧的资源模型下安全操作终端
  - 不持久化用户态 Shell 集成状态（该职责在 `shell/` 中）

- 不属于此处的内容
  - 用户 Pane 的 Shell 集成、CWD/历史/标题等“应用态”观测与管理（见 `src-tauri/src/shell/`）

- 相关文件
  - `mux_terminal.rs`：AI 工具终端能力的命令集合（如 `terminal_create`、`terminal_write`、`terminal_resize` 等）
  - `mod.rs`：模块导出

- 与 `shell/` 的边界
  - `ai/tool/shell/`：面向 AI 工具的“操作型”能力，短生命周期、任务驱动
  - `shell/`：面向用户 Pane 的“集成/观测/运维”能力，长生命周期、状态驱动

目录结构示意：

```text
ai/tool/shell/
├── mux_terminal.rs   # 终端适配命令
└── mod.rs            # 模块导出
```
