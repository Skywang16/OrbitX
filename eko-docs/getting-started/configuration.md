# 配置

如我们之前在[安装](./installation.md)中看到的，[`Eko`](/eko/docs/api/classes/Eko.html) 接受一个 [`EkoConfig`](/eko/docs/api/classes/Eko.html) 类型的参数，定义如下：

```typescript
type EkoConfig = {
  llms: LLMs
  agents?: Agent[]
  planLlms?: string[]
  callback?: StreamCallback & HumanCallback
  defaultMcpClient?: IMcpClient
  a2aClient?: IA2aClient
}
```

本指南将引导您配置这些参数。您也可以查看 [`EkoConfig` 参考文档](/eko/docs/api/types/EkoConfig.html) 了解代码详情。

## EkoConfig.llms

`EkoConfig.llms` 以键值对格式存储 Eko 可用的大型模型，您可以为每个模型设置以下内容：

- 提供商
- 模型名称
- API 密钥
- 基础 URL
- TopK
- TopP

这里定义的模型将被规划器和代理使用。您可以配置不同性能级别的模型，以满足不同组件的不同需求。

以下是配置两个模型的示例，Claude 3.5 作为默认模型，另一个是 GPT-4o：

```typescript
let llms = {
  default: {
    provider: 'anthropic',
    model: 'claude-3-5-sonnet-20241022',
    apiKey: 'sk-xxx',
    config: { topK: 5 },
  },
  openai: {
    provider: 'openai',
    model: 'gpt-4o',
    apiKey: 'sk-xxx',
    config: { baseURL: 'https://example.com/v1', topP: 0.7 },
  },
}
```

## EkoConfig.agents

`EkoConfig.agents` 描述了 Eko 工作流中可用的代理。每个代理都有自己的名称、描述、工具包和可用模型。

有关代理的更多信息，请参阅[代理](../agents/overview.md)部分。

在这个简单示例中，只有一个 `BrowserAgent`：

```typescript
import { BrowserAgent } from '@eko-ai/eko-extension'
let agents: Agent[] = [new BrowserAgent()]
```

## EkoConfig.planLlms

`EkoConfig.planLlms` 指定将被 Eko 规划器使用的模型。建议选择高性能模型，因为它将负责规划整个工作流。例如：

```typescript
let llms = {
  default: {
    provider: 'openai',
    model: 'gpt-4o-mini',
    apiKey: 'sk-xxx',
  },
  powerful: {
    provider: 'anthropic',
    model: 'claude-3-7-sonnet',
    apiKey: 'sk-xxx',
  },
}
let eko = new Eko({ llms, planLlms: ['powerful'] })
```

## EkoConfig.callback

`EkoConfig.callback` 接受一组用户定义的回调函数，可以包括 `StreamCallback` 和 `HumanCallback`。

- `StreamCallback` 允许您接收关于工作流进度、工具使用和结果的流式更新。
- `HumanCallback` 使您能够处理需要人工输入或确认的情况（例如，当用户选择选项、确认操作或提供输入时）。

有关更多信息，请参阅[回调系统](../architecture/callback-system.md)部分。

示例：

```typescript
let callback = {
  onMessage: async msg => {
    /* 处理流式更新 */
  },
  onHumanInput: async (ctx, prompt) => {
    /* 提示用户输入 */ return '用户输入'
  },
}
```

## EkoConfig.defaultMcpClient

`EkoConfig.defaultMcpClient` 是用于处理与 MCP（多组件平台）后端通信的 `IMcpClient` 实例。如果您的代理/工具需要后端编排或状态管理，请设置此项。

## EkoConfig.a2aClient

`EkoConfig.a2aClient` 是用于代理间通信的 `IA2aClient` 实例。如果您的工作流涉及多个代理之间的协调或消息传递，请使用此项。
