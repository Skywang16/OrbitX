# OrbitX - 终端应用后端

这是一个基于 Tauri 框架构建的终端应用后端，使用 Rust 语言开发。采用现代化的 Mux 架构，提供高性能、稳定的终端模拟功能。

## 项目结构

```text
src-tauri/src/
├── main.rs                        # 应用程序入口
├── lib.rs                         # 应用初始化、插件加载、命令注册入口
├── commands/
│   └── mod.rs                     # 统一注册所有领域命令（invoke_handler 聚合器）
├── setup/
│   └── mod.rs                     # 初始化日志、状态注入、统一事件/深链/启动参数
├── mux/                           # 终端多路复用器核心模块
│   ├── mod.rs
│   ├── terminal_mux.rs            # 核心 Mux 管理器
│   ├── io_handler.rs              # I/O 处理
│   ├── pane.rs                    # Pane 接口与实现
│   ├── performance_monitor.rs     # 性能监控
│   └── singleton.rs               # 全局单例（应用生命周期）
├── terminal/                      # 终端上下文与事件
│   ├── mod.rs
│   ├── commands/                  # 终端上下文相关的 Tauri 命令
│   ├── context_registry.rs
│   ├── context_service.rs
│   ├── event_handler.rs           # 统一将 Mux 事件转为 Tauri 事件
│   ├── channel_manager.rs
│   └── channel_state.rs
├── shell/
│   └── commands.rs                # Shell 集成命令（与 Pane/终端交互）
├── ai/
│   ├── mod.rs
│   ├── commands/                  # AI 相关命令（模型、会话等）
│   ├── service.rs
│   └── tool/
│       └── shell/
│           └── mux_terminal.rs    # AI 工具对终端的适配命令
├── llm/
│   ├── mod.rs
│   ├── commands.rs                # LLM 调用/流式/模型注册等
│   ├── registry.rs
│   ├── service.rs
│   └── providers/
├── filesystem/
│   └── commands.rs                # 文件系统与代码结构解析命令
├── config/
│   ├── mod.rs
│   ├── commands.rs
│   ├── terminal_commands.rs
│   ├── shortcuts/
│   └── theme/
└── utils/
    ├── mod.rs
    ├── api_response.rs
    ├── error.rs
    ├── error_handler.rs
    ├── language.rs
    ├── language_commands.rs
    └── i18n.rs
```

## 核心功能

### 终端多路复用器 (Mux)

- **统一管理**: 中心化的终端会话管理
- **事件驱动**: 基于通知系统的松耦合架构
- **零延迟 I/O**: 每个面板独立读线程，直接流式输出
- **线程安全**: 使用 RwLock 和 Arc 确保并发安全
- **资源管理**: 智能的生命周期管理和资源清理

### 终端操作

- 创建新的终端会话
- 向终端发送输入
- 调整终端大小
- 关闭终端会话
- 实时输出处理

### 窗口管理

- 设置窗口置顶

## 技术栈

- **Tauri**: 跨平台应用框架
- **portable-pty**: 跨平台伪终端实现
- **tokio**: 异步运行时
- **tracing**: 结构化日志
- **serde**: 序列化/反序列化
- **thiserror**: 错误处理
- **crossbeam-channel**: 高性能线程间通信

## 架构设计

### Mux 中心化架构

采用高效的 Mux 架构，提供统一的终端会话管理：

1. **TerminalMux**: 核心多路复用器，管理所有终端面板
2. **Pane**: 面板接口，封装 PTY 操作
3. **IoHandler**: I/O 处理器，负责高效的数据读写
4. **统一事件转发**: 在 `terminal/event_handler.rs` 中订阅 `mux::singleton` 事件并转发为 Tauri 事件

### 关键特性

- **线程安全**: 使用 `RwLock<HashMap>` 支持并发读取
- **事件驱动**: 基于订阅-发布模式的通知系统
- **低延迟输出**: 流式处理终端数据，避免缓冲带来的额外延迟
- **资源管理**: 自动的生命周期管理和清理

### 错误处理

统一的错误类型定义，使用 `thiserror` 提供清晰的错误信息。

### 日志系统

基于 `tracing` 的结构化日志，由 `setup::init_logging()` 初始化；支持不同级别的日志输出，包含详细的操作跟踪。

## 扩展性

### 添加新功能

1. 在 `mux/` 或 `terminal/` 中扩展核心能力（如事件、上下文、I/O）
2. 在对应领域的 `commands` 中添加 Tauri 命令：
   - 终端/壳集成：`shell/commands.rs` 或 `terminal/commands/*`
   - AI 工具对终端的适配：`ai/tool/shell/mux_terminal.rs`
3. 在 `commands/mod.rs` 中统一聚合注册（无需在 `lib.rs` 分散注册）

### 添加新的终端功能

1. 在 `Pane` trait 中添加新方法
2. 在 `LocalPane` 中实现具体功能
3. 在 `TerminalMux` 中添加管理方法
4. 通过命令层暴露给前端

### 扩展通知系统

1. 在 `terminal/event_handler.rs` 中扩展 `TerminalEventHandler` 以订阅新的 Mux 事件
2. 将事件转换/转发为 Tauri 事件
3. 前端监听对应的事件

### 错误处理扩展

在相应领域的错误定义中添加新的错误类型（如 `mux/`、`terminal/` 等）。
