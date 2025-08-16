# 回调系统

## 理解 Eko 中的回调

在开发 AI 驱动的自动化系统时，开发者面临一个关键挑战：在保持透明度和控制的同时确保效率。Eko 的钩子通过弥合 AI 自动化和人工监督之间的差距来解决这个问题。它们充当工作流中的战略检查点，允许开发者：

- 监控 AI 的决策过程。
- 收集关于自动化性能的指标。

通过特殊的回调 - _钩子_，开发者甚至可以：

- 在输入被处理之前验证或修改输入。
- 在输出被使用之前调整输出。
- 在必要时进行干预，同时让自动化处理常规工作。

Eko 中的回调对于维护系统的可观察性、可控性和可靠性至关重要，确保高效、透明和安全的操作。

## 回调域

Eko 的回调系统分为两个主要域：

1. **流回调**：用于监控、日志记录和 UI 更新。
2. **人工回调**：暂停并请求用户输入或确认。

```typescript
import { Eko, LLMs, StreamCallbackMessage } from '@eko-ai/eko'
import { StreamCallback, HumanCallback } from '@eko-ai/eko/types'

let callback: StreamCallback & HumanCallback = {
  onMessage: async (message: StreamCallbackMessage) => {
    /* 监控模型实时事件和工具使用参数，修改工具结果*/
  },
  onHumanConfirm: async (context, prompt) => {
    /* 向用户显示提示，用户将确认或拒绝*/
    return /* 如果确认则为 true，如果拒绝则为 false*/
  },
}

let eko = new Eko({ llms, agents, callback })
```

### 流回调

流回调提供关于工作流执行的实时更新。它们用于监控、日志记录和 UI 更新。

**可用的流回调（`StreamCallbackMessage` 类型和时机）：**

- `workflow`: 当工作流正在生成或更新时发出。
- `text`: 为来自代理或 LLM 的流式文本输出发出。
- `thinking`: 为流式中间推理或思考发出。
- `tool_streaming`: 为流式工具调用发出。
- `tool_use`: 在工具执行之前发出，包括工具名称和参数。
- `tool_running`: 在工具运行时发出，显示工具运行的详细信息。
- `tool_result`: 在工具完成执行后发出，包括结果。
- `file`: 当文件作为输出产生时发出。
- `error`: 当执行过程中发生错误时发出。
- `finish`: 当工作流或节点执行完成时发出。

查看 [`StreamCallback`](/eko/docs/api/interfaces/StreamCallback.html) 和 [`StreamCallbackMessage`](/eko/docs/api/types/StreamCallbackMessage.html) 了解完整的类型定义。

#### 使用方法

```typescript
import { Eko, LLMs, StreamCallbackMessage } from '@eko-ai/eko'
import { StreamCallback, HumanCallback } from '@eko-ai/eko/types'

let callback: StreamCallback & HumanCallback = {
  onMessage: async (message: StreamCallbackMessage) => {
    // 等待流完成并做一些重要的事情...
    if (message.streamDone) {
      switch (message.type) {
        case 'workflow':
          // 当工作流正在生成或更新时发出。
          break
        case 'text':
          // 为来自代理或 LLM 的流式文本输出发出。
          break
        case 'tool_streaming':
          // 为流式工具调用发出。
          break
        case 'tool_use':
          // 在工具执行之前发出，包括工具名称和参数。
          break
        case 'tool_result':
          // 在工具完成执行后发出，包括结果。
          break
        case '...':
          // 其他事件。
          break
      }
    }
  },
}

let eko = new Eko({ llms, agents, callback })
```

#### 干预模型调用的工具的输入和输出

```typescript
// navigate_to 工具定义
{
  name: "navigate_to",
  description: "导航到特定 url",
  parameters: {
    type: "object",
    properties: {
      url: {
        type: "string",
        description: "要导航到的 url",
      },
    },
    required: ["url"],
  }
}

// 干预模型调用的工具的输入和输出
let callback: StreamCallback = {
  onMessage: async (message: StreamCallbackMessage) => {
    switch(message.type) {
      case "tool_use":
        // 干预输入
        if (message.agentName == "Browser" && message.toolName == "navigate_to") {
          // 修改 navigate_to 工具的返回结果，在这种情况下，将 "twitter" 更改为 "x"。
          // message.params.url 中的 URL 参数在 navigate_to 工具的 `properties` 参数中定义。
          if (message.params.url == "https://twitter.com") {
            message.params.url = "https://x.com";
          }
        }
        break;
      case "tool_result":
        // 干预输出
        if (message.agentName == "File" && message.toolName == "file_read") {
          if (message.params.path == "/account.md") {
            let content = message.toolResult.content;
            if (content[0].type == "text") {
              // 通过干预输出过滤敏感信息。
              content[0].text = content[0].text.replace("这是密码", "********");
            }
          }
        }
        break;
    }
  }
};
```

### 人工回调

人工回调启用人在环路交互，允许工作流暂停并请求用户输入或确认。

**可用的人工回调及其时机：**

- `onHumanConfirm(context, prompt)`: 当工作流需要确认潜在危险或重要操作时调用（例如，删除文件）。
- `onHumanInput(context, prompt)`: 当工作流需要自由形式的用户输入时调用（例如，输入邮件标题或填写表单）。
- `onHumanSelect(context, prompt, options, multiple)`: 当工作流需要用户从选项列表中选择时调用（单选或多选）。
- `onHumanHelp(context, helpType, prompt)`: 当工作流请求特定帮助类型的人工协助时调用（例如，登录、故障排除）。

查看 [`HumanCallback`](/eko/docs/api/interfaces/HumanCallback.html) 了解完整接口。

#### HumanInteractTool

人工回调由 HumanInteractTool 触发，提示定义如下：

```
AI 与人类交互：
confirm: 要求用户确认是否执行操作，特别是在执行危险操作（如删除系统文件）时。
input: 提示用户输入文本；例如，当任务模糊时，AI 可以选择询问用户详细信息，用户可以通过输入来回应。
select: 允许用户做出选择；在需要选择的情况下，AI 可以要求用户做出决定。
request_help: 请求用户协助；例如，当操作被阻止时，AI 可以要求用户帮助，如需要登录网站或解决验证码。
```

#### 使用方法

```typescript
import { Eko, LLMs, StreamCallbackMessage } from '@eko-ai/eko'
import { StreamCallback, HumanCallback } from '@eko-ai/eko/types'

let callback: StreamCallback & HumanCallback = {
  onHumanConfirm: async (message: StreamCallbackMessage) => {
    // 等待流完成并做一些重要的事情...
    return confirm(prompt)
  },
  onHumanInput: async (message: StreamCallbackMessage) => {
    // 等待流完成并做一些重要的事情...
  },
  onHumanSelect: async (message: StreamCallbackMessage) => {
    // 等待流完成并做一些重要的事情...
  },
  onHumanHelp: async (message: StreamCallbackMessage) => {
    // 等待流完成并做一些重要的事情...
  },
}

let eko = new Eko({ llms, agents, callback })
```

## 回调使用案例

此示例使用 `onMessage` 将工作流、大型语言模型响应和工具调用参数记录到日志中，还实现了 `onHumanConfirm` 接口以允许用户确认某些操作：

```typescript
import { Eko, LLMs, StreamCallbackMessage } from '@eko-ai/eko'
import { StreamCallback, HumanCallback } from '@eko-ai/eko/types'

function printLog(message: string, level?: 'info' | 'success' | 'error') {
  /* 例如 console.log(message); */
}

let callback: StreamCallback & HumanCallback = {
  onMessage: async (message: StreamCallbackMessage) => {
    if (message.type == 'workflow' && message.streamDone) {
      printLog('计划\n' + message.workflow.xml)
    } else if (message.type == 'text' && message.streamDone) {
      printLog(message.text)
    } else if (message.type == 'tool_use') {
      printLog(`${message.agentName} > ${message.toolName}\n${JSON.stringify(message.params)}`)
    }
    console.log('消息: ', JSON.stringify(message, null, 2))
  },
  onHumanConfirm: async (context, prompt) => {
    /* 向用户显示提示，用户将确认或拒绝*/
    return /* 如果确认则为 true，如果拒绝则为 false*/
  },
}

let eko = new Eko({ llms, agents, callback })
```

## 回调类型

根据通信方式，回调可以分为两类：

- **单向**：只能读取值，适用于日志记录和监控场景（例如，关于工作流进度、工具使用和结果的流式更新）。
- **双向（又称钩子）**：既可以读取也可以修改值，可以理解为中间件，适用于高度定制的场景（例如，拦截和修改工具输入/输出，或工作流节点执行）。

根据调用方式，回调可以分为两类：

- **一次调用**：在适当时间只调用一次的常规回调，如 `tool_result`。
- **流式调用**：在一段时间内多次调用，每次返回该时刻可用的完整数据，如 `workflow`。
