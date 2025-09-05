# Eko-Core AI-SDK 到原生后端重构详细任务清单

## 项目概述

将 `src/eko-core` 目录中所有使用 ai-sdk 的代码改为使用原生后端接口，通过 Tauri 2.0 原生 Channel 双向通信传递流式数据。

## 后端接口概览

### 可用的 Tauri 命令

- `llm_call(request: LLMRequest) -> LLMResponse` - 非流式调用
- `llm_call_stream(request: LLMRequest, on_chunk: Channel<LLMStreamChunk>) -> ()` - 流式调用
- `llm_get_available_models() -> Vec<String>` - 获取模型列表
- `llm_test_model_connection(model_id: String) -> bool` - 测试连接

### 后端类型定义（Rust）

```rust
// 消息类型
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

// 工具类型
pub struct LLMTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct LLMToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

// 请求类型
pub struct LLMRequest {
    pub model_id: String,
    pub messages: Vec<LLMMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<LLMTool>>,
    pub tool_choice: Option<String>,
    pub stream: bool,
}

// 响应类型
pub struct LLMResponse {
    pub content: String,
    pub finish_reason: String,
    pub tool_calls: Option<Vec<LLMToolCall>>,
    pub usage: Option<LLMUsage>,
}

// 流式数据块类型
pub enum LLMStreamChunk {
    TextStart { stream_id: String },
    TextDelta { stream_id: String, delta: String, text: String },
    TextEnd { stream_id: String, text: String },
    ReasoningStart { stream_id: String },
    ReasoningDelta { stream_id: String, delta: String, text: String },
    ReasoningEnd { stream_id: String, text: String },
    ToolCallStart { tool_id: String, tool_name: String },
    ToolArgsStreaming { tool_id: String, tool_name: String, args_text: String },
    ToolCall { tool_id: String, tool_name: String, arguments: serde_json::Value },
    Finish { finish_reason: String, usage: Option<LLMUsage> },
    Error { error: String },
}
```

## 详细任务清单

### 阶段一：类型定义重构

#### 任务 1.1：修改 `src/eko-core/types/llm.types.ts`

- [ ] 移除所有 ai-sdk 导入
  ```typescript
  // 删除这些导入
  import {
    ProviderV2,
    LanguageModelV2CallWarning,
    LanguageModelV2FinishReason,
    LanguageModelV2StreamPart,
    LanguageModelV2FunctionTool,
    LanguageModelV2ToolChoice,
    LanguageModelV2Prompt,
    LanguageModelV2CallOptions,
    LanguageModelV2Content,
    SharedV2Headers,
    SharedV2ProviderMetadata,
    LanguageModelV2Usage,
    LanguageModelV2ResponseMetadata,
  } from '@ai-sdk/provider'
  ```
- [ ] 添加原生类型定义

  ```typescript
  // 原生消息类型
  export interface NativeLLMMessage {
    role: 'system' | 'user' | 'assistant' | 'tool'
    content: string | NativeLLMMessagePart[]
  }

  export interface NativeLLMMessagePart {
    type: 'text' | 'file' | 'tool-call' | 'tool-result'
    text?: string
    mimeType?: string
    data?: string
    toolCallId?: string
    toolName?: string
    args?: Record<string, unknown>
    result?: string | Record<string, unknown>
  }

  // 原生工具类型
  export interface NativeLLMTool {
    name: string
    description: string
    parameters: any
  }

  // 原生请求类型
  export interface NativeLLMRequest {
    modelId: string
    messages: NativeLLMMessage[]
    temperature?: number
    maxTokens?: number
    tools?: NativeLLMTool[]
    toolChoice?: string
    stream: boolean
    abortSignal?: AbortSignal
  }

  // 原生响应类型
  export interface NativeLLMResponse {
    content: string
    finishReason: string
    toolCalls?: NativeLLMToolCall[]
    usage?: NativeLLMUsage
  }

  export interface NativeLLMToolCall {
    id: string
    name: string
    arguments: any
  }

  export interface NativeLLMUsage {
    promptTokens: number
    completionTokens: number
    totalTokens: number
  }

  // 流式数据块类型
  export type NativeLLMStreamChunk =
    | { type: 'text-start'; streamId: string }
    | { type: 'text-delta'; streamId: string; delta: string; text: string }
    | { type: 'text-end'; streamId: string; text: string }
    | { type: 'reasoning-start'; streamId: string }
    | { type: 'reasoning-delta'; streamId: string; delta: string; text: string }
    | { type: 'reasoning-end'; streamId: string; text: string }
    | { type: 'tool-call-start'; toolId: string; toolName: string }
    | { type: 'tool-args-streaming'; toolId: string; toolName: string; argsText: string }
    | { type: 'tool-call'; toolId: string; toolName: string; arguments: any }
    | { type: 'finish'; finishReason: string; usage?: NativeLLMUsage }
    | { type: 'error'; error: string }
  ```

- [ ] 更新现有类型定义

  ```typescript
  // 简化配置，直接使用原生后端，不需要provider字段
  export type LLMConfig = {
    modelId: string // 对应后端的 model_id
    temperature?: number
    maxTokens?: number
  }

  // 生成结果类型
  export type GenerateResult = {
    modelId: string
    text?: string
    content: string
    finishReason: string
    usage: NativeLLMUsage
    toolCalls?: NativeLLMToolCall[]
  }

  // 流式结果类型
  export type StreamResult = {
    modelId: string
    stream: ReadableStream<NativeLLMStreamChunk>
  }

  // 兼容性类型别名
  export type LLMRequest = NativeLLMRequest
  export type LLMs = {
    default: LLMConfig
    [key: string]: LLMConfig
  }
  ```

#### 任务 1.2：修改 `src/eko-core/types/core.types.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2FinishReason } from '@ai-sdk/provider'
  ```
- [ ] 添加原生类型定义
  ```typescript
  export type FinishReason = 'stop' | 'length' | 'tool_calls' | 'content_filter' | 'function_call'
  ```
- [ ] 更新 StreamCallbackMessage 类型
  ```typescript
  // 在 StreamCallbackMessage 联合类型中更新 finish 类型
  | {
      type: 'finish'
      finishReason: FinishReason  // 使用原生类型
      usage: {
        promptTokens: number
        completionTokens: number
        totalTokens: number
      }
    }
  ```

#### 任务 1.3：修改 `src/eko-core/types/dialogue.types.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2FinishReason, LanguageModelV2ToolCallPart } from '@ai-sdk/provider'
  // 保留：import { JSONSchema7 } from 'json-schema'  // 这个不是ai-sdk的，保留
  ```
- [ ] 更新 DialogueTool 接口
  ```typescript
  export interface DialogueTool {
    readonly name: string
    readonly description?: string
    readonly parameters: JSONSchema7 // 保持JSONSchema7，这不是ai-sdk的
    execute: (args: Record<string, unknown>, toolCall: NativeLLMToolCall) => Promise<ToolResult>
  }
  ```
- [ ] 更新 ChatStreamCallbackMessage 类型
  ```typescript
  // 在 ChatStreamCallbackMessage 联合类型中更新 finish 类型
  | {
      type: 'finish'
      finishReason: FinishReason  // 使用原生类型
      usage: {
        promptTokens: number
        completionTokens: number
        totalTokens: number
      }
    }
  ```

#### 任务 1.4：修改 `src/eko-core/types/index.ts`

- [ ] 移除 ai-sdk 类型导出

  ```typescript
  // 删除这些ai-sdk导出
  export type {
    LanguageModelV2Prompt,
    LanguageModelV2TextPart,
    LanguageModelV2FilePart,
    LanguageModelV2StreamPart,
    LanguageModelV2ToolCallPart,
    LanguageModelV2ToolChoice,
    LanguageModelV2FunctionTool,
    LanguageModelV2ToolResultPart,
    LanguageModelV2ToolResultOutput,
  } from '@ai-sdk/provider'

  // 保留JSONSchema7导出（这不是ai-sdk的）
  export type { JSONSchema7 } from 'json-schema'
  ```

- [ ] 添加原生类型别名导出
  ```typescript
  // 导出原生类型别名以保持兼容性
  export type {
    NativeLLMMessage as LanguageModelV2Prompt,
    NativeLLMMessagePart as LanguageModelV2TextPart,
    NativeLLMMessagePart as LanguageModelV2FilePart,
    NativeLLMStreamChunk as LanguageModelV2StreamPart,
    NativeLLMToolCall as LanguageModelV2ToolCallPart,
    NativeLLMTool as LanguageModelV2FunctionTool,
    NativeLLMMessagePart as LanguageModelV2ToolResultPart,
  } from './llm.types'
  ```

### 阶段二：LLM核心模块重构

#### 任务 2.1：完全重写 `src/eko-core/llm/index.ts`

- [ ] 移除所有 ai-sdk 导入
  ```typescript
  // 删除这些导入
  import { LanguageModelV2, LanguageModelV2CallOptions, LanguageModelV2StreamPart } from '@ai-sdk/provider'
  import { createOpenAI } from '@ai-sdk/openai'
  import { createAnthropic } from '@ai-sdk/anthropic'
  import { createGoogleGenerativeAI } from '@ai-sdk/google'
  import { createAmazonBedrock } from '@ai-sdk/amazon-bedrock'
  import { createOpenRouter } from '@openrouter/ai-sdk-provider'
  import { createOpenAICompatible } from '@ai-sdk/openai-compatible'
  ```
- [ ] 添加 Tauri 导入
  ```typescript
  import { invoke } from '@tauri-apps/api/core'
  import { Channel } from '@tauri-apps/api/core'
  ```
- [ ] 重写 RetryLanguageModel 类的 call 方法
- [ ] 重写 RetryLanguageModel 类的 callStream 方法
- [ ] 实现 createNativeStream 方法
- [ ] 实现 convertMessages 方法
- [ ] 删除 getLLM 方法（不再需要）
- [ ] 删除 streamWrapper 方法（不再需要）

### 阶段三：Agent模块重构

#### 任务 3.1：修改 `src/eko-core/agent/llm.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除这些导入
  import {
    LanguageModelV2FunctionTool,
    LanguageModelV2Prompt,
    LanguageModelV2StreamPart,
    LanguageModelV2TextPart,
    LanguageModelV2ToolCallPart,
    LanguageModelV2ToolChoice,
    LanguageModelV2ToolResultOutput,
    LanguageModelV2ToolResultPart,
    SharedV2ProviderOptions,
  } from '@ai-sdk/provider'
  ```
- [ ] 更新导入为原生类型
  ```typescript
  import {
    NativeLLMRequest,
    NativeLLMMessage,
    NativeLLMTool,
    NativeLLMStreamChunk,
    NativeLLMToolCall,
    NativeLLMMessagePart,
  } from '../types'
  ```
- [ ] 重写 defaultLLMProviderOptions 函数
- [ ] 重写 defaultMessageProviderOptions 函数
- [ ] 重写 convertTools 函数
- [ ] 重写 convertToolResult 函数
- [ ] 完全重写 callAgentLLM 函数
- [ ] 实现 appendUserConversation 函数

#### 任务 3.2：修改 `src/eko-core/agent/base.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除这些导入
  import {
    LanguageModelV2FilePart,
    LanguageModelV2Prompt,
    LanguageModelV2TextPart,
    LanguageModelV2ToolCallPart,
    LanguageModelV2ToolResultPart,
  } from '@ai-sdk/provider'
  ```
- [ ] 更新函数签名以使用原生类型
- [ ] 更新 runWithContext 方法中的类型引用
- [ ] 更新 handleCallResult 方法中的类型引用
- [ ] 更新 handleMessages 方法中的类型引用

#### 任务 3.3：修改 `src/eko-core/agent/context_compressor.ts`

- [ ] 更新 compressContext 方法中的消息类型
- [ ] 更新 buildCompressionSystemPrompt 方法
- [ ] 更新 compressMultipleResults 方法中的类型引用

### 阶段四：核心模块重构

#### 任务 4.1：修改 `src/eko-core/core/dialogue/llm.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2FilePart, LanguageModelV2StreamPart, LanguageModelV2ToolResultPart } from '@ai-sdk/provider'
  ```
- [ ] 更新导入为原生类型
- [ ] 重写 callChatLLM 函数
- [ ] 重写 convertToolResults 函数
- [ ] 重写 convertUserContent 函数
- [ ] 重写 convertAssistantToolResults 函数

#### 任务 4.2：修改 `src/eko-core/core/plan.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2Prompt, LanguageModelV2StreamPart, LanguageModelV2TextPart } from '@ai-sdk/provider'
  ```
- [ ] 更新 Planner 类中的类型引用
- [ ] 更新 plan 方法中的消息处理
- [ ] 更新 replan 方法中的消息处理

#### 任务 4.3：修改 `src/eko-core/core/dialogue.ts`

- [ ] 更新导入的类型引用
- [ ] 更新 EkoDialogue 类中的方法签名
- [ ] 更新 chat 方法中的类型处理
- [ ] 更新 handleCallResult 方法

#### 任务 4.4：修改 `src/eko-core/core/chain.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2ToolCallPart } from '@ai-sdk/provider'
  ```
- [ ] 更新 ToolChain 类中的类型引用

### 阶段五：内存和工具模块重构

#### 任务 5.1：修改 `src/eko-core/memory/index.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除这些导入
  import {
    LanguageModelV2FunctionTool,
    LanguageModelV2Prompt,
    LanguageModelV2TextPart,
    LanguageModelV2ToolCallPart,
  } from '@ai-sdk/provider'
  ```
- [ ] 更新 extractUsedTool 函数的类型参数
- [ ] 更新 compressAgentMessages 函数的类型参数
- [ ] 更新 doCompressAgentMessages 函数的实现

#### 任务 5.2：修改 `src/eko-core/memory/memory.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2Message } from '@ai-sdk/provider'
  ```
- [ ] 更新 EkoMemory 类中的类型引用
- [ ] 更新 buildMessages 方法的返回类型
- [ ] 更新消息转换逻辑

#### 任务 5.3：修改 `src/eko-core/tools/task_result_check.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2FunctionTool, LanguageModelV2Prompt } from '@ai-sdk/provider'
  ```
- [ ] 更新 doTaskResultCheck 函数的类型参数
- [ ] 更新工具调用逻辑

#### 任务 5.4：修改 `src/eko-core/tools/todo_list_manager.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2FunctionTool, LanguageModelV2Prompt } from '@ai-sdk/provider'
  ```
- [ ] 更新 doTodoListManager 函数的类型参数
- [ ] 更新工具调用逻辑

#### 任务 5.5：修改 `src/eko-core/tools/index.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2ToolCallPart } from '@ai-sdk/provider'
  ```
- [ ] 更新工具相关的类型引用

#### 任务 5.6：修改 `src/eko-core/tools/wrapper.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2FunctionTool, LanguageModelV2ToolCallPart } from '@ai-sdk/provider'
  ```
- [ ] 更新 ToolWrapper 类中的类型引用
- [ ] 更新 getTool 方法的返回类型
- [ ] 更新 callTool 方法的参数类型

#### 任务 5.7：修改 `src/eko-core/tools/human_interact.ts`

- [ ] 更新 LLMRequest 类型引用（已在类型文件中定义）
- [ ] 检查 execute 方法中的 LLM 调用逻辑
- [ ] 更新 checkLoginStatus 方法中的消息格式

#### 任务 5.8：修改 `src/eko-core/tools/watch_trigger.ts`

- [ ] 更新 LLMRequest 类型引用（已在类型文件中定义）
- [ ] 检查 is_dom_change 方法中的 LLM 调用逻辑
- [ ] 更新消息格式以适配原生后端

#### 任务 5.9：修改 `src/eko-core/core/dialogue/task_planner.ts`

- [ ] 保留 JSONSchema7 导入（这不是ai-sdk的依赖）
- [ ] 检查是否有其他ai-sdk相关的导入需要移除
- [ ] 确认 DialogueTool 接口的实现正确

#### 任务 5.10：修改 `src/eko-core/core/dialogue/execute_task.ts`

- [ ] 保留 JSONSchema7 导入（这不是ai-sdk的依赖）
- [ ] 检查是否有其他ai-sdk相关的导入需要移除
- [ ] 确认 DialogueTool 接口的实现正确

#### 任务 5.11：修改 `src/eko-core/tools/foreach_task.ts`

- [ ] 保留 JSONSchema7 导入（这不是ai-sdk的依赖）
- [ ] 检查是否有其他ai-sdk相关的导入需要移除
- [ ] 确认 Tool 接口的实现正确

#### 任务 5.12：修改 `src/eko-core/tools/task_node_status.ts`

- [ ] 保留 JSONSchema7 导入（这不是ai-sdk的依赖）
- [ ] 检查是否有其他ai-sdk相关的导入需要移除
- [ ] 确认 Tool 接口的实现正确

#### 任务 5.13：修改 `src/eko-core/common/utils.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2FunctionTool } from '@ai-sdk/provider'
  ```
- [ ] 更新 convertToolSchema 函数的返回类型
  ```typescript
  export function convertToolSchema(tool: ToolSchema): NativeLLMTool {
    if ('function' in tool) {
      return {
        name: tool.function.name,
        description: tool.function.description,
        parameters: tool.function.parameters,
      }
    } else if ('input_schema' in tool) {
      return {
        name: tool.name,
        description: tool.description,
        parameters: tool.input_schema,
      }
    } else if ('inputSchema' in tool) {
      return {
        name: tool.name,
        description: tool.description,
        parameters: tool.inputSchema,
      }
    } else {
      return {
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters,
      }
    }
  }
  ```
- [ ] 更新 mergeTools 函数的泛型约束
  ```typescript
  export function mergeTools<T extends Tool | NativeLLMTool>(tools1: T[], tools2: T[]): T[] {
    // 函数实现保持不变
  }
  ```

### 阶段六：其他文件修改

#### 任务 6.1：修改 `src/eko-core/index.ts`

- [ ] 检查并更新所有导出的类型引用
- [ ] 确保所有导出的类和函数使用正确的类型

#### 任务 6.1.1：修改 `src/eko-core/types/tools.types.ts`

- [ ] 移除 ai-sdk 导入
  ```typescript
  // 删除：import { LanguageModelV2ToolCallPart } from '@ai-sdk/provider'
  ```
- [ ] 更新 ToolExecuter 接口
  ```typescript
  export interface ToolExecuter {
    execute: (
      args: Record<string, unknown>,
      agentContext: AgentContext,
      toolCall: NativeLLMToolCall // 使用原生类型
    ) => Promise<ToolResult>
  }
  ```

#### 任务 6.2：创建原生LLM服务包装器

- [ ] 创建新文件 `src/eko-core/native/llm-service.ts`
- [ ] 实现 NativeLLMService 类
- [ ] 封装 Tauri 命令调用
- [ ] 提供统一的错误处理

#### 任务 6.3：创建类型转换工具

- [ ] 创建新文件 `src/eko-core/native/type-converters.ts`
- [ ] 实现 ai-sdk 类型到原生类型的转换函数
- [ ] 实现原生类型到 ai-sdk 类型的转换函数（如需要）

#### 任务 6.4：检查遗漏的文件

以下文件经检查不使用 ai-sdk，无需修改：

- [ ] `src/eko-core/config/index.ts` - 仅包含配置，无 ai-sdk 依赖
- [ ] `src/eko-core/config/prompt.config.ts` - 仅包含提示配置
- [ ] `src/eko-core/core/index.ts` - 仅导出类，无 ai-sdk 依赖
- [ ] `src/eko-core/core/eko.ts` - 核心逻辑，无直接 ai-sdk 依赖
- [ ] `src/eko-core/core/context.ts` - 上下文管理，使用已定义的类型别名
- [ ] `src/eko-core/agent/index.ts` - 仅导出类，无 ai-sdk 依赖
- [ ] `src/eko-core/memory/snapshot.ts` - 快照工具，无 ai-sdk 依赖
- [ ] `src/eko-core/mcp/` 目录下所有文件 - MCP 协议实现，无 ai-sdk 依赖
- [ ] `src/eko-core/prompt/` 目录下所有文件 - 提示模板，无 ai-sdk 依赖
- [ ] `src/eko-core/common/log.ts` - 日志工具，无 ai-sdk 依赖
- [ ] `src/eko-core/common/xml.ts` - XML 处理工具，无 ai-sdk 依赖
- [ ] `src/eko-core/types/mcp.types.ts` - MCP 类型定义，无 ai-sdk 依赖

### 阶段七：测试和验证

#### 任务 7.1：编译验证

- [ ] 确保所有 TypeScript 编译错误已解决
- [ ] 检查类型兼容性
- [ ] 验证导入导出正确性

#### 任务 7.2：功能测试

- [ ] 测试非流式LLM调用
- [ ] 测试流式LLM调用
- [ ] 测试工具调用功能
- [ ] 测试错误处理

#### 任务 7.3：性能验证

- [ ] 对比重构前后的性能
- [ ] 验证流式数据传输效率
- [ ] 检查内存使用情况

## 注意事项

1. **保持向后兼容性**：通过类型别名确保现有代码不会因类型更改而破坏
2. **错误处理**：确保原生后端的错误能正确传播到前端
3. **流式数据**：特别注意流式数据的正确处理和清理
4. **AbortSignal**：确保取消操作能正确传递到后端
5. **类型安全**：保持强类型检查，避免使用过多的 any 类型

#### 任务 7.4：验证后端接口兼容性

- [ ] 确认后端 Rust 类型与前端 TypeScript 类型的对应关系
- [ ] 验证 Tauri Channel 双向通信的正确实现
- [ ] 测试流式数据传输的性能和稳定性
- [ ] 确保错误处理机制的一致性

## 预期收益

1. **性能提升**：直接使用原生后端，减少中间层开销
2. **更好的控制**：完全控制LLM调用流程
3. **简化依赖**：移除对 ai-sdk 的依赖
4. **统一接口**：所有AI提供商通过统一的后端接口访问

## 补充说明

### 检查遗漏的文件

以下文件经检查不使用 ai-sdk，无需修改：

- `src/eko-core/config/index.ts` - 仅包含配置，无 ai-sdk 依赖
- `src/eko-core/config/prompt.config.ts` - 仅包含提示配置
- `src/eko-core/core/index.ts` - 仅导出类，无 ai-sdk 依赖
- `src/eko-core/core/eko.ts` - 核心逻辑，无直接 ai-sdk 依赖
- `src/eko-core/core/context.ts` - 上下文管理，使用已定义的类型别名
- `src/eko-core/agent/index.ts` - 仅导出类，无 ai-sdk 依赖
- `src/eko-core/memory/snapshot.ts` - 快照工具，无 ai-sdk 依赖
- `src/eko-core/mcp/` 目录下所有文件 - MCP 协议实现，无 ai-sdk 依赖
- `src/eko-core/prompt/` 目录下所有文件 - 提示模板，无 ai-sdk 依赖
- `src/eko-core/common/log.ts` - 日志工具，无 ai-sdk 依赖
- `src/eko-core/common/xml.ts` - XML 处理工具，无 ai-sdk 依赖
- `src/eko-core/types/mcp.types.ts` - MCP 类型定义，无 ai-sdk 依赖
