# Shell Integration Commands

本目录定义“Shell 集成”相关命令与组件，面向应用层终端 Pane 的集成与可观测性，而非 AI 工具内部的适配。

- 职责
  - 管理 Pane 的 Shell 集成状态（开启/关闭/检测）
  - 上报/查询 Pane 的当前命令、历史、CWD、窗口标题等状态
  - 生成 Shell 集成脚本与环境变量（根据 Shell 类型）
  - 与 `mux::TerminalMux`、`ShellIntegrationManager` 协作

- 不属于此处的内容
  - AI 工具为自身执行任务创建/写入/调整终端的命令适配（见 `ai/tool/shell/`）

- 相关文件
  - `commands.rs`：对前端暴露的 Tauri 命令（如 `shell_check_integration_status`、`shell_setup_integration` 等）
  - `integration.rs` / `osc_parser.rs` / `script_generator/`：壳集成细节

- 与 `ai/tool/shell/` 的边界
  - `shell/`：提供“应用层”对现有 Pane 的集成、可观测与运维能力
  - `ai/tool/shell/`：提供“AI 工具”在需要时临时操控终端（创建、写入、调整大小等）的命令适配

目录结构示意：

```text
shell/
├── commands.rs           # Shell 集成相关 Tauri 命令
├── integration.rs        # 集成逻辑
├── integration_test.rs   # 集成测试
├── osc_parser.rs         # OSC 解析
└── script_generator/     # 脚本生成
```
