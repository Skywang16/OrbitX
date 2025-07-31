# TermX 架构文档

## 项目概述

TermX 是一个基于 Tauri 框架的跨平台终端应用，后端使用 Rust 语言开发。采用现代化的 Mux 架构设计，参考了 WezTerm 的设计理念，具有高性能、高稳定性和良好的可扩展性。

## 架构演进历程

### 第一代架构的问题

- **单文件巨石**: 所有代码都在 `lib.rs` 中，超过 300 行
- **功能耦合**: 终端管理、状态管理、错误处理混在一起
- **难以扩展**: 添加新功能需要修改核心文件
- **测试困难**: 功能没有分离，单元测试难以编写

### 第二代架构（事件循环）的问题

- **复杂的事件循环**: 基于 mio 的复杂异步事件处理
- **线程管理复杂**: 多个线程间的协调和同步困难
- **资源泄漏风险**: 复杂的生命周期管理容易出错
- **调试困难**: 异步事件流难以跟踪和调试

### 第三代架构（Mux）的优势

- **中心化管理**: 统一的 Mux 管理所有终端面板
- **事件驱动**: 基于通知系统的松耦合架构
- **高性能 I/O**: 独立线程处理，支持批处理优化
- **线程安全**: 使用 RwLock 和 Arc 确保并发安全
- **易于测试**: 清晰的接口和模块边界

## 架构设计

### 整体架构图

```
┌─────────────────────────────────────────────────────────────┐
│                        前端 (Vue.js)                        │
│                     终端界面 + 用户交互                      │
└─────────────────────┬───────────────────────────────────────┘
                      │ Tauri IPC 通信
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                    Tauri 命令层                             │
│              (commands/ 模块)                               │
├─────────────────────────────────────────────────────────────┤
│  create_terminal  │  write_to_terminal  │  set_always_on_top │
│  resize_terminal  │  close_terminal     │                    │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                   业务逻辑层                                │
├─────────────────────┬───────────────────┬───────────────────┤
│   终端管理模块       │    状态管理模块    │   工具模块        │
│  (terminal/)        │   (state/)        │  (utils/)         │
│                     │                   │                   │
│ • TerminalManager   │ • TerminalState   │ • AppError        │
│ • TerminalOutput    │ • TerminalInfo    │ • init_logging    │
│ • 创建/调整终端      │ • 会话管理        │ • 错误处理        │
└─────────────────────┼───────────────────┼───────────────────┘
                      │                   │
┌─────────────────────▼───────────────────▼───────────────────┐
│                    系统调用层                               │
│                                                             │
│  portable-pty  │  tracing  │  tokio  │  std::sync         │
│  (伪终端)       │  (日志)    │ (异步)   │  (线程同步)         │
└─────────────────────────────────────────────────────────────┘
```

## 模块详细设计

### 1. 主入口模块 (`lib.rs`)

**职责**: 应用程序初始化和配置

```rust
pub fn run() {
    // 1. 初始化日志系统
    // 2. 配置 Tauri 应用
    // 3. 注册状态管理
    // 4. 注册命令处理器
    // 5. 启动应用程序
}
```

**特点**:

- 代码简洁，只负责应用启动
- 模块导入和重新导出
- 统一的应用配置入口

### 2. 命令处理模块 (`commands/`)

**职责**: 处理前端调用的 Tauri 命令

#### 2.1 终端命令 (`commands/terminal.rs`)

- `create_terminal()`: 创建新终端会话
- `write_to_terminal()`: 向终端发送输入
- `resize_terminal()`: 调整终端大小
- `close_terminal()`: 关闭终端会话

#### 2.2 窗口命令 (`commands/window.rs`)

- `set_always_on_top()`: 设置窗口置顶

**设计特点**:

- 每个命令都有详细的文档注释
- 统一的错误处理和日志记录
- 参数验证和状态检查

### 3. 状态管理模块 (`state/`)

**职责**: 管理应用程序状态

```rust
pub struct TerminalState {
    pub terminals: Arc<Mutex<Vec<(u32, TerminalInfo)>>>,
    pub next_id: Arc<Mutex<u32>>,
}
```

**核心方法**:

- `get_next_id()`: 生成唯一终端 ID
- `add_terminal()`: 添加终端会话
- `remove_terminal()`: 移除终端会话
- `find_terminal()`: 查找终端会话

**线程安全**:

- 使用 `Arc<Mutex<>>` 确保多线程安全
- 提供原子操作接口

### 4. 终端管理模块 (`terminal/`)

**职责**: 核心终端功能实现

```rust
pub struct TerminalManager;

impl TerminalManager {
    pub fn create_terminal<R: Runtime>(app: AppHandle<R>, id: u32) -> Result<TerminalInfo, AppError>
    pub fn resize_terminal(terminal_info: &TerminalInfo, rows: u16, cols: u16) -> Result<(), AppError>
}
```

**核心功能**:

- PTY (伪终端) 创建和管理
- Shell 进程启动和配置
- 输入输出线程管理
- 跨平台兼容性处理

**线程模型**:

- 每个终端有独立的读写线程
- 使用 `mpsc` 通道进行线程间通信
- 异步事件发送到前端

### 5. 工具模块 (`utils/`)

**职责**: 通用工具和基础设施

#### 5.1 错误处理 (`utils/error.rs`)

```rust
#[derive(Error, Debug)]
pub enum AppError {
    Terminal(String),
    Window(String),
    Io(#[from] std::io::Error),
    Internal(String),
}
```

#### 5.2 日志系统 (`utils/logging.rs`)

- 基于 `tracing` 的结构化日志
- 环境变量配置支持
- 线程信息包含

## 数据流设计

### 终端创建流程

```
前端调用 create_terminal()
        ↓
commands/terminal.rs 接收请求
        ↓
获取新的终端 ID (TerminalState)
        ↓
调用 TerminalManager::create_terminal()
        ↓
创建 PTY 和 Shell 进程
        ↓
启动读写线程
        ↓
存储终端信息到状态管理器
        ↓
返回终端 ID 给前端
```

### 终端输入输出流程

```
前端输入 → write_to_terminal() → 写入通道 → 写入线程 → PTY → Shell

Shell → PTY → 读取线程 → terminal-output 事件 → 前端显示
```

## 扩展性设计

### 添加新的终端功能

1. **在 `terminal/mod.rs` 中添加新方法**:

```rust
impl TerminalManager {
    pub fn new_feature() -> Result<(), AppError> {
        // 实现新功能
    }
}
```

2. **在 `commands/terminal.rs` 中添加命令**:

```rust
#[tauri::command]
pub fn new_command() -> Result<(), String> {
    TerminalManager::new_feature()
        .map_err(|e| e.to_string())
}
```

3. **在 `lib.rs` 中注册命令**:

```rust
.invoke_handler(tauri::generate_handler![
    // ... 现有命令
    new_command
])
```

### 添加新的模块

1. 创建新的模块目录和文件
2. 在 `lib.rs` 中声明和导入模块
3. 根据需要添加到状态管理或命令处理中

## 性能考虑

### 内存管理

- 使用 `Arc` 进行引用计数，避免数据复制
- 及时清理关闭的终端会话
- 缓冲区大小优化 (4KB)

### 并发处理

- 每个终端独立的读写线程
- 非阻塞的通道通信
- 线程安全的状态管理

### 错误处理

- 统一的错误类型定义
- 详细的错误信息和日志
- 优雅的错误恢复机制

## 安全考虑

### 输入验证

- 终端 ID 验证
- 输入数据长度限制
- 命令参数检查

### 资源管理

- 终端会话数量限制
- 内存使用监控
- 进程生命周期管理

## 测试策略

### 单元测试

- 每个模块独立测试
- 状态管理逻辑测试
- 错误处理测试

### 集成测试

- 终端创建和销毁流程
- 输入输出数据流测试
- 多终端并发测试

### 性能测试

- 大量数据输出性能
- 多终端并发性能
- 内存泄漏检测

## 部署和维护

### 日志配置

```bash
# 开发环境
export RUST_LOG=debug

# 生产环境
export RUST_LOG=info
```

### 监控指标

- 活跃终端会话数
- 内存使用情况
- 错误发生频率
- 响应时间统计

## 总结

重构后的 TermX 项目具有以下优势：

1. **模块化**: 清晰的模块边界，便于维护和扩展
2. **可测试**: 每个模块可以独立测试
3. **可扩展**: 新功能可以轻松添加
4. **可维护**: 代码结构清晰，文档完善
5. **性能优化**: 合理的线程模型和资源管理
6. **错误处理**: 统一的错误处理机制

这个架构为后续的功能扩展和维护提供了坚实的基础。
