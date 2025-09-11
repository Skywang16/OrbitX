# OrbitX API标准化规范手册

## 📋 概述

本文档是**严格的规范要求**，定义了每个模块在标准化改造时必须遵循的具体格式和做法。所有模块改造都必须严格按照此规范执行，确保整个项目的一致性。

**适用范围**: 所有功能模块（workspace、terminal、window、ai、config、llm等）

---

## 🎯 标准化四部曲

每个模块的标准化改造分为4个步骤：

1. **命名标准化** - 统一命令名称格式
2. **参数标准化** - 统一参数结构和验证
3. **分层架构重构** - 抽取Service层，实现4层架构
4. **前端API标准化** - 统一调用方式

---

## 📏 命名规范（严格要求）

### 规则1：命令命名格式

```
格式: {domain}_{verb}[_{target}]
```

### 规则2：功能域（domain）定义

| 功能域     | 命名        | 示例命令前缀  |
| ---------- | ----------- | ------------- |
| 工作区管理 | `workspace` | `workspace_*` |
| 终端管理   | `terminal`  | `terminal_*`  |
| 窗口管理   | `window`    | `window_*`    |
| AI功能     | `ai`        | `ai_*`        |
| 配置管理   | `config`    | `config_*`    |
| LLM调用    | `llm`       | `llm_*`       |

### 规则3：动词（verb）定义

| 动作类型 | 动词      | 用途         | 示例                      |
| -------- | --------- | ------------ | ------------------------- |
| 查询类   | `get`     | 获取单个实体 | `workspace_get`           |
| 查询类   | `list`    | 获取列表     | `workspace_list_all`      |
| 查询类   | `check`   | 检查状态     | `workspace_check_current` |
| 查询类   | `find`    | 查找/搜索    | `workspace_find`          |
| 操作类   | `create`  | 创建新实体   | `workspace_create`        |
| 操作类   | `update`  | 更新实体     | `workspace_update`        |
| 操作类   | `delete`  | 删除实体     | `workspace_delete`        |
| 操作类   | `build`   | 构建/生成    | `workspace_build_index`   |
| 操作类   | `refresh` | 刷新/重建    | `workspace_refresh`       |
| 控制类   | `start`   | 启动         | `terminal_start`          |
| 控制类   | `stop`    | 停止         | `terminal_stop`           |
| 控制类   | `toggle`  | 切换         | `window_toggle_opacity`   |
| 控制类   | `set`     | 设置         | `config_set_theme`        |

### 规则4：目标（target）定义（可选）

| 目标       | 用途       | 示例                      |
| ---------- | ---------- | ------------------------- |
| `_current` | 当前活动的 | `workspace_check_current` |
| `_all`     | 所有/全部  | `workspace_list_all`      |
| `_active`  | 活跃的     | `terminal_get_active`     |
| `_default` | 默认的     | `config_get_default`      |

---

## 📁 文件结构规范（严格要求）

### 规范1：目录结构

每个模块必须按以下结构组织：

```
src-tauri/src/commands/{domain}/
├── mod.rs           # 模块主文件，导出所有命令
├── types.rs         # 请求/响应类型定义
└── {sub_module}.rs  # 子模块（可选，用于复杂功能域）

src/api/{domain}/
├── index.ts         # API客户端主文件
└── types.ts         # TypeScript类型定义
```

### 规范2：文件命名

- 目录名: `snake_case`（如：`workspace`、`terminal`）
- 文件名: `snake_case.rs` 或 `camelCase.ts`
- 模块名: 与目录名一致

---

## 🔧 后端实现规范

### 规范1：types.rs 文件格式

```rust
//! {功能域}相关的请求和响应类型定义

use serde::{Deserialize, Serialize};

// ===== 请求类型 =====

/// {动作}请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct {Verb}{Domain}Request {
    // 必要字段
    pub {field}: {Type},

    // 可选字段
    pub {optional_field}: Option<{Type}>,
}

// ===== 响应类型 =====
// 复用现有类型或定义新类型

// ===== 验证方法 =====

impl {Verb}{Domain}Request {
    /// 验证请求参数
    pub fn validate(&self) -> Result<(), String> {
        // 必要的验证逻辑
        if self.{field}.trim().is_empty() {
            return Err("{字段}不能为空".to_string());
        }

        Ok(())
    }
}
```

**示例**：

```rust
//! 工作区相关的请求和响应类型定义

use serde::{Deserialize, Serialize};

// ===== 请求类型 =====

/// 构建工作区索引请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildWorkspaceRequest {
    pub path: String,
    pub name: Option<String>,
}

/// 删除工作区请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorkspaceRequest {
    pub id: i32,
}

// ===== 验证方法 =====

impl BuildWorkspaceRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.path.trim().is_empty() {
            return Err("工作区路径不能为空".to_string());
        }
        Ok(())
    }
}

impl DeleteWorkspaceRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.id <= 0 {
            return Err("无效的工作区ID".to_string());
        }
        Ok(())
    }
}
```

### 规范2：mod.rs 文件格式

```rust
//! {功能域}管理命令模块
//!
//! 提供标准化的{功能域}相关Tauri命令

pub mod types;

use self::types::*;
// 其他必要的导入...

// ===== 新的标准化命令 =====

/// {动作描述}
#[tauri::command]
pub async fn {domain}_{verb}[_{target}](
    request: {Verb}{Domain}Request,    // 有参数的命令
    state: State<'_, {StateType}>,
) -> TauriApiResult<{ResponseType}> {
    debug!("执行命令: {command_name}, request: {:?}", request);

    // 1. 参数验证
    if let Err(e) = request.validate() {
        return Ok(api_error!(&e));
    }

    // 2. 调用业务逻辑（复用现有实现或新实现）
    // ... 具体实现

    // 3. 返回结果
    match result {
        Ok(data) => Ok(api_success!(data)),
        Err(e) => {
            error!("{动作}失败: {}", e);
            Ok(api_error!(&e.to_string()))
        }
    }
}

/// {动作描述}（无参数版本）
#[tauri::command]
pub async fn {domain}_{verb}[_{target}](
    state: State<'_, {StateType}>,
) -> TauriApiResult<{ResponseType}> {
    debug!("执行命令: {command_name}");

    // 直接调用业务逻辑
    // ... 具体实现

    match result {
        Ok(data) => Ok(api_success!(data)),
        Err(e) => {
            error!("{动作}失败: {}", e);
            Ok(api_error!(&e.to_string()))
        }
    }
}

// ===== 向后兼容的废弃命令 =====

#[deprecated(note = "请使用 {new_command_name} 替代")]
#[tauri::command]
pub async fn {old_command_name}(
    // 原有参数...
) -> TauriApiResult<{ResponseType}> {
    // 转换参数并调用新命令
    let request = {NewRequestType} {
        // 参数转换...
    };
    {new_command_name}(request, state).await
}
```

### 规范3：命令参数规范

#### 3.1 有参数的命令

```rust
#[tauri::command]
pub async fn {domain}_{verb}(
    request: {Verb}{Domain}Request,        // 第1个参数：业务请求
    state: State<'_, {StateType}>,        // 第2个参数：应用状态
) -> TauriApiResult<{ResponseType}> {
    // 实现...
}
```

#### 3.2 无参数的命令

```rust
#[tauri::command]
pub async fn {domain}_{verb}(
    state: State<'_, {StateType}>,        // 唯一参数：应用状态
) -> TauriApiResult<{ResponseType}> {
    // 实现...
}
```

### 规范4：错误处理规范

```rust
// 统一的错误处理模式
match result {
    Ok(data) => Ok(api_success!(data)),
    Err(e) => {
        error!("{操作描述}失败: {}", e);
        Ok(api_error!(&e.to_string()))
    }
}

// 参数验证错误
if let Err(e) = request.validate() {
    return Ok(api_error!(&e));
}

// 业务逻辑错误
if some_condition {
    return Ok(api_error!("具体的错误描述"));
}
```

---

## 🏗️ 分层架构规范（核心）

### 架构概述

```
app_state → services → repositories → system_apis
    ↓           ↓           ↓            ↓
   全局状态   业务逻辑层   数据访问层    系统调用层
```

### 规范1：Service层实现

#### 1.1 Service文件结构

```
src-tauri/src/services/
├── mod.rs              # 导出所有Service
├── base.rs             # Service基础接口
└── {domain}.rs         # 具体功能域Service
```

#### 1.2 base.rs 规范格式

```rust
//! Service层基础接口定义

use anyhow::Result;
use std::sync::Arc;

/// 应用服务基础接口
#[async_trait::async_trait]
pub trait AppService: Send + Sync {
    /// 服务名称
    fn name(&self) -> &'static str;

    /// 初始化服务
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    /// 清理资源
    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}

/// 统一的应用状态管理
pub struct AppState {
    // 数据层
    pub repositories: Arc<RepositoryManager>,
    pub cache: Arc<UnifiedCache>,

    // 系统层
    pub terminal_context_service: Arc<TerminalContextService>,

    // 业务服务层
    pub workspace_service: Arc<WorkspaceService>,
    pub terminal_service: Arc<TerminalService>,
    pub window_service: Arc<WindowService>,
    pub ai_service: Arc<AiService>,
    pub config_service: Arc<ConfigService>,
    pub llm_service: Arc<LlmService>,
}

impl AppState {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        cache: Arc<UnifiedCache>,
        terminal_context_service: Arc<TerminalContextService>,
    ) -> Result<Self> {
        Ok(Self {
            // 初始化所有服务
            workspace_service: Arc::new(WorkspaceService::new(Arc::clone(&repositories))),
            terminal_service: Arc::new(TerminalService::new(Arc::clone(&repositories))),
            window_service: Arc::new(WindowService::new()),
            ai_service: Arc::new(AiService::new(Arc::clone(&repositories))),
            config_service: Arc::new(ConfigService::new(Arc::clone(&repositories))),
            llm_service: Arc::new(LlmService::new()),

            repositories,
            cache,
            terminal_context_service,
        })
    }
}
```

#### 1.3 具体Service实现规范

```rust
//! {功能域}业务逻辑服务

use super::base::AppService;
use crate::storage::repositories::RepositoryManager;
use anyhow::Result;
use std::sync::Arc;

/// {功能域}业务逻辑服务
pub struct {Domain}Service {
    repositories: Arc<RepositoryManager>,  // → repositories层
    // 其他依赖...
}

impl {Domain}Service {
    /// 创建服务实例
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        Self { repositories }
    }

    /// 核心业务方法1
    pub async fn {business_method1}(&self, {params}) -> Result<{ReturnType}> {
        // 1. 业务逻辑处理
        // 2. 调用repositories层获取数据
        // 3. 调用system_apis（如文件系统、网络等）
        // 4. 返回业务结果
    }

    /// 核心业务方法2
    pub async fn {business_method2}(&self, {params}) -> Result<{ReturnType}> {
        // 业务逻辑实现...
    }

    // 私有辅助方法
    async fn {helper_method}(&self, {params}) -> Result<{ReturnType}> {
        // 辅助逻辑...
    }
}

#[async_trait::async_trait]
impl AppService for {Domain}Service {
    fn name(&self) -> &'static str {
        "{Domain}Service"
    }

    async fn initialize(&self) -> Result<()> {
        // 服务初始化逻辑（如果需要）
        Ok(())
    }
}
```

#### 1.4 Service层职责划分

| 层级             | 职责                             | 不应该做                    |
| ---------------- | -------------------------------- | --------------------------- |
| **Service层**    | 业务逻辑处理、参数验证、业务规则 | 直接数据库操作、系统API调用 |
| **Repository层** | 数据存取、查询构建               | 业务逻辑、复杂计算          |
| **System APIs**  | 文件系统、网络、进程等系统调用   | 业务逻辑、数据缓存          |

### 规范2：Command层改造

#### 2.1 薄化Command层

```rust
/// 标准化Command实现（薄薄的转发层）
#[tauri::command]
pub async fn {domain}_{verb}(
    request: {Verb}{Domain}Request,
    state: State<'_, AppState>,           // 使用统一的AppState
) -> TauriApiResult<{ResponseType}> {
    debug!("执行命令: {domain}_{verb}, request: {:?}", request);

    // 1. 参数验证（简单验证，复杂验证在Service层）
    if let Err(e) = request.validate() {
        return Ok(api_error!(&e));
    }

    // 2. 调用Service层（核心业务逻辑）
    match state.{domain}_service.{business_method}(request.into()).await {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => {
            error!("{domain}_{verb} 失败: {}", e);
            Ok(api_error!(&e.to_string()))
        }
    }
}
```

#### 2.2 参数转换规范

```rust
// 在types.rs中实现From trait进行参数转换
impl From<{Verb}{Domain}Request> for {ServiceMethodParams} {
    fn from(request: {Verb}{Domain}Request) -> Self {
        Self {
            // 参数转换逻辑
        }
    }
}
```

### 规范3：现有逻辑迁移

#### 3.1 迁移策略

```rust
// 第1步：将现有Command中的业务逻辑移到Service层
impl WorkspaceService {
    pub async fn check_current_workspace(&self) -> Result<Option<WorkspaceIndex>> {
        // 这里放原来在 check_current_workspace_index 命令中的业务逻辑

        // 调用repositories层
        let workspace = self.repositories
            .vector_workspaces()
            .find_by_path(&current_path)
            .await?;

        // 调用system APIs
        let current_dir = std::env::current_dir()
            .map_err(|e| anyhow!("获取当前目录失败: {}", e))?;

        // 业务逻辑处理
        match workspace {
            Some(mut index) => {
                // 检查索引文件是否存在等业务判断
                Ok(Some(index))
            }
            None => Ok(None),
        }
    }
}

// 第2步：Command层调用Service层
#[tauri::command]
pub async fn workspace_check_current(
    state: State<'_, AppState>,
) -> TauriApiResult<Option<WorkspaceIndex>> {
    match state.workspace_service.check_current_workspace().await {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => Ok(api_error!(&e.to_string())),
    }
}
```

#### 3.2 依赖注入模式

```rust
// Service层通过构造函数注入依赖
impl WorkspaceService {
    pub fn new(
        repositories: Arc<RepositoryManager>,           // repositories层
        terminal_context: Arc<TerminalContextService>, // system APIs层
    ) -> Self {
        Self { repositories, terminal_context }
    }

    pub async fn get_current_directory(&self) -> Result<String> {
        // 优先从terminal context获取（system APIs层）
        match self.terminal_context.get_active_cwd().await {
            Ok(cwd) => Ok(cwd),
            Err(_) => {
                // 回退到系统API
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .map_err(|e| anyhow!("获取当前目录失败: {}", e))
            }
        }
    }
}
```

### 规范4：Service层测试

#### 4.1 Service层单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repositories::RepositoryManager;
    use std::sync::Arc;

    fn create_test_service() -> WorkspaceService {
        // 创建测试用的Service实例
        let repositories = Arc::new(RepositoryManager::new_test());
        WorkspaceService::new(repositories)
    }

    #[tokio::test]
    async fn test_check_current_workspace() {
        let service = create_test_service();
        let result = service.check_current_workspace().await;

        assert!(result.is_ok());
        // 更多断言...
    }
}
```

---

## 🌐 前端实现规范

### 规范1：types.ts 文件格式

```typescript
// src/api/{domain}/types.ts

// ===== 请求类型 =====
export interface {Verb}{Domain}Request {
  {field}: {Type}
  {optionalField}?: {Type}
}

// ===== 响应类型 =====
export interface {Domain}{Entity} {
  {field}: {Type}
  // 使用camelCase命名
}

// ===== 枚举类型 =====
export enum {Domain}{EnumName} {
  {Value1} = 'value1',
  {Value2} = 'value2',
}
```

**示例**：

```typescript
// src/api/workspace/types.ts

// ===== 请求类型 =====
export interface BuildWorkspaceRequest {
  path: string
  name?: string
}

export interface DeleteWorkspaceRequest {
  id: number
}

// ===== 响应类型 =====
export interface WorkspaceIndex {
  workspaceId: number
  workspacePath: string
  name?: string
  status: WorkspaceStatus
  createdAt: string
  updatedAt: string
}

// ===== 枚举类型 =====
export enum WorkspaceStatus {
  Building = 'building',
  Ready = 'ready',
  Error = 'error',
}
```

### 规范2：index.ts 文件格式

```typescript
// src/api/{domain}/index.ts
import { ServiceApi } from '@/api/base/ServiceApi'
import type { /* 导入所有需要的类型 */ } from './types'

export class {Domain}Api extends ServiceApi {
  constructor() {
    super('{domain}')
  }

  // 按字母顺序排列方法

  async {verb}({params}): Promise<{ReturnType}> {
    return await this.invoke<{ReturnType}>('{verb}', {params})
  }

  async {verb}(): Promise<{ReturnType}> {
    return await this.invoke<{ReturnType}>('{verb}')
  }
}

export const {domain}Api = new {Domain}Api()
export default {domain}Api

// 导出所有类型
export type * from './types'
```

**示例**：

```typescript
// src/api/workspace/index.ts
import { ServiceApi } from '@/api/base/ServiceApi'
import type { BuildWorkspaceRequest, DeleteWorkspaceRequest, RefreshWorkspaceRequest, WorkspaceIndex } from './types'

export class WorkspaceApi extends ServiceApi {
  constructor() {
    super('workspace')
  }

  async buildIndex(request: BuildWorkspaceRequest): Promise<WorkspaceIndex> {
    return await this.invoke<WorkspaceIndex>('build_index', request)
  }

  async checkCurrent(): Promise<WorkspaceIndex | null> {
    return await this.invoke<WorkspaceIndex | null>('check_current')
  }

  async delete(request: DeleteWorkspaceRequest): Promise<void> {
    await this.invoke<void>('delete', request)
  }

  async listAll(): Promise<WorkspaceIndex[]> {
    return await this.invoke<WorkspaceIndex[]>('list_all')
  }

  async refresh(request: RefreshWorkspaceRequest): Promise<WorkspaceIndex> {
    return await this.invoke<WorkspaceIndex>('refresh', request)
  }
}

export const workspaceApi = new WorkspaceApi()
export default workspaceApi

export type * from './types'
```

---

## 📋 模块改造检查清单

每个模块改造完成后，必须通过以下检查：

### ✅ 命名检查

- [ ] 所有新命令遵循 `{domain}_{verb}[_{target}]` 格式
- [ ] 功能域名称在规范列表中
- [ ] 动词选择正确且一致
- [ ] 无拼写错误

### ✅ 文件结构检查

- [ ] 目录结构符合规范
- [ ] `mod.rs` 文件包含所有必要部分
- [ ] `types.rs` 文件格式正确
- [ ] 导入导出语句完整

### ✅ 类型定义检查

- [ ] 所有Request类型使用 `#[serde(rename_all = "camelCase")]`
- [ ] 参数验证方法完整
- [ ] 前端TypeScript类型与后端一致
- [ ] 枚举类型定义正确

### ✅ 错误处理检查

- [ ] 使用统一的错误处理模式
- [ ] 参数验证错误处理正确
- [ ] 日志记录完整
- [ ] 错误消息用户友好

### ✅ 向后兼容检查

- [ ] 旧命令标记为 `#[deprecated]`
- [ ] 旧命令功能正常
- [ ] 参数转换正确
- [ ] 前端调用不受影响

### ✅ 功能检查

- [ ] 所有新命令编译通过
- [ ] 所有新命令功能正常
- [ ] 前端API调用成功
- [ ] 错误场景处理正确

### ✅ 代码质量检查

- [ ] 代码注释完整
- [ ] 命名清晰一致
- [ ] 无重复代码
- [ ] 性能无回归

---

## 🔧 命令注册规范

### 规范1：模块导入

在 `src-tauri/src/commands/mod.rs` 中：

```rust
// 在模块声明部分添加
pub mod {domain};

// 在重新导出部分添加
pub use {domain}::*;
```

### 规范2：命令注册

在 `src-tauri/src/commands/mod.rs` 的 `register_all_commands` 函数中：

```rust
pub fn register_all_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        // ... 现有命令

        // {功能域}命令 - 新的标准化命令
        crate::commands::{domain}::{domain}_{verb1},
        crate::commands::{domain}::{domain}_{verb2},
        // ... 其他新命令

        // {功能域}命令 - 旧命令（向后兼容）
        crate::commands::{domain}::{old_command1},
        crate::commands::{domain}::{old_command2},
        // ... 其他旧命令

        // ... 其他现有命令
    ])
}
```

---

## 📊 API导出规范

### 规范1：统一导出

在 `src/api/index.ts` 中：

```typescript
// 导出各个API实例
export { {domain}Api } from './{domain}'
export { {domain2}Api } from './{domain2}'

// 统一导出所有API
export const api = {
  {domain}: {domain}Api,
  {domain2}: {domain2}Api,
  // ... 其他API
}

// 导出所有类型（可选）
export type * from './{domain}/types'
export type * from './{domain2}/types'
```

---

## ⚠️ 注意事项和最佳实践

### 1. 复用现有逻辑

- ✅ 优先复用现有的业务逻辑实现
- ✅ 将现有函数重命名为 `{old_name}_impl` 形式
- ❌ 不要重写已经工作的业务逻辑

### 2. 渐进式改造

- ✅ 一次只改造一个模块
- ✅ 每个模块改造完成后立即测试
- ❌ 不要同时改造多个模块

### 3. 向后兼容

- ✅ 保持所有旧命令的功能
- ✅ 使用 `#[deprecated]` 标记旧命令
- ❌ 不要立即删除旧命令

### 4. 测试验证

- ✅ 每完成一个模块就进行完整测试
- ✅ 检查前端调用是否正常
- ✅ 验证错误处理是否正确

### 5. 提交规范

每个模块改造完成后的提交格式：

```
feat: {功能域}模块API标准化

- 新增标准化的{功能域}命令（{domain}_*格式）
- 保持旧命令向后兼容（标记为deprecated）
- 统一请求参数格式和验证
- 更新前端API客户端

Breaking Changes: 无（保持向后兼容）
```

---

## 🎯 模块改造优先级

建议按以下顺序改造模块：

1. **workspace** - 最复杂，先做样板
2. **terminal** - 核心功能
3. **window** - 相对简单
4. **config** - 配置相关
5. **ai** - AI功能
6. **llm** - LLM调用

---

严格按照这个规范执行，确保每个模块都有完全一致的结构和风格！
