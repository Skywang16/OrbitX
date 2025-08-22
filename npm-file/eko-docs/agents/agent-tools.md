# 代理工具

## 什么是代理工具？

在 Eko 框架中，工具是代理的核心能力，是真正执行调用的组件，代理必须包含一个或多个工具才能完成其工作。

## 如何在代理中自定义工具？

每个工具需要实现以下核心元素：

- `name`: 工具的唯一标识符
- `description`: 工具的功能描述
- `parameters`: 输入参数结构定义
- `execute`: 具体执行逻辑

要在 Eko 代理中创建自定义工具，您需要遵循以下步骤：

1. **定义输入和输出结构**: 明确指定工具的输入参数和输出结果。
2. **实现工具接口**: 继承或实现 `Tool` 接口并定义相关属性和方法。
3. **处理参数**: 在 `execute` 方法中编写特定功能的实现逻辑。

```typescript
interface Tool {
  name: string // 工具名称
  description: string // 功能描述，解释工具的目的和使用场景
  parameters: InputSchema // 功能描述，解释工具的目的和使用场景
  execute: (
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: LanguageModelV1ToolCallPart
  ) => Promise<ToolResult> // 执行函数
}
```

### 创建代理工具

让我们通过一个简单的示例来解释如何创建自定义工具。

假设我们想创建一个发送邮件的工具：

```typescript
import { AgentContext } from '@eko-ai/eko'
import { Tool, ToolResult, LanguageModelV1ToolCallPart } from '@eko-ai/eko/types'

class SendEmail implements Tool {
  name: string
  description: string
  parameters: any

  constructor() {
    this.name = 'send_email'
    this.description = '向指定收件人发送邮件'
    this.parameters = {
      type: 'object',
      properties: {
        to: {
          type: 'string',
          description: '收件人的邮箱地址',
        },
        subject: {
          type: 'string',
          description: '邮件主题',
        },
        content: {
          type: 'string',
          description: '邮件内容',
        },
      },
      required: ['to', 'subject', 'content'],
    }
  }

  async execute(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: LanguageModelV1ToolCallPart
  ): Promise<ToolResult> {
    // 实现具体的邮件发送逻辑
    // 这只是一个示例
    let emailService: any
    const result = await emailService.send({
      to: args.to,
      subject: args.subject,
      content: args.content,
    })
    return {
      content: [{ type: 'text', text: JSON.stringify({ messageId: result.id }) }],
    }
  }
}
```

## 高级功能

### 会话状态（变量）

在工具执行函数中，支持跨工具或跨代理读取或保存变量信息。

```typescript
async execute(
  args: Record<string, unknown>,
  agentContext: AgentContext,
  toolCall: LanguageModelV1ToolCallPart
): Promise<ToolResult> {
  // 示例：读取和写入变量（在当前代理执行过程中的当前任务期间有效，跨工具）
  let id1 = agentContext.variables.get("id1");
  if (!id1) {
    agentContext.variables.set("id1", "xxxx");
  }

  // 示例：读取和写入变量（生命周期在当前任务执行过程中有效，跨代理）
  let id2 = agentContext.context.variables.get("id2");
  if (!id2) {
    agentContext.context.variables.set("id2", "xxxx");
  }

  return {
    content: [
      { type: "text", text: "成功" },
    ],
  };
}
```

### 回调 `tool_running` 事件

支持工具调用期间的回调事件。例如，一些工具执行时间较长，在执行过程中生成日志或流式输出数据，需要通过回调实时公开，如 Python 代码执行工具生成的实时日志。

```typescript
async execute(
  args: Record<string, unknown>,
  agentContext: AgentContext,
  toolCall: LanguageModelV1ToolCallPart
): Promise<ToolResult> {
  // 模拟工具执行期间生成的实时日志
  const logs = ["模拟日志1", "模拟日志2", "模拟日志3"];
  // 回调函数
  const callback = agentContext.context.config.callback || {
    onMessage: async (message: StreamCallbackMessage) => {},
  };
  // 流式消息 ID
  const streamId = uuidv4();
  for (let i = 0; i < logs.length; i++) {
    let log = logs[i];
    // 回调流式 tool_running 消息
    await callback.onMessage({
      taskId: agentContext.context.taskId,
      agentName: agentContext.agent.Name,
      nodeId: agentContext.agentChain.agent.id,
      type: "tool_running",
      toolName: toolCall.toolName,
      toolId: toolCall.toolCallId,
      text: log,
      streamId: streamId,
      streamDone: false,
    });
  }
  // 最终 tool_running 消息
  await callback.onMessage({
    taskId: agentContext.context.taskId,
    agentName: agentContext.agent.Name,
    nodeId: agentContext.agentChain.agent.id,
    type: "tool_running",
    toolName: toolCall.toolName,
    toolId: toolCall.toolCallId,
    text: logs.join("\n"),
    streamId: streamId,
    streamDone: true,
  });
  return {
    content: [{ type: "text", text: "成功" }],
  };
}
```

具体钩子描述请参考：[钩子系统](../architecture/callback-system.md)。
