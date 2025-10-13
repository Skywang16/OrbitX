# Rust 后端代码规范

## 设计哲学

好的 Rust 代码应该：**零成本抽象、内存安全、并发安全、类型驱动**

## 核心原则

### 1. 所有权优先,克隆最后

**原则**：默认使用引用传递,只在必要时克隆数据。

#### 所有权规则

- **默认使用引用** - 函数参数优先用 `&T` 而非 `T`
- **避免不必要的克隆** - `.clone()` 应该是有意识的性能决策
- **使用 Copy 语义** - 小类型(≤16字节)实现 `Copy` trait
- **共享所有权用 Arc** - 多个所有者共享不可变数据

#### 示例

```rust
// ❌ 过度克隆
pub fn process_data(&self) -> UserData {
    self.data.clone()  // 每次调用都完整克隆
}

pub fn format_message(&self) -> String {
    self.message.clone()  // 不必要的字符串克隆
}

// ✅ 使用引用
pub fn process_data(&self) -> &UserData {
    &self.data  // 零成本借用
}

pub fn format_message(&self) -> &str {
    &self.message  // 返回字符串切片
}

// ✅ 小类型使用 Copy
#[derive(Copy, Clone)]
pub enum TaskStatus {
    Running,
    Paused,
    Done,
}

pub fn get_status(&self) -> TaskStatus {
    self.status  // Copy 是按位复制,零成本
}
```

#### 何时可以克隆

只在以下情况才使用 `.clone()`:

1. **必须拥有所有权** - 数据需要跨线程传递
2. **修改副本** - 需要修改但不能影响原始数据
3. **性能可接受** - 克隆成本在可接受范围内

```rust
// ✅ 合理的克隆场景
// 1. 跨线程传递(必须拥有所有权)
tokio::spawn(async move {
    let data = shared_data.clone();  // 必须克隆
    process(data).await;
});

// 2. 修改副本
let mut modified = original.clone();
modified.update();

// 3. 明确标记为快照
pub fn snapshot(&self) -> TaskState {
    self.state.clone()  // 函数名暗示会克隆
}
```

### 2. Arc 不是万能钥匙

**原则**：过度使用 `Arc<T>` 会掩盖所有权问题,增加运行时开销。

#### Arc 使用场景

只在以下情况使用 `Arc`:

1. **真正的共享所有权** - 多个所有者需要同时持有数据
2. **跨线程共享不可变数据** - 需要 Send/Sync 语义
3. **避免深拷贝** - 数据量大且共享频繁

```rust
// ❌ 过度使用 Arc
pub struct Config {
    database: Arc<String>,      // String 本身就是堆分配,Arc 多余
    max_connections: Arc<u32>,  // u32 应该用 Copy,不需要 Arc
}

// ✅ 合理使用
pub struct Config {
    database: String,           // 直接使用,Clone 成本可接受
    max_connections: u32,       // 小类型直接 Copy
}

// ✅ 真正需要 Arc 的场景
pub struct AppState {
    db: Arc<DatabasePool>,      // 多个服务共享连接池
    cache: Arc<RwLock<Cache>>,  // 跨线程共享可变状态
}
```

#### Arc 嵌套问题

```rust
// ❌ 过度嵌套
pub struct TaskExecutor {
    repositories: Arc<RepositoryManager>,
    llm_registry: Arc<LLMRegistry>,
    tool_registry: Arc<ToolRegistry>,
    // ... 10+ 个 Arc 字段
}

impl Clone for TaskExecutor {
    fn clone(&self) -> Self {
        Self {
            repositories: Arc::clone(&self.repositories),
            llm_registry: Arc::clone(&self.llm_registry),
            // ... 每次克隆 10+ 次原子操作
        }
    }
}

// ✅ 改进:TaskExecutor 本身就是 Arc
pub type TaskExecutor = Arc<TaskExecutorInner>;

pub struct TaskExecutorInner {
    repositories: Arc<RepositoryManager>,
    llm_registry: Arc<LLMRegistry>,
}

// 克隆只需一次原子操作
let executor2 = Arc::clone(&executor);
```

### 3. 锁的粒度要精细

**原则**：锁应该保护最小必要的数据范围,避免锁竞争。

#### 锁使用规范

- **最小化锁范围** - 只锁必要的数据
- **避免跨 await 持锁** - 异步代码中尽快释放锁
- **考虑无锁数据结构** - 使用 `DashMap`、`Arc<[T]>` 等

```rust
// ❌ 锁粒度太粗
pub struct Registry {
    tools: Arc<RwLock<HashMap<String, Tool>>>,
}

impl Registry {
    pub async fn execute(&self, name: &str) -> Result<()> {
        let tools = self.tools.read().await;  // 持锁整个执行过程
        let tool = tools.get(name)?;
        tool.execute().await  // 跨 await 持锁,阻塞其他读取
    }
}

// ✅ 缩小锁范围
impl Registry {
    pub async fn execute(&self, name: &str) -> Result<()> {
        let tool = {
            let tools = self.tools.read().await;
            tools.get(name).cloned()?  // 克隆工具句柄,立即释放锁
        };  // 锁在此释放
        tool.execute().await  // 执行时不持锁
    }
}

// ✅ 使用无锁数据结构
use dashmap::DashMap;

pub struct Registry {
    tools: Arc<DashMap<String, Tool>>,  // 细粒度锁
}

impl Registry {
    pub async fn execute(&self, name: &str) -> Result<()> {
        let tool = self.tools.get(name).cloned()?;  // 只锁单个条目
        tool.execute().await
    }
}
```

#### Arc<RwLock> 过度嵌套

```rust
// ❌ 过度嵌套锁
pub struct TaskContext {
    messages: Arc<RwLock<Vec<Message>>>,
    tool_results: Arc<RwLock<Vec<ToolResult>>>,
    chain: Arc<RwLock<Chain>>,
    conversation: Arc<RwLock<Vec<String>>>,
    // ... 20+ 个 Arc<RwLock>
}

// 访问时需要多次获取锁
let msg = context.messages.read().await.clone();
let results = context.tool_results.read().await.clone();

// ✅ 合并相关状态
pub struct TaskState {
    messages: Vec<Message>,
    tool_results: Vec<ToolResult>,
    chain: Chain,
}

pub struct TaskContext {
    state: Arc<RwLock<TaskState>>,  // 单个锁保护相关状态

    // 不可变数据不需要锁
    task_id: String,
    config: TaskConfig,
}

// 一次锁获取访问多个字段
let state = context.state.read().await;
let msg = &state.messages;
let results = &state.tool_results;
```

### 4. 错误处理要优雅

**原则**：使用类型系统表达错误,避免字符串传递错误信息。

#### 错误类型设计

```rust
// ❌ 使用字符串错误
pub async fn execute(&self) -> Result<(), String> {
    Err("工具未找到".to_string())  // 每次都分配
}

// ✅ 使用枚举错误
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("工具未找到: {0}")]
    NotFound(String),

    #[error("权限不足")]
    PermissionDenied,

    #[error("执行超时")]
    Timeout,
}

pub async fn execute(&self) -> Result<(), ToolError> {
    Err(ToolError::PermissionDenied)  // 零分配
}
```

#### 错误传播链

```rust
// ❌ 吞掉错误上下文
pub async fn process(&self) -> Result<()> {
    self.step1().await.map_err(|_| Error::ProcessFailed)?;
    self.step2().await.map_err(|_| Error::ProcessFailed)?;
    Ok(())
}

// ✅ 保留错误链
pub async fn process(&self) -> Result<()> {
    self.step1().await
        .map_err(|e| Error::Step1Failed(e))?;
    self.step2().await
        .map_err(|e| Error::Step2Failed(e))?;
    Ok(())
}

// ✅ 使用 anyhow 简化
pub async fn process(&self) -> anyhow::Result<()> {
    self.step1().await
        .context("Step 1 failed")?;  // 自动添加上下文
    self.step2().await
        .context("Step 2 failed")?;
    Ok(())
}
```

### 5. 字符串处理要高效

**原则**：避免不必要的字符串分配和拷贝。

#### 字符串类型选择

```rust
// ❌ 过度使用 String
pub struct Config {
    pub name: String,
    pub version: String,
    pub author: String,
}

pub fn get_name(&self) -> String {
    self.name.clone()  // 每次都分配
}

// ✅ 使用 &str 和 Cow
use std::borrow::Cow;

pub struct Config {
    pub name: Cow<'static, str>,  // 可以是静态字符串或拥有的字符串
    pub version: &'static str,     // 静态字符串
}

pub fn get_name(&self) -> &str {
    &self.name  // 零分配
}

// ✅ 使用 Arc<str> 共享不可变字符串
pub struct TaskSummary {
    pub task_id: Arc<str>,  // 共享所有权,克隆时只增加引用计数
    pub status: TaskStatus,
}
```

#### 常量字符串

```rust
// ❌ 重复分配字符串
pub fn error_msg() -> String {
    "工具未找到".to_string()  // 每次调用都分配
}

// ✅ 使用静态字符串
const ERROR_TOOL_NOT_FOUND: &str = "工具未找到";

pub fn error_msg() -> &'static str {
    ERROR_TOOL_NOT_FOUND  // 零分配
}
```

### 6. 异步代码的最佳实践

**原则**：合理使用异步,避免不必要的 `.await`。

#### 异步函数设计

```rust
// ❌ 不必要的异步
pub async fn get_status(&self) -> TaskStatus {
    self.status  // 没有异步操作,不应该是 async
}

// ✅ 同步方法
pub fn get_status(&self) -> TaskStatus {
    self.status
}

// ✅ 真正需要异步的场景
pub async fn load_data(&self) -> Result<Data> {
    self.db.query("SELECT ...").await  // 有 I/O 操作
}
```

#### 并发执行

```rust
// ❌ 串行执行独立任务
let result1 = fetch_data1().await;
let result2 = fetch_data2().await;
let result3 = fetch_data3().await;

// ✅ 并发执行
let (result1, result2, result3) = tokio::join!(
    fetch_data1(),
    fetch_data2(),
    fetch_data3(),
);
```

### 7. 类型设计要精确

**原则**：用类型系统表达业务规则,让编译器帮你检查。

#### 类型安全

```rust
// ❌ 使用原始类型
pub struct User {
    pub id: i64,        // 可能和其他 ID 混淆
    pub age: u32,       // 可能超出合理范围
}

pub fn get_user(id: i64) -> User { ... }

// 错误用法:传错 ID
let order_id: i64 = 123;
get_user(order_id);  // 编译通过,但逻辑错误

// ✅ 使用新类型(newtype)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserId(i64);

#[derive(Debug, Clone, Copy)]
pub struct Age(u8);  // 限制范围 0-255

impl Age {
    pub fn new(value: u8) -> Result<Self, &'static str> {
        if value > 150 {
            Err("年龄不合理")
        } else {
            Ok(Age(value))
        }
    }
}

pub struct User {
    pub id: UserId,
    pub age: Age,
}

pub fn get_user(id: UserId) -> User { ... }

// 编译错误:类型不匹配
let order_id: OrderId = OrderId(123);
get_user(order_id);  // 编译失败!
```

#### 状态机类型

```rust
// ❌ 使用布尔值表示状态
pub struct Task {
    pub is_running: bool,
    pub is_paused: bool,
    pub is_completed: bool,
    // 可能出现非法状态:同时 running 和 paused
}

// ✅ 使用枚举表示互斥状态
#[derive(Debug, Clone, Copy)]
pub enum TaskStatus {
    Created,
    Running,
    Paused,
    Completed,
    Error,
}

pub struct Task {
    pub status: TaskStatus,  // 编译器保证状态互斥
}
```

## 性能优化指南

### 1. 内存分配优化

```rust
// ❌ 频繁分配
let mut vec = Vec::new();
for i in 0..1000 {
    vec.push(i);  // 多次重新分配
}

// ✅ 预分配容量
let mut vec = Vec::with_capacity(1000);
for i in 0..1000 {
    vec.push(i);  // 只分配一次
}
```

### 2. 避免不必要的转换

```rust
// ❌ 重复转换
fn process(path: &Path) {
    let s = path.to_str().unwrap().to_string();  // Path -> &str -> String
    do_something(&s);
}

// ✅ 直接使用
fn process(path: &Path) {
    let s = path.to_str().unwrap();  // Path -> &str
    do_something(s);
}
```

### 3. 使用迭代器

```rust
// ❌ 中间分配
let items: Vec<_> = data.iter()
    .filter(|x| x.is_valid())
    .collect();  // 分配 Vec

let results: Vec<_> = items.iter()
    .map(|x| x.process())
    .collect();  // 再次分配

// ✅ 链式迭代器(零分配)
let results: Vec<_> = data.iter()
    .filter(|x| x.is_valid())
    .map(|x| x.process())
    .collect();  // 只分配一次
```

## 代码组织规范

### 模块结构

```rust
// src/agent/mod.rs
pub mod config;      // 配置相关
pub mod context;     // 上下文管理
pub mod core;        // 核心逻辑
pub mod error;       // 错误定义
pub mod tools;       // 工具系统

// 重导出常用类型
pub use self::context::TaskContext;
pub use self::error::{AgentError, Result};
```

### 可见性控制

```rust
// ❌ 过度暴露
pub struct TaskContext {
    pub messages: Vec<Message>,  // 外部可随意修改
    pub status: TaskStatus,
}

// ✅ 封装内部状态
pub struct TaskContext {
    messages: Vec<Message>,  // 私有字段
    status: TaskStatus,
}

impl TaskContext {
    // 只暴露必要的接口
    pub fn add_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
}
```

## 测试规范

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("test");
        assert_eq!(task.status(), TaskStatus::Created);
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### 集成测试

```rust
// tests/integration_test.rs
use orbitx::agent::TaskExecutor;

#[tokio::test]
async fn test_full_workflow() {
    let executor = TaskExecutor::new(/* ... */);
    let result = executor.execute(/* ... */).await;
    assert!(result.is_ok());
}
```

## Clippy 配置

在 `Cargo.toml` 中启用严格检查:

```toml
[lints.clippy]
# 禁止的 lint
clone_on_copy = "deny"
clone_double_ref = "deny"
unnecessary_to_owned = "deny"

# 警告的 lint
cloned_instead_of_copied = "warn"
needless_pass_by_value = "warn"
large_types_passed_by_value = "warn"
```

## 性能检查清单

开发时检查以下几点:

### 所有权和借用

- [ ] 是否过度使用 `.clone()`?
- [ ] 能否用引用 `&T` 替代 `T`?
- [ ] 小类型是否实现了 `Copy`?

### Arc 和锁

- [ ] 是否真正需要 `Arc`?
- [ ] `Arc` 嵌套是否过深?
- [ ] 锁的粒度是否够细?
- [ ] 是否跨 `.await` 持锁?

### 字符串处理

- [ ] 是否用了 `&str` 而非 `String`?
- [ ] 常量是否用了 `&'static str`?
- [ ] 是否考虑了 `Cow` 或 `Arc<str>`?

### 异步代码

- [ ] 是否不必要地使用了 `async`?
- [ ] 独立任务是否并发执行?

## 反模式总结

| 反模式                | 问题           | 解决方案                       |
| --------------------- | -------------- | ------------------------------ |
| 到处 `.clone()`       | 掩盖所有权问题 | 优先使用引用 `&T`              |
| `Arc<Arc<T>>`         | 过度嵌套       | 简化为 `Arc<T>`                |
| `Arc<RwLock<Arc<T>>>` | 复杂且低效     | 重新设计数据结构               |
| `String` everywhere   | 频繁分配       | 使用 `&str`、`Cow`、`Arc<str>` |
| 粗粒度锁              | 锁竞争         | 使用 `DashMap` 或细粒度锁      |
| 跨 await 持锁         | 阻塞其他任务   | 缩小锁范围                     |
| 字符串错误            | 类型不安全     | 使用枚举错误                   |
| 不必要的 async        | 复杂度增加     | 只在需要时使用                 |

## 工具推荐

### 开发工具

```bash
# 格式化代码
cargo fmt

# 静态检查
cargo clippy -- -W clippy::all

# 运行测试
cargo test

# 性能分析
cargo bench
```

### 依赖库

- `thiserror` - 错误定义
- `anyhow` - 错误传播
- `tokio` - 异步运行时
- `dashmap` - 并发哈希表
- `parking_lot` - 高性能锁

## 总结

好的 Rust 代码应该:

1. **所有权明确** - 默认借用,必要时克隆
2. **零成本抽象** - 利用编译期检查,避免运行时开销
3. **类型安全** - 用类型系统表达业务规则
4. **并发安全** - 合理使用锁和无锁数据结构
5. **性能优先** - 避免不必要的分配和拷贝

记住:**如果代码写得像 GC 语言(到处 clone),就没有发挥 Rust 的优势!**

**最后更新**: 2025-10-05
