# 多代理概述

## 什么是代理？

在 Eko 框架中，代理是核心驱动器。通常，一个代理由多个工具组成，每个代理都有其独立的功能，例如：

- 浏览器代理
- 计算机代理
- 文件代理
- Shell 代理
- 自定义代理

## 为什么使用代理

### 1. 模块化设计

代理是独立的模块，每个代理负责完成特定的功能。这种设计允许开发者将复杂的流程分解为简单、可管理的步骤。通过这种模块化方法，开发者可以在不同项目中重用这些代理，减少代码重复并提高开发效率。

### 2. 清晰的接口定义

每个代理都遵循统一的接口结构，包括 `name`、`description`、`tools`，并支持 mcp 扩展和不同 LLM 模型的规范。

```typescript
class Agent {
  name: string
  description: string
  tools: Tool[] = []
  llms?: string[]
  mcpClient?: IMcpClient
}
```

• name: 代理的唯一标识符（例如 `Browser`）

• description: 功能描述，解释代理的角色和使用场景

• tools: 具体的执行工具，所有操作都由工具完成

• llms: 指定使用的模型，支持回退调用，默认模型

• mcpClient: 支持通过 MCP 接口动态扩展工具

## 在 Eko 中使用

Eko 框架为不同环境提供了各种内置代理，可以直接使用，您也可以自定义代理来完成工作流任务。

### 内置代理

以下是 node.js 环境的演示

```typescript
import { Eko } from '@eko-ai/eko'
import { BrowserAgent, FileAgent } from '@eko-ai/eko-nodejs'

let eko = new Eko({
  llms: {
    default: {
      provider: 'anthropic',
      model: 'claude-3-7-sonnet',
      apiKey: 'your_api_key',
    },
  },
  agents: [new BrowserAgent(), new FileAgent()],
})

let result = await eko.run(`
  搜索关于马斯克的最新新闻，总结并保存为 musk_news.md 文件到桌面。
`)
```

了解更多：[可用代理](available-agent.md)。

### 自定义代理

```typescript
import { Eko, Agent, AgentContext } from '@eko-ai/eko'
import { ToolResult } from '@eko-ai/eko/types'

let weather_agent = new Agent({
  name: 'Weather',
  description: '提供天气查询服务',
  tools: [
    {
      name: 'get_weather',
      description: '天气查询',
      parameters: {
        type: 'object',
        properties: {
          city: {
            type: 'string',
          },
        },
      },
      execute: async (args: Record<string, unknown>, agentContext: AgentContext): Promise<ToolResult> => {
        return {
          content: [
            {
              type: 'text',
              text: `今天，${args.city}的天气是多云，25-30°（摄氏度），适合外出散步。`,
            },
          ],
        }
      },
    },
  ],
})

let eko = new Eko({
  llms: {
    default: {
      provider: 'anthropic',
      model: 'claude-3-7-sonnet',
      apiKey: 'your_api_key',
    },
  },
  agents: [weather_agent],
})

let result = await eko.run(`
  北京今天的天气怎么样？
`)
```

了解更多：[自定义代理](custom-agent.md)。

## 下一步

现在您已经了解了代理的概念，让我们看看内置代理以及如何自定义代理：

- 框架在不同环境中的内置[可用代理](available-agent.md)
- 学习如何[自定义代理](custom-agent.md)
- 学习如何[代理工具](agent-tools.md)
