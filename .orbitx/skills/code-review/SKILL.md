---
name: code-review
description: Perform thorough code reviews for OrbitX project following Linus Torvalds' "good taste" philosophy and project-specific standards for Vue 3 + TypeScript frontend and Rust + Tauri backend.
license: MIT
metadata:
  author: OrbitX Team
  version: '2.0.0'
  category: development
  project: OrbitX
---

# OrbitX Code Review Skill

## When to use this skill

Use this skill when reviewing code for the OrbitX project:

- Pull request reviews
- Architecture design validation
- Security vulnerability checks
- Performance optimization suggestions
- Standards compliance verification

## Review Philosophy

遵循 Linus Torvalds 的"好品味"(Good Taste)原则:

1. **消除特殊情况** - 好代码没有边界情况,只有通用解决方案
2. **数据结构优先** - "糟糕的程序员担心代码,优秀的程序员担心数据结构"
3. **简洁性** - 如果需要超过3层缩进,重新设计它
4. **实用主义** - 解决真实问题,不是假想的威胁
5. **零破坏性** - 向后兼容是神圣不可侵犯的

## OrbitX 架构概览

### 前端 (Vue 3 + TypeScript)

- **状态管理**: Pinia stores (`src/stores/`)
- **组件结构**: `src/components/` - AI聊天、终端集成、主题管理
- **API层**: `src/api/` - 模块化 Tauri 命令接口

### 后端 (Rust + Tauri)

- **Mux核心**: `src-tauri/src/mux/` - 终端多路复用器
- **Domain模块**: terminal, ai, llm, completion, storage, shell, config
- **Agent系统**: AI任务编排与工具执行

## 前端代码审查标准

### 1. 架构设计原则

#### ✅ 前后端协同

- **后端职责**: 数据处理、业务逻辑、顺序保证、时间标记
- **前端职责**: 数据展示、用户交互、状态管理

```typescript
// ❌ Bad: 前端不信任后端
const sortedMessages = apiData.sort((a, b) => a.order - b.order)
const timestamp = Date.now() // 应该用后端时间

// ✅ Good: 信任后端
const messages = apiData // 后端保证顺序
const timestamp = message.createdAt // 使用后端时间戳
```

#### ✅ 状态管理分层

- **全局状态**: Pinia Store (跨组件共享)
- **页面状态**: setup/data (页面级别)
- **组件状态**: ref/reactive (组件内部)

```typescript
// ❌ Bad: 所有状态都放全局
globalStore.isDialogOpen = true
globalStore.tempInputValue = 'hello'

// ✅ Good: 分层明确
const userStore = useUserStore() // 全局
const messages = ref<Message[]>([]) // 页面
const isExpanded = ref(false) // 组件
```

#### ✅ 单一数据源

- 同一份数据只有一个权威来源
- 业务数据(ID、时间戳、排序)由后端提供
- UI状态(动画、临时ID)由前端管理

### 2. 代码风格规范

#### ✅ 函数定义 - 统一使用箭头函数

```typescript
// ❌ Bad: 混合函数定义风格
function handleClick() {}
const handleSubmit = function () {}

// ✅ Good: 统一箭头函数
const handleClick = () => {}
const handleSubmit = async () => {}
```

#### ✅ API 错误处理 - 不要重复处理

```typescript
// ❌ Bad: 重复错误处理
try {
  await workspaceApi.maintainWorkspaces()
} catch (error) {
  console.warn('Failed:', error) // API层已经处理
}

// ✅ Good: 信任API层
workspaceApi.maintainWorkspaces() // 错误已统一处理
```

#### ✅ 导入规范 - 禁止动态导入

```typescript
// ❌ Bad: 动态导入
const { workspaceApi } = await import('@/api/workspace')

// ✅ Good: 静态导入
import { workspaceApi } from '@/api/workspace'
```

### 3. 性能优化检查

- [ ] 是否使用了项目已有的工具函数?(见 `docs/coding-standards/frontend/utility-functions.md`)
- [ ] 大列表是否使用虚拟滚动?
- [ ] 计算属性是否正确使用 `computed()`?
- [ ] 是否避免了不必要的响应式对象?

## 后端代码审查标准

### 1. 所有权和借用

#### ✅ 所有权优先,克隆最后

```rust
// ❌ Bad: 过度克隆
pub fn get_name(&self) -> String {
    self.name.clone() // 每次都分配
}

// ✅ Good: 使用引用
pub fn get_name(&self) -> &str {
    &self.name // 零成本借用
}
```

#### ✅ 检查清单

- [ ] 是否过度使用 `.clone()`?
- [ ] 能否用引用 `&T` 替代 `T`?
- [ ] 小类型(<= 16字节)是否实现了 `Copy`?

### 2. Arc 和锁使用

#### ✅ Arc 不是万能钥匙

```rust
// ❌ Bad: 过度使用 Arc
pub struct Config {
    database: Arc<String>, // String 本身是堆分配
    max_connections: Arc<u32>, // u32 应该 Copy
}

// ✅ Good: 合理使用
pub struct Config {
    database: String, // 直接使用
    max_connections: u32, // Copy 类型
}
```

#### ✅ 锁粒度要精细

```rust
// ❌ Bad: 跨 await 持锁
let tools = self.tools.read().await;
let tool = tools.get(name)?;
tool.execute().await // 持锁整个执行过程

// ✅ Good: 缩小锁范围
let tool = {
    let tools = self.tools.read().await;
    tools.get(name).cloned()?
}; // 锁在此释放
tool.execute().await
```

#### ✅ 检查清单

- [ ] 是否真正需要 `Arc`?
- [ ] `Arc` 嵌套是否过深?
- [ ] 锁的粒度是否够细?
- [ ] 是否跨 `.await` 持锁?

### 3. 错误处理

#### ✅ 使用类型系统表达错误

```rust
// ❌ Bad: 字符串错误
pub async fn execute(&self) -> Result<(), String> {
    Err("工具未找到".to_string()) // 每次都分配
}

// ✅ Good: 枚举错误
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("工具未找到: {0}")]
    NotFound(String),
    #[error("权限不足")]
    PermissionDenied,
}
```

#### ✅ 检查清单

- [ ] 错误类型是否使用 `thiserror::Error`?
- [ ] 是否保留错误传播链?
- [ ] 生产代码中是否避免了 `panic!`?

### 4. 字符串处理

#### ✅ 避免不必要的分配

```rust
// ❌ Bad: 过度使用 String
pub fn get_name(&self) -> String {
    self.name.clone()
}

// ✅ Good: 使用 &str
pub fn get_name(&self) -> &str {
    &self.name
}

// ✅ Good: 使用 Arc<str> 共享
pub struct TaskSummary {
    pub task_id: Arc<str>, // 克隆只增加引用计数
}
```

#### ✅ 检查清单

- [ ] 是否用了 `&str` 而非 `String`?
- [ ] 常量是否用了 `&'static str`?
- [ ] 是否考虑了 `Cow` 或 `Arc<str>`?

### 5. 异步代码

#### ✅ 合理使用异步

```rust
// ❌ Bad: 不必要的异步
pub async fn get_status(&self) -> TaskStatus {
    self.status // 没有异步操作
}

// ✅ Good: 同步方法
pub fn get_status(&self) -> TaskStatus {
    self.status
}
```

#### ✅ 并发执行

```rust
// ❌ Bad: 串行执行
let r1 = fetch_data1().await;
let r2 = fetch_data2().await;

// ✅ Good: 并发执行
let (r1, r2) = tokio::join!(
    fetch_data1(),
    fetch_data2(),
);
```

## 安全检查清单

### 输入验证

- [ ] 用户输入是否被验证?
- [ ] 文件路径是否防止路径遍历(`../`)?
- [ ] 数组/集合是否有大小限制?

### 注入漏洞

- [ ] SQL 是否使用参数化查询?
- [ ] 命令执行是否避免 shell 插值?
- [ ] XSS: 是否使用 `textContent` 而非 `innerHTML`?

### 敏感数据

- [ ] 密码是否被哈希(bcrypt, Argon2)?
- [ ] API密钥是否避免硬编码?
- [ ] 日志中是否避免记录敏感信息?

### Rust 特有

- [ ] `unsafe` 块是否有充分理由?
- [ ] 是否避免了未检查的数组访问?
- [ ] 跨线程数据是否实现了 `Send + Sync`?

## Review 输出格式

```markdown
## Summary

[一行总结变更内容]

## 🟢 Good Taste (优点)

- [设计优雅的地方]
- [消除特殊情况的例子]

## 🔴 Critical Issues (必须修复)

1. **[问题类型]**: [具体问题]
   - 位置: `file.ts:123`
   - 建议: [如何修复]

## 🟡 Suggestions (建议改进)

- [性能优化建议]
- [代码简化建议]

## 📝 Standards Compliance (规范检查)

- [ ] 前端: 箭头函数统一使用
- [ ] 前端: API错误处理正确
- [ ] 后端: 无过度 `.clone()`
- [ ] 后端: 锁粒度合理

## Verdict

[APPROVE / REQUEST CHANGES / COMMENT]
```

## Example Review

```markdown
## Summary

Add user authentication middleware for terminal sessions

## 🟢 Good Taste

- 干净的中间件模式分离了认证逻辑
- 使用 `Result<T, E>` 类型安全的错误处理
- 错误消息清晰且可操作

## 🔴 Critical Issues

1. **Security**: JWT secret 硬编码 (`auth.rs:45`)
   - 应该: 移至环境变量或secret管理系统
   - 风险: 高 - 密钥泄露

2. **Logic Error**: Token 过期检查反了 (`auth.rs:78`)
   - 当前: `if token.exp > now` (错误)
   - 应该: `if token.exp < now`

3. **Arc 过度使用**: `Arc<String>` 可简化 (`middleware.rs:23`)
   - 建议: 改为 `String` 或 `Arc<str>`

## 🟡 Suggestions

- 考虑添加速率限制防止暴力破解
- Token 刷新逻辑可抽取为独立函数(DRY)
- 添加失败认证的结构化日志(`tracing::warn!`)

## 📝 Standards Compliance

- [x] 后端: 使用 `Result<T, ToolError>` 而非 `Result<T, String>`
- [x] 后端: 避免跨 `.await` 持锁
- [ ] 后端: Secret 不应硬编码
- [x] 安全: 使用参数化查询

## Verdict

REQUEST CHANGES - 修复 security 和 logic issues 后可合并
```

## References

参考项目规范文档:

- `docs/coding-standards/backend/rust-best-practices.md`
- `docs/coding-standards/frontend/architecture-design.md`
- `docs/coding-standards/frontend/api-error-handling.md`
- `docs/coding-standards/frontend/function-style.md`
