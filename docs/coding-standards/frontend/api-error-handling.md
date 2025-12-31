# 前端 API 错误处理规范

## 核心原则

### 后端统一处理，前端信任后端

本项目采用 **后端统一错误处理** 的架构设计。所有 API 调用的错误处理、错误提示已在后端和请求拦截器层面完成，**前端不需要也不应该重复处理**。

## 错误处理流程

### 后端统一响应格式

所有 API 响应遵循统一格式：

```typescript
interface ApiResponse<T> {
  code: number // 状态码：200 表示成功，其他表示失败
  message?: string // 错误信息（已国际化）
  data?: T // 业务数据
}
```

### 请求拦截器自动处理

`src/utils/request/index.ts` 中的 `invoke` 函数已实现：

```typescript
export const invoke = async <T>(command: string, args?: Record<string, unknown>, options?: APIOptions): Promise<T> => {
  const response = await api.invoke<ApiResponse<T>>(command, args, options)

  if (response.code === 200) {
    return response.data as T
  } else {
    // 统一错误提示 - 后端已完成国际化
    createMessage.error(response.message || '操作失败')
    throw new APIError(response.message || '操作失败', String(response.code))
  }
}
```

**关键点**：

- ✅ 自动显示错误提示（`createMessage.error`）
- ✅ 错误信息已由后端国际化
- ✅ 抛出标准的 `APIError` 异常

## API 声明规范

### 统一在 src/api 中声明

**所有后端接口调用都必须在 `src/api` 目录中声明**，不允许在组件或 Store 中直接调用 Tauri 命令。

### 使用封装的 invoke 函数

所有 API 声明必须使用 `src/utils/request/index.ts` 中导出的 `invoke` 函数：

```typescript
import { invoke } from '@/utils/request'
```

### 标准 API 声明示例

```typescript
/**
 * Workspace 会话 API - 基于 workspace_v2 命令
 */

import { invoke } from '@/utils/request'
import type { SessionRecord, UiMessage } from './types'

export class WorkspaceV2Api {
  /**
   * 创建 Session
   */
  async createSession(workspacePath: string, title?: string): Promise<SessionRecord> {
    return invoke<SessionRecord>('workspace_v2_create_session', { path: workspacePath, title })
  }

  /**
   * 获取 Session 列表
   */
  async listSessions(workspacePath: string): Promise<SessionRecord[]> {
    return invoke<SessionRecord[]>('workspace_v2_list_sessions', { path: workspacePath })
  }

  /**
   * 获取消息列表
   */
  async getMessages(sessionId: number): Promise<UiMessage[]> {
    return invoke<UiMessage[]>('workspace_v2_get_messages', { sessionId })
  }

  /**
   * 切换活跃 Session
   */
  async setActiveSession(workspacePath: string, sessionId: number): Promise<void> {
    return invoke<void>('workspace_v2_set_active_session', { path: workspacePath, sessionId })
  }
}

export const workspaceV2Api = new WorkspaceV2Api()
```

### 为什么要统一声明？

1. **类型安全**：在 API 层统一定义类型，确保类型正确
2. **易于维护**：所有接口集中管理，修改时只需改一处
3. **错误处理统一**：所有接口自动应用 `invoke` 的错误处理逻辑
4. **便于测试**：可以轻松 mock API 层进行测试
5. **代码复用**：避免在多个地方重复定义相同的接口调用

### ❌ 错误的做法

```typescript
// ❌ 直接在组件中调用 Tauri 命令
import { invoke as tauriInvoke } from '@tauri-apps/api/core'

const setActiveSession = async (workspacePath: string, sessionId: number) => {
  // 跳过了统一的错误处理
  const result = await tauriInvoke('workspace_v2_set_active_session', {
    path: workspacePath,
    sessionId,
  })
  // 需要手动处理错误，容易遗漏
}
```

### ✅ 正确的做法

```typescript
// ✅ 使用 API 层声明的接口
import { workspaceV2Api } from '@/api/workspace/v2'

const switchSession = async (workspacePath: string, sessionId: number) => {
  // 自动应用错误处理、类型检查
  await workspaceV2Api.setActiveSession(workspacePath, sessionId)
}
```

### API 文件组织

```text
src/api/
├── index.ts           # 导出所有 API
├── agent/
│   ├── index.ts       # AgentApi 类
│   └── types.ts       # Agent 相关类型
├── terminal/
│   ├── index.ts       # TerminalApi 类
│   └── types.ts       # Terminal 相关类型
├── config/
│   └── index.ts       # ConfigApi 类
└── ...
```

### 使用示例

```typescript
// 在组件或 Store 中使用
import { workspaceV2Api, terminalApi, configApi } from '@/api'

// 直接调用，不需要 try-catch
await workspaceV2Api.createSession(currentWorkspacePath.value!)
await terminalApi.createTerminal({ rows: 24, cols: 80 })
await configApi.saveConfig(config)
```

## ❌ 错误的写法

### 反模式 1：不必要的 try-catch

```typescript
// ❌ 错误：API 调用使用 try-catch 包裹
const switchSession = async (sessionId: number): Promise<void> => {
  try {
    await workspaceV2Api.setActiveSession(currentWorkspacePath.value!, sessionId)
    activeSessionId.value = sessionId
  } catch (err) {
    error.value = '切换会话失败' // 重复的错误处理
  }
}
```

**问题**：

1. 错误消息已经在 `invoke` 中显示，这里又显示一次（重复）
2. 后端返回的错误信息更准确，前端硬编码的错误信息不够精确
3. 增加了不必要的代码复杂度

### 反模式 2：捕获后忽略错误

```typescript
// ❌ 错误：捕获错误但什么都不做
const refreshSessions = async () => {
  try {
    await workspaceStore.refreshSessions()
  } catch {
    // Refresh failures are non-critical
  }
}
```

**问题**：

1. 用户看不到错误提示（错误被吞掉）
2. 开发者无法追踪问题（没有日志）
3. 如果确实不重要，应该在后端决定是否显示错误

### 反模式 3：重复的错误提示

```typescript
// ❌ 错误：前端自己再次显示错误
const buildCkIndex = async () => {
  try {
    const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
    if (!activeTerminal || !activeTerminal.cwd) return
    await ckApi.buildIndex({ path: activeTerminal.cwd })
  } catch (error) {
    console.error('构建CK索引失败:', error) // 重复的错误处理
    createMessage.error('构建失败') // 重复的错误提示
  }
}
```

**问题**：

1. `ckApi.buildIndex` 失败时，`invoke` 已经显示错误
2. 这里又显示一次，用户会看到两次错误提示
3. 前端的错误信息不如后端准确

## ✅ 正确的写法

### 标准模式：直接调用，不包裹 try-catch

```typescript
// ✅ 正确：直接调用 API，信任后端错误处理
const activateSession = async (sessionId: number): Promise<void> => {
  await workspaceV2Api.setActiveSession(currentWorkspacePath.value!, sessionId)
  activeSessionId.value = sessionId
}

const refreshSessions = async (): Promise<void> => {
  sessions.value = await workspaceV2Api.listSessions(currentWorkspacePath.value!)
}

const buildCkIndex = async () => {
  const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
  if (!activeTerminal || !activeTerminal.cwd) return

  await ckApi.buildIndex({ path: activeTerminal.cwd })
  // API 调用成功后继续执行
  startProgressPolling(activeTerminal.cwd)
}
```

**优势**：

1. 代码简洁清晰
2. 错误处理统一（由后端和拦截器负责）
3. 错误信息准确（后端提供的详细信息）
4. 避免重复显示错误

## 特殊场景处理

### 场景 1：需要在 UI 中反映 loading 状态

```typescript
// ✅ 正确：使用 loading 状态，不需要 try-catch
const loadSession = async (sessionId: number): Promise<void> => {
  isLoading.value = true
  currentSessionId.value = sessionId
  messageList.value = await workspaceV2Api.getMessages(sessionId)
  isLoading.value = false
}
```

**注意**：

- 如果 API 失败，`isLoading` 不会被设置为 `false`，这是正确的
- 因为页面会因为错误被中断，用户会看到错误提示
- 不需要 `finally` 来重置 loading 状态

### 场景 2：需要执行后续清理操作

```typescript
// ✅ 正确：使用 finally 执行清理，但不捕获错误
const sendMessage = async (content: string): Promise<void> => {
  isLoading.value = true
  error.value = null

  try {
    const stream = await agentApi.executeTask({
      workspacePath: currentWorkspacePath.value!,
      sessionId: currentSessionId.value!,
      userPrompt: content,
      modelId: selectedModelId.value,
    })

    // 设置流的回调
    stream.onProgress(handleProgress)
    stream.onError(handleError)
    stream.onClose(handleClose)
  } finally {
    // 无论成功失败，都清理资源
    cancelFunction.value = null
  }
}
```

**关键点**：

- 使用 `finally` 来清理资源
- 不使用 `catch`，让错误继续传播
- 错误由拦截器统一处理

### 场景 3：错误不应中断流程

某些情况下，API 失败不应该中断整个流程（例如日志记录、统计上报）。

```typescript
// ✅ 正确：捕获错误但使用 console.warn
const recordAnalytics = async (event: string) => {
  try {
    await analyticsApi.record(event)
  } catch (error) {
    // 分析记录失败不应影响用户操作
    console.warn('Analytics recording failed:', error)
  }
}
```

**适用场景**（极少）：

- 日志记录
- 统计上报
- 非关键的后台任务

**注意**：这种场景非常罕见，99% 的 API 调用都不应该这样处理。

### 场景 4：需要自定义错误处理逻辑

如果确实需要根据错误类型执行不同的操作：

```typescript
// ✅ 正确：捕获错误后执行特定逻辑，但仍然重新抛出
const initialize = async (): Promise<void> => {
  try {
    await loadConfiguration()
  } catch (error) {
    // 加载失败时使用默认配置
    useDefaultConfiguration()

    // 重新抛出，让用户知道发生了错误
    throw error
  }
}
```

**关键点**：

- 执行必要的容错处理
- **必须重新抛出错误**，让拦截器显示错误提示
- 不要吞掉错误

## 检查清单

在编写 API 调用代码时，检查以下几点：

- [ ] **不使用 try-catch** 包裹 API 调用（99% 的情况）
- [ ] **不捕获后忽略** 错误
- [ ] **不重复显示** 错误提示（`createMessage.error`）
- [ ] **不自己编写** 错误提示文案（应由后端提供）
- [ ] 如果使用 try-catch，确认是否符合特殊场景
- [ ] 如果捕获错误，确认是否重新抛出

## 为什么这样设计？

### 1. 单一职责原则

- **后端职责**：业务逻辑、数据验证、错误处理、国际化
- **前端职责**：数据展示、用户交互

错误处理属于业务逻辑的一部分，应该由后端负责。

### 2. DRY 原则（Don't Repeat Yourself）

如果每个 API 调用都要写 try-catch：

- 代码重复（每个地方都要写错误处理）
- 维护困难（修改错误提示要改多处）
- 容易遗漏（忘记某个地方的错误处理）

统一在拦截器中处理，一次编写，处处生效。

### 3. 一致性原则

所有错误提示格式统一：

- 提示方式统一（都用 `createMessage.error`）
- 提示内容统一（都来自后端国际化）
- 用户体验一致（所有错误都用同样的方式展示）

### 4. 后端更了解错误

后端可以提供更详细、准确的错误信息：

- 数据库错误 → "记录不存在"
- 权限错误 → "没有操作权限"
- 业务错误 → "订单已关闭，无法修改"

前端很难提供这么准确的错误信息。

## 迁移指南

### 识别需要修复的代码

搜索以下模式：

```typescript
// 模式 1：try-catch 包裹 API 调用
try {
  await someApi.method()
} catch (error) {
  // ...
}

// 模式 2：catch 后显示错误
} catch (error) {
  createMessage.error('xxx失败')
}

// 模式 3：catch 后设置 error 状态
} catch (err) {
  error.value = 'xxx失败'
}
```

### 修复步骤

1. **删除 try-catch 包裹**

   ```typescript
   // 修复前
   try {
     await workspaceV2Api.setActiveSession(currentWorkspacePath.value!, sessionId)
   } catch (err) {
     error.value = '切换会话失败'
   }

   // 修复后
   await workspaceV2Api.setActiveSession(currentWorkspacePath.value!, sessionId)
   ```

2. **移除重复的错误提示**

   ```typescript
   // 修复前
   try {
     await ckApi.buildIndex({ path })
   } catch (error) {
     createMessage.error('构建失败')
   }

   // 修复后
   await ckApi.buildIndex({ path })
   ```

3. **移除不必要的 finally**

   ```typescript
   // 修复前
   try {
     isLoading.value = true
     await api.getData()
   } finally {
     isLoading.value = false
   }

   // 修复后
   isLoading.value = true
   await api.getData()
   isLoading.value = false
   ```

4. **保留必要的 finally**（用于资源清理）
   ```typescript
   // ✅ 这种情况保留
   try {
     await longRunningOperation()
   } finally {
     cleanup() // 必须执行的清理操作
   }
   ```

## 总结

| 场景               | 是否使用 try-catch | 原因                         |
| ------------------ | ------------------ | ---------------------------- |
| 普通 API 调用      | ❌ 否              | 错误已在拦截器中统一处理     |
| 需要 loading 状态  | ❌ 否              | 直接设置 loading，失败会中断 |
| 需要资源清理       | ✅ 使用 finally    | 清理资源但不捕获错误         |
| 非关键操作（极少） | ✅ 使用 try-catch  | 捕获错误并 console.warn      |
| 需要容错处理       | ✅ 使用 try-catch  | 处理后**必须重新抛出**       |

**记住**：99% 的 API 调用都不需要 try-catch。如果你写了 try-catch，先问问自己是否真的需要。

**黄金法则**：**后端统一处理错误，前端信任后端。**
