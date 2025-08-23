# 自定义代理

## 什么是自定义代理？

在 Eko 框架中，代理是实现不同功能的核心概念组件。内置代理提供一些基本功能，如用于浏览器操作的 BrowserAgent。然而，实际应用场景各不相同，不同的应用可能需要特定功能来满足其需求。这就是自定义代理发挥作用的地方。

自定义代理允许开发者根据特定的业务逻辑或应用需求设计和实现自己的功能模块，而不依赖于内置代理。通过自定义代理，开发者可以扩展 Eko 框架的功能并增强其灵活性和适应性。

## 为什么需要自定义代理？

自定义代理帮助开发者解决以下问题：

- 当现有代理无法满足特定需求时，可以创建自定义代理来实现它们。
- 开发者可以根据业务逻辑的复杂性在自定义代理下扩展工具以满足应用需求。
- 将重复功能模块化，便于管理和调试。
- 调用内部公司服务并集成特定的业务逻辑。

代理定义如下：

```typescript
class Agent {
  protected name: string
  protected description: string
  protected tools: Tool[] = []
  protected llms?: string[]
  protected mcpClient?: IMcpClient
  protected planDescription?: string
}
```

## 如何在 Eko 中自定义代理？

支持以下两种方法来自定义代理，以文件代理为例：

1. 直接 `new Agent({ name, description, tools })` 方法

```typescript
import { Agent } from '@eko-ai/eko'

let fileAgent = new Agent({
  name: 'File',
  description: '您是一个文件代理，处理文件相关任务，如创建、查找、读取、修改文件等。',
  tools: [],
})
```

2. 定义一个代理类，然后扩展 Agent 方法

```typescript
import { Agent } from '@eko-ai/eko'

class FileAgent extends Agent {
  constructor() {
    super({
      name: 'File',
      description: '您是一个文件代理，处理文件相关任务，如创建、查找、读取、修改文件等。',
      tools: [],
    })
  }
}

let fileAgent = new FileAgent()
```

完整的示例代码如下：

```typescript
import { Agent, AgentContext } from '@eko-ai/eko'
import { Tool, ToolResult } from '@eko-ai/eko/types'

// 定义代理工具
let tools: Tool[] = [
  {
    name: 'file_read',
    description: '读取文件内容。',
    parameters: {
      type: 'object',
      properties: {
        path: {
          type: 'string',
          description: '文件路径',
        },
      },
      required: ['path'],
    },
    execute: async (args: Record<string, unknown>, agentContext: AgentContext): Promise<ToolResult> => {
      // TODO 具体读取逻辑
      let file_content = '文件内容...'
      return {
        isError: false,
        content: [{ type: 'text', text: file_content }],
      }
    },
  },
  {
    name: 'file_write',
    description: '覆盖或追加内容到文件。',
    parameters: {
      type: 'object',
      properties: {
        path: {
          type: 'string',
          description: '文件路径',
        },
        content: {
          type: 'string',
          description: '文本内容',
        },
        append: {
          type: 'boolean',
          description: '（可选）是否使用追加模式',
          default: false,
        },
      },
      required: ['path', 'content'],
    },
    execute: async (args: Record<string, unknown>, agentContext: AgentContext): Promise<ToolResult> => {
      // TODO 具体写入逻辑
      return {
        isError: false,
        content: [{ type: 'text', text: '写入成功。' }],
      }
    },
  },
]

let fileAgent: Agent

// 方法一
fileAgent = new Agent({
  name: 'File',
  description: '您是一个文件代理，支持读取和写入文件。',
  tools: tools,
})

// 方法二
class FileAgent extends Agent {
  constructor() {
    super({
      name: 'File',
      description: '您是一个文件代理，处理文件相关任务，如创建、查找、读取、修改文件等。',
      tools: tools,
    })
  }
}

fileAgent = new FileAgent()
```

## 高级功能

代理支持一些高级扩展功能。

### 多模型回退调用

默认情况下，代理遵循默认模型配置，允许设置多个模型配置作为回退。当模型 1 无法访问时，将回退到模型 2，然后是模型 3，依此类推。

```typescript
import { Eko, Agent, LLMs } from '@eko-ai/eko'

// 多模型配置
let llms: LLMs = {
  default: {
    provider: 'anthropic',
    model: 'claude-3-7-sonnet',
    apiKey: 'your_api_key',
  },
  model1: {
    provider: 'openai',
    model: 'gpt-mini-4o',
    apiKey: 'your_api_key',
  },
  model2: {
    provider: 'anthropic',
    model: 'claude-3-5-sonnet',
    apiKey: 'your_api_key',
  },
  model3: {
    provider: 'openrouter',
    model: 'openai/gpt-mini-4o',
    apiKey: 'your_api_key',
  },
}

// 自定义文件代理
let fileAgent = new Agent({
  name: 'File',
  description: '您是一个文件代理，支持读取和写入文件。',
  tools: tools,
  llms: ['model1', 'model2', 'model3'], // 模型链回退
})

let eko = new Eko({
  llms: llms,
  agents: [fileAgent],
})

// 运行
let result = await eko.run('读取桌面 test.txt 文件')
```

## 下一步

现在您已经了解了自定义代理的概念，让我们看看如何定义工具：

了解更多：[代理工具](agent-tools.md)。

了解更多：[MCP 工具](mcp-tools.md)。
