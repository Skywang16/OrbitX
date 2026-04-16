# Storage 模块架构文档

## 📁 文件结构

```
storage/
├── mod.rs              # 模块入口，统一导出
├── cache.rs            # 统一内存缓存（带命名空间）
├── database.rs         # SQLite 数据库管理
├── messagepack.rs      # MessagePack 序列化存储
├── paths.rs            # 路径管理
├── error.rs            # 统一错误类型
├── types.rs            # 通用类型定义
├── sql_scripts.rs      # SQL 脚本加载器
└── repositories/       # 数据访问层
    ├── mod.rs          # Repository 模块入口
    ├── ai_models.rs    # AI 模型配置表
    ├── ai_features.rs  # AI 功能配置表
    ├── audit_logs.rs   # 审计日志表
    └── recent_workspaces.rs  # 最近工作区表
```

## 🎯 各模块职责

### 1. UnifiedCache (cache.rs)

**统一内存缓存管理**

- ✅ 命名空间隔离（Rules、Thread、UI、Agent、Completion、Terminal）
- ✅ TTL 支持
- ✅ 自动序列化/反序列化
- ✅ 访问统计
- ✅ 过期清理

**使用示例:**

```rust
use crate::storage::{CacheNamespace, UnifiedCache};

let cache = UnifiedCache::new();

// 带命名空间的 API
cache.set_serialized_ns(CacheNamespace::Rules, "global_rules", &rules).await?;
let rules: Option<String> = cache.get_deserialized_ns(CacheNamespace::Rules, "global_rules").await?;

// 便捷方法
cache.set_global_rules(Some(rules)).await?;
let rules = cache.get_global_rules().await;

// 命名空间管理
cache.clear_namespace(CacheNamespace::Thread).await;
let keys = cache.keys_in_namespace(CacheNamespace::Rules).await;
```

### 2. DatabaseManager (database.rs)

**SQLite 数据库管理**

- ✅ 连接池管理
- ✅ 数据加密（AES-GCM）
- ✅ 密钥管理（KeyVault）
- ✅ 自动迁移

**使用示例:**

```rust
use crate::storage::{DatabaseManager, DatabaseOptions, StoragePaths};

let paths = StoragePaths::new(app_dir)?;
let options = DatabaseOptions::default();
let db = DatabaseManager::new(paths, options).await?;

// 加密/解密
let encrypted = db.encrypt_data("secret").await?;
let decrypted = db.decrypt_data(&encrypted).await?;
```

### 3. MessagePackManager (messagepack.rs)

**MessagePack 序列化存储**

- ✅ 二进制序列化
- ✅ CRC32 校验
- ✅ 自动备份

**使用示例:**

```rust
use crate::storage::{MessagePackManager, SessionState};

let msgpack = MessagePackManager::new(paths, options).await?;
msgpack.save_session_state(&session_state).await?;
let state = msgpack.load_session_state().await?;
```

### 4. Repositories (repositories/)

**数据访问层 - 无抽象，直接 sqlx**

- ✅ 每个表一个简单结构体
- ✅ 借用 &DatabaseManager，避免 Arc 套 Arc
- ✅ 直接使用 sqlx::query，无中间层

**设计原则:**

1. **无 Repository trait** - 避免虚假抽象
2. **无 QueryBuilder** - 直接写 SQL，sqlx 已提供参数绑定
3. **借用优先** - `&DatabaseManager` 而非 `Arc<DatabaseManager>`
4. **简单直接** - 只暴露实际需要的方法

**使用示例:**

```rust
use crate::storage::repositories::{AIModels, RecentWorkspaces};

// 直接构造，传入借用
let ai_models = AIModels::new(&database);
let models = ai_models.find_all().await?;

let workspaces = RecentWorkspaces::new(&database);
workspaces.add_or_update("/path/to/workspace").await?;
```

## 📦 导出清单

### 从 `crate::storage` 可以导入：

**核心管理器:**

```rust
use crate::storage::{
    CacheNamespace,      // 缓存命名空间枚举
    UnifiedCache,        // 统一缓存
    DatabaseManager,     // 数据库管理器
    DatabaseOptions,     // 数据库选项
    MessagePackManager,  // MessagePack 管理器
    MessagePackOptions,  // MessagePack 选项
    StoragePaths,        // 路径管理
    StoragePathsBuilder, // 路径构建器
};
```

**错误类型:**

```rust
use crate::storage::{
    CacheError, CacheResult,
    DatabaseError, DatabaseResult,
    MessagePackError, MessagePackResult,
    RepositoryError, RepositoryResult,
    StorageError, StorageResult,
    StoragePathsError, StoragePathsResult,
    SqlScriptError, SqlScriptResult,
};
```

**通用类型:**

```rust
use crate::storage::{
    SessionState,    // 会话状态
    StorageLayer,    // 存储层枚举
};
```

**数据访问:**

```rust
use crate::storage::repositories::{
    AIModels, AIModelConfig, AIProvider, ModelType,
    AIFeatures,
    AuditLogs,
    RecentWorkspaces, RecentWorkspace,
    Pagination,  // 分页参数
    Ordering,    // 排序参数
};
```

## 🔧 在 setup 中初始化

```rust
// 1. 初始化 DatabaseManager
let database_manager = {
    let paths = StoragePaths::new(app_dir)?;
    let options = DatabaseOptions::default();
    Arc::new(DatabaseManager::new(paths, options).await?)
};
app.manage(database_manager.clone());

// 2. 初始化 MessagePackManager
let messagepack_manager = {
    let paths = StoragePaths::new(app_dir)?;
    Arc::new(MessagePackManager::new(paths, MessagePackOptions::default()).await?)
};
app.manage(messagepack_manager);

// 3. 初始化 UnifiedCache
let cache = Arc::new(UnifiedCache::new());
app.manage(cache);
```

## 🎨 最佳实践

### ✅ 好的做法

1. **使用命名空间避免 key 冲突:**

```rust
cache.set_serialized_ns(CacheNamespace::Agent, "temp_data", &data).await?;
```

2. **Repository 直接构造:**

```rust
let models = AIModels::new(&database).find_all().await?;
```

3. **使用便捷方法:**

```rust
let rules = cache.get_global_rules().await;
```

### ❌ 不好的做法

1. **不要创建多余的包装层:**

```rust
// ❌ 不要这样
struct MyDataAccess {
    database: Arc<DatabaseManager>,
}
impl MyDataAccess {
    fn ai_models(&self) -> AIModels { /* 只是转发 */ }
}

// ✅ 直接用
AIModels::new(&database)
```

2. **不要重复管理状态:**

```rust
// ❌ 不要另外创建 RulesManager
struct RulesManager { global_rules: RwLock<...> }

// ✅ 直接用 cache
cache.set_global_rules(rules).await
```

3. **不要 Arc 套 Arc:**

```rust
// ❌ 不要这样
struct MyStruct {
    db: Arc<Arc<DatabaseManager>>,  // 多余！
}

// ✅ 这样就够了
struct MyStruct {
    db: Arc<DatabaseManager>,
}
```

## 📝 待办事项

- [ ] 替换所有 `RulesManager` 使用为 `UnifiedCache`
- [ ] 确保所有调用方使用新的命名空间 API
- [ ] 添加缓存监控命令（stats、cleanup）
