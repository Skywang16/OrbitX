# TermX - 终端应用后端

这是一个基于 Tauri 框架构建的终端应用后端，使用 Rust 语言开发。采用现代化的 Mux 架构，提供高性能、稳定的终端模拟功能。

## 项目结构

```
src-tauri/src/
├── main.rs          # 应用程序入口点
├── lib.rs           # 主库文件，负责应用初始化
├── commands/        # Tauri 命令模块
│   ├── mod.rs       # 命令模块导出
│   ├── mux_terminal.rs  # 基于 Mux 的终端命令
│   └── window.rs    # 窗口相关命令
├── mux/             # 终端多路复用器核心模块
│   ├── mod.rs       # 模块导出和类型定义
│   ├── terminal_mux.rs  # 核心 Mux 管理器
│   ├── pane.rs      # 面板接口和实现
│   ├── io_handler.rs    # I/O 处理器
│   ├── singleton.rs # 全局单例管理
│   └── types.rs     # 核心数据类型
├── state/           # 应用状态管理（向后兼容）
│   └── mod.rs       # 状态管理
├── terminal/        # 终端核心功能（向后兼容）
│   └── mod.rs       # 终端管理器
└── utils/           # 工具模块
    ├── mod.rs       # 工具模块导出
    ├── error.rs     # 错误处理
    └── logging.rs   # 日志系统
```

## 核心功能

### 终端多路复用器 (Mux)

- **统一管理**: 中心化的终端会话管理
- **事件驱动**: 基于通知系统的松耦合架构
- **高性能 I/O**: 独立线程处理 PTY 读写，支持批处理优化
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

采用 WezTerm 启发的 Mux 架构，提供统一的终端会话管理：

1. **TerminalMux**: 核心多路复用器，管理所有终端面板
2. **Pane**: 面板接口，封装 PTY 操作
3. **IoHandler**: I/O 处理器，负责高效的数据读写
4. **通知系统**: 事件驱动的组件间通信

### 关键特性

- **线程安全**: 使用 `RwLock<HashMap>` 支持并发读取
- **事件驱动**: 基于订阅-发布模式的通知系统
- **批处理优化**: 智能的数据批处理，提升性能
- **资源管理**: 自动的生命周期管理和清理

### 错误处理

统一的错误类型定义，使用 `thiserror` 提供清晰的错误信息。

### 日志系统

基于 `tracing` 的结构化日志，支持不同级别的日志输出，包含详细的操作跟踪。

## 扩展性

### 添加新功能

1. 在 `mux` 模块中扩展核心功能
2. 在 `commands/mux_terminal.rs` 中添加对应的 Tauri 命令
3. 在 `lib.rs` 中注册新的命令处理器

### 添加新的终端功能

1. 在 `Pane` trait 中添加新方法
2. 在 `LocalPane` 中实现具体功能
3. 在 `TerminalMux` 中添加管理方法
4. 通过命令层暴露给前端

### 扩展通知系统

1. 在 `MuxNotification` 枚举中添加新的通知类型
2. 在 `TerminalMux::notification_to_tauri_event` 中添加转换逻辑
3. 前端监听对应的事件

### 错误处理扩展

在 `mux/error.rs` 中的 `MuxError` 和 `PaneError` 枚举添加新的错误类型。
