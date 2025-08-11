# AI聊天数据结构设计文档

## 📋 概述

本文档描述了AI聊天系统的数据结构设计，包括消息存储、步骤管理和UI渲染的完整流程。

## 🏗️ 核心数据结构

### 1. Message 接口

```typescript
interface Message {
  // === 基础字段 ===
  id: number // 消息唯一ID
  conversationId: number // 所属会话ID
  role: 'user' | 'assistant' | 'system' // 消息角色
  createdAt: Date // 创建时间

  // === AI消息扩展字段 ===
  steps?: AIOutputStep[] // AI输出的所有步骤（核心字段）
  status?: 'pending' | 'streaming' | 'complete' | 'error' // 消息状态
  duration?: number // 总耗时（毫秒）

  // === 兼容字段（用户消息需要）===
  content?: string // 用户消息内容，AI消息从steps中获取
}
```

### 2. AIOutputStep 步骤接口

```typescript
interface AIOutputStep {
  // === 基础字段 ===
  type: 'thinking' | 'workflow' | 'text' | 'tool_use' | 'tool_result' | 'error'
  content: string // 步骤内容
  timestamp: number // 步骤时间戳

  // === 元数据（根据type不同而不同）===
  metadata?: {
    // 思考阶段
    thinkingDuration?: number // 思考持续时间（毫秒）

    // 工具调用
    toolName?: string // 工具名称
    toolParams?: Record<string, any> // 工具参数
    toolResult?: any // 工具执行结果

    // 工作流
    workflowName?: string // 工作流名称
    agentName?: string // Agent名称
    taskId?: string // 任务ID

    // 错误信息
    errorType?: string // 错误类型
    errorDetails?: string // 错误详情
  }
}
```

## 数据流程

### 1. 消息创建流程

```typescript
// 1. 用户发送消息后，创建临时AI消息
const tempMessage: Message = {
  id: Date.now(), // 临时ID
  conversationId: currentId,
  role: 'assistant',
  createdAt: new Date(),
  steps: [], // 空步骤数组
  status: 'streaming', // 流式状态
}
```

### 2. 流式更新流程

```typescript
// 2. Eko回调函数接收流式数据
onMessage: async message => {
  if (message.type === 'tool_use') {
    // 处理工具调用
    tempMessage.steps?.push({
      type: 'tool_use',
      content: message.tool?.description || '正在调用工具...',
      timestamp: Date.now(),
      metadata: {
        toolName: message.tool?.name || '未知工具',
        toolParams: message.tool?.parameters,
      },
    })
  } else if (message.type === 'tool_result') {
    // 处理工具结果
    tempMessage.steps?.push({
      type: 'tool_result',
      content: message.result || '工具执行完成',
      timestamp: Date.now(),
      metadata: {
        toolName: message.tool?.name || '未知工具',
        toolResult: message.result,
      },
    })
  } else if (message.type === 'workflow' && message.workflow?.thought) {
    // 处理思考步骤
    let thinkingStep = tempMessage.steps?.find(step => step.type === 'thinking')
    if (thinkingStep) {
      thinkingStep.content = message.workflow.thought

      // 如果thinking完成，记录持续时间
      if (message.streamDone) {
        thinkingStep.metadata = {
          ...thinkingStep.metadata,
          thinkingDuration: Date.now() - thinkingStep.timestamp,
        }
      }
    } else {
      const newStep = {
        type: 'thinking',
        content: message.workflow.thought,
        timestamp: Date.now(),
        metadata: {
          workflowName: message.workflow.name,
          agentName: message.agentName,
          taskId: message.taskId,
        },
      }

      // 如果thinking瞬间完成，记录0持续时间
      if (message.streamDone) {
        newStep.metadata.thinkingDuration = 0
      }

      tempMessage.steps?.push(newStep)
    }
  } else if (message.type === 'text' && !message.streamDone) {
    // 更新或添加文本步骤
    const textStep = tempMessage.steps?.find(s => s.type === 'text')
    if (textStep) {
      textStep.content = message.text // 更新现有步骤
    } else {
      tempMessage.steps?.push({
        // 添加新步骤
        type: 'text',
        content: message.text,
        timestamp: Date.now(),
      })
    }
  }
}
```

### 3. 完成和保存流程

```typescript
// 3. AI输出完成后
tempMessage.status = 'complete'
tempMessage.duration = Date.now() - startTime

// 4. 保存到数据库（包含完整的steps数组）
await conversationAPI.saveMessage(conversationId, 'assistant', {
  content: tempMessage.content,
  steps: tempMessage.steps,
  status: tempMessage.status,
  duration: tempMessage.duration,
})
```

## 🎨 UI渲染逻辑

### AIMessage.vue 组件结构

```vue
<template>
  <div class="ai-message">
    <!-- 遍历所有步骤进行渲染 -->
    <template v-for="step in message.steps" :key="step.timestamp">
      <!-- 思考块：可折叠，带计时器 -->
      <ThinkingBlock
        v-if="step.type === 'thinking'"
        :thinking="step.content"
        :start-time="step.timestamp"
        :duration="step.metadata?.thinkingDuration"
      />

      <!-- 文本内容：主要AI回复（支持Markdown渲染） -->
      <div v-else-if="step.type === 'text'" class="ai-message-text">
        <div v-html="renderMarkdown(step.content)"></div>
      </div>

      <!-- 工具调用：显示工具名称和参数 -->
      <div v-else-if="step.type === 'tool_use'" class="tool-use-block">
        <div class="tool-header">🛠️ {{ step.metadata?.toolName }}</div>
        <div class="tool-params">{{ JSON.stringify(step.metadata?.toolParams, null, 2) }}</div>
      </div>

      <!-- 工具结果：显示执行结果 -->
      <div v-else-if="step.type === 'tool_result'" class="tool-result-block">
        <div class="tool-header">✅ {{ step.metadata?.toolName }} 结果</div>
        <div class="tool-result">{{ step.content }}</div>
      </div>
    </template>

    <!-- 兜底渲染：如果没有steps但有content -->
    <div v-else-if="message.content" class="ai-message-text">
      <div v-html="renderMarkdown(message.content)"></div>
    </div>

    <div class="ai-message-time">{{ formatTime(message.createdAt) }}</div>
  </div>
</template>
```

## 📊 数据示例

### 完整对话示例

```javascript
{
  // === 基础消息字段 ===
  "id": 1002,                                    // 消息唯一ID，数据库主键
  "conversationId": 123,                         // 所属会话ID，关联到conversations表
  "role": "assistant",                           // 消息角色：user/assistant/system
  "createdAt": "2024-01-15T10:30:05Z",          // 消息创建时间

  // === AI消息状态字段 ===
  "status": "complete",                          // 消息状态：pending/streaming/complete/error
  "duration": 3500,                             // AI处理总耗时（毫秒）

  // === 核心步骤数组 - 存储AI执行的完整过程 ===
  "steps": [
    {
      // 第1步：AI思考阶段
      "type": "thinking",                        // 步骤类型：thinking（思考过程）
      "content": "用户询问当前目录，需要执行pwd命令获取路径", // 思考内容（流式更新）
      "timestamp": 1705315805000,               // 步骤开始时间戳
      "metadata": {                             // 思考阶段的元数据
        "thinkingDuration": 1200,               // 思考持续时间（毫秒）
        "workflowName": "查询当前目录",          // 工作流名称
        "agentName": "Planer"                   // 执行的Agent名称
      }
    },
    {
      // 第2步：工具调用
      "type": "tool_use",                       // 步骤类型：tool_use（工具调用）
      "content": "执行shell命令",                // 工具调用描述
      "timestamp": 1705315806200,               // 工具调用时间戳
      "metadata": {                             // 工具调用的元数据
        "toolName": "shell",                    // 调用的工具名称
        "toolParams": {                         // 工具调用参数
          "command": "pwd"                      // 具体的shell命令
        }
      }
    },
    {
      // 第3步：工具执行结果
      "type": "tool_result",                    // 步骤类型：tool_result（工具结果）
      "content": "/Users/username/project",     // 工具返回的结果内容
      "timestamp": 1705315806800,               // 工具完成时间戳
      "metadata": {                             // 工具结果的元数据
        "toolName": "shell",                    // 对应的工具名称
        "toolResult": {                         // 详细的工具执行结果
          "stdout": "/Users/username/project",  // 标准输出
          "stderr": "",                         // 标准错误（空表示无错误）
          "exitCode": 0                         // 退出码（0表示成功）
        }
      }
    },
    {
      // 第4步：最终AI回复
      "type": "text",                           // 步骤类型：text（最终文本回复）
      "content": "当前目录是 /Users/username/project", // AI的最终回复内容
      "timestamp": 1705315808500,               // 回复生成时间戳
      "metadata": {}                            // 文本步骤通常无额外元数据
    }
  ],

  // 注意：
  // 1. 通过 status 字段判断消息状态：'streaming' = 正在渲染，'complete' = 渲染完成
  // 2. 通过 steps.length 可以知道当前有多少个步骤
  // 3. AI消息的最终内容从 steps 数组中的 text 类型步骤获取
}
```

## ✅ 设计优势

1. **完整性** - 保存AI对话的完整执行过程
2. **可重现** - 从数据库恢复后UI效果完全一致
3. **结构化** - 每个步骤都有明确类型和元数据
4. **扩展性** - 容易添加新的步骤类型
5. **时序性** - 通过timestamp保持正确的执行顺序
6. **流式友好** - 支持实时更新和渲染

## 🔧 技术实现要点

1. **步骤去重** - 同类型步骤会被更新而不是重复添加
2. **状态管理** - 通过 `status` 字段跟踪消息生命周期
3. **时间控制** - thinking步骤支持计时器停止和持续时间记录
4. **工具追踪** - 完整记录工具调用和执行结果
5. **Markdown渲染** - 文本内容支持Markdown语法解析和渲染
6. **元数据丰富** - 每个步骤都包含丰富的上下文信息

这种设计确保了AI对话数据的完整性和可重现性，同时支持复杂的流式渲染需求。
