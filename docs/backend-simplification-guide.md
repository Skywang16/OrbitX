# 后端模块简化规范文档

## 📋 概述

本文档基于AI模块的成功简化经验，提供了一套标准的后端模块简化规范，旨在减少代码复杂度、提高可维护性，并确保前后端API的一致性。

## 🎯 设计原则

### 1. 最小化原则

- **只保留核心功能**：删除过度设计和不必要的抽象层
- **单一职责**：每个文件和结构体都有明确的单一职责
- **避免过度工程**：优先选择简单直接的解决方案

### 2. 直接调用原则

- **减少中间层**：避免多层嵌套调用，直接调用目标功能
- **简化依赖关系**：避免复杂的依赖注入和工厂模式
- **优先函数调用**：使用简单的函数调用而非复杂的trait系统

### 3. 统一管理原则

- **单一服务入口**：每个模块一个主要的Service结构体
- **一致的API设计**：所有模块遵循相同的API设计模式
- **统一错误处理**：使用一致的错误处理和日志记录方式

## 🏗️ 标准架构模式

### 文件结构

每个后端模块应该严格按照以下结构组织：

```
src/模块名/
├── mod.rs          # 模块导出和重新导出
├── service.rs      # 核心服务层，包含所有业务逻辑
├── commands.rs     # Tauri命令接口，处理前端调用
└── types.rs        # 数据类型定义和序列化
```

### 架构层次

```
Frontend (Vue/React)
    ↓
Commands Layer (Tauri Commands)
    ↓
Service Layer (Business Logic)
    ↓
Storage Layer (Database/File)
```

## 📝 文件职责详解

### 1. `mod.rs` - 模块导出

```rust
/*!
 * 模块名称 - 简要描述
 */

pub mod commands;
pub mod service;
pub mod types;

// 重新导出主要类型和功能
pub use commands::*;
pub use service::*;
pub use types::*;
```

### 2. `service.rs` - 核心服务层

```rust
/*!
 * 模块核心服务 - 统一管理所有功能
 */

use crate::storage::sqlite::SqliteManager;
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// 核心服务结构体
pub struct ModuleService {
    /// 数据存储
    data: RwLock<HashMap<String, DataType>>,
    /// 简单缓存（如需要）
    cache: RwLock<SimpleCache>,
    /// 存储管理器
    storage: Option<Arc<SqliteManager>>,
}

impl ModuleService {
    /// 创建新服务实例
    pub fn new(storage: Option<Arc<SqliteManager>>) -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            cache: RwLock::new(SimpleCache::new()),
            storage,
        }
    }

    /// 初始化服务，从存储加载数据
    pub async fn initialize(&self) -> AppResult<()> {
        if let Some(storage) = &self.storage {
            let items = storage.get_items().await
                .context("从存储加载数据失败")?;

            let mut data = self.data.write().await;
            for item in items {
                data.insert(item.id.clone(), item);
            }
        }
        Ok(())
    }

    /// 获取所有数据
    pub async fn get_items(&self) -> Vec<DataType> {
        let data = self.data.read().await;
        data.values().cloned().collect()
    }

    /// 添加新数据
    pub async fn add_item(&self, item: DataType) -> AppResult<()> {
        let item_id = item.id.clone();

        // 保存到存储
        if let Some(storage) = &self.storage {
            storage.save_item(&item).await
                .context("保存数据失败")?;
        }

        // 更新内存
        let mut data = self.data.write().await;
        data.insert(item_id.clone(), item);

        info!("成功添加数据: {}", item_id);
        Ok(())
    }

    /// 更新数据（支持部分更新）
    pub async fn update_item(&self, id: &str, updates: serde_json::Value) -> AppResult<()> {
        // 获取现有数据
        let updated_item = {
            let data = self.data.read().await;
            let existing_item = data.get(id)
                .ok_or_else(|| anyhow!("数据不存在: {}", id))?;
            existing_item.clone()
        };

        // 应用部分更新
        let mut item_value = serde_json::to_value(&updated_item)
            .context("序列化现有数据失败")?;

        if let serde_json::Value::Object(ref mut item_obj) = item_value {
            if let serde_json::Value::Object(updates_obj) = updates {
                for (key, value) in updates_obj {
                    item_obj.insert(key, value);
                }
            }
        }

        let final_item: DataType = serde_json::from_value(item_value)
            .context("反序列化更新后的数据失败")?;

        // 保存到存储
        if let Some(storage) = &self.storage {
            storage.update_item(&final_item).await
                .context("更新数据失败")?;
        }

        // 更新内存
        let mut data = self.data.write().await;
        data.insert(id.to_string(), final_item);

        info!("成功更新数据: {}", id);
        Ok(())
    }

    /// 删除数据
    pub async fn remove_item(&self, id: &str) -> AppResult<()> {
        // 从存储删除
        if let Some(storage) = &self.storage {
            storage.delete_item(id).await
                .context("删除数据失败")?;
        }

        // 从内存删除
        let mut data = self.data.write().await;
        data.remove(id);

        info!("成功删除数据: {}", id);
        Ok(())
    }
}

/// 简单缓存实现
pub struct SimpleCache {
    entries: HashMap<String, CacheEntry>,
    default_ttl: Duration,
}

impl SimpleCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl: Duration::from_secs(3600),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        if let Some(entry) = self.entries.get(key) {
            if entry.is_expired() {
                self.entries.remove(key);
                None
            } else {
                Some(entry.value.clone())
            }
        } else {
            None
        }
    }

    pub fn put(&mut self, key: String, value: String) {
        let entry = CacheEntry::new(value, self.default_ttl);
        self.entries.insert(key, entry);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: String,
    created_at: std::time::Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn new(value: String, ttl: Duration) -> Self {
        Self {
            value,
            created_at: std::time::Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}
```

### 3. `commands.rs` - Tauri命令接口

```rust
/*!
 * 模块的Tauri命令接口
 */

use crate::模块名::{DataType, ModuleService};
use crate::storage::sqlite::SqliteManager;
use std::sync::Arc;
use tauri::State;
use tracing::info;

/// 模块管理器状态
pub struct ModuleManagerState {
    pub service: Arc<ModuleService>,
}

impl ModuleManagerState {
    /// 创建新的管理器状态
    pub fn new(storage: Option<Arc<SqliteManager>>) -> Result<Self, String> {
        let service = Arc::new(ModuleService::new(storage));
        Ok(Self { service })
    }

    /// 初始化服务
    pub async fn initialize(&self) -> Result<(), String> {
        self.service
            .initialize()
            .await
            .map_err(|e| e.to_string())
    }
}

// ===== CRUD命令 =====

/// 获取所有数据
#[tauri::command]
pub async fn get_items(
    state: State<'_, ModuleManagerState>
) -> Result<Vec<DataType>, String> {
    info!("获取数据列表");

    let items = state.service.get_items().await;

    info!("成功获取 {} 条数据", items.len());
    Ok(items)
}

/// 添加数据
#[tauri::command]
pub async fn add_item(
    item: DataType,
    state: State<'_, ModuleManagerState>,
) -> Result<(), String> {
    info!("添加数据: {}", item.id);

    state
        .service
        .add_item(item)
        .await
        .map_err(|e| e.to_string())
}

/// 更新数据
#[tauri::command]
pub async fn update_item(
    id: String,
    updates: serde_json::Value,
    state: State<'_, ModuleManagerState>,
) -> Result<(), String> {
    info!("更新数据: {}", id);

    state
        .service
        .update_item(&id, updates)
        .await
        .map_err(|e| e.to_string())
}

/// 删除数据
#[tauri::command]
pub async fn remove_item(
    id: String,
    state: State<'_, ModuleManagerState>,
) -> Result<(), String> {
    info!("删除数据: {}", id);

    state
        .service
        .remove_item(&id)
        .await
        .map_err(|e| e.to_string())
}
```

### 4. `types.rs` - 数据类型定义

```rust
/*!
 * 模块数据类型定义
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 核心数据类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataType {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// 请求类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestType {
    pub action: String,
    pub params: HashMap<String, serde_json::Value>,
}

/// 响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseType {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub message: Option<String>,
}
```

## 🔧 实现规范

### 错误处理

```rust
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};

// 统一错误处理模式
result.context("操作描述")
      .map_err(|e| e.to_string())

// 自定义错误
return Err(anyhow!("具体错误信息: {}", param));
```

### 日志记录

```rust
use tracing::{debug, info, warn, error};

info!("操作开始: {}", param);
warn!("警告信息: {}", warning);
error!("错误信息: {}", error);
debug!("调试信息: {}", debug_info);
```

### 并发控制

```rust
use tokio::sync::RwLock;

// 读多写少场景使用RwLock
data: RwLock<HashMap<String, DataType>>

// 读取操作
let data = self.data.read().await;
let item = data.get(id);

// 写入操作
let mut data = self.data.write().await;
data.insert(id, item);
```

### 缓存模式

```rust
// 简单TTL缓存
pub struct SimpleCache {
    entries: HashMap<String, CacheEntry>,
    default_ttl: Duration,
    max_entries: usize,
}

// 缓存键生成
fn cache_key(param1: &str, param2: &str) -> String {
    format!("{}:{}", param1, param2)
}

// 缓存使用
let cache_key = cache_key(&id, &action);
if let Some(cached) = cache.get(&cache_key) {
    return Ok(cached);
}
```

## 🚫 避免的反模式

### 1. 过度抽象

```rust
// ❌ 避免 - 不必要的trait抽象
trait Manager<T> {
    fn process(&self, item: T) -> Result<ProcessedItem, Error>;
}

// ✅ 推荐 - 直接实现
impl Service {
    pub async fn process_item(&self, item: Item) -> AppResult<ProcessedItem> {
        // 直接处理逻辑
    }
}
```

### 2. 复杂依赖注入

```rust
// ❌ 避免 - 复杂的依赖注入
pub struct ComplexManager {
    adapter: Arc<dyn Adapter>,
    processor: Arc<dyn Processor>,
    cache: Arc<dyn Cache>,
    validator: Arc<dyn Validator>,
}

// ✅ 推荐 - 简单直接的依赖
pub struct SimpleService {
    data: RwLock<HashMap<String, Item>>,
    storage: Option<Arc<SqliteManager>>,
}
```

### 3. 多层嵌套调用

```rust
// ❌ 避免 - 多层嵌套
Frontend → Commands → Processor → Manager → Adapter → Client → API

// ✅ 推荐 - 直接调用
Frontend → Commands → Service → Storage/API
```

### 4. 过度配置化

```rust
// ❌ 避免 - 过度配置
pub struct OverConfiguredService {
    config: ComplexConfig,
    strategies: HashMap<String, Box<dyn Strategy>>,
    plugins: Vec<Box<dyn Plugin>>,
}

// ✅ 推荐 - 简单配置
pub struct SimpleService {
    storage: Option<Arc<SqliteManager>>,
    cache_ttl: Duration,
}
```
