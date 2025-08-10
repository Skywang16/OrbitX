# 架构概述

Eko 是一个专为构建生产就绪的代理工作流而设计的框架。它为自动化工作流的规划和执行提供了高效的跨平台解决方案。此外，Eko 提供高度可定制的接口，使开发者能够自由设计工作流，确保满足生产级要求。

Eko 是一个多代理工作流框架，通过工作流规划使多个代理能够协作，提供生产就绪的能力。

用户输入提示后，规划器将设计工作流，Eko 将根据工作流协调不同的代理执行各种任务，最终将结果返回给用户。

![架构图](https://fellou.ai/eko/docs/_astro/architecture-new-placeholder.iZRdPanV_Z39vQo.webp)

## 核心概念

### 工作流

工作流是基于 XML 的 DSL，旨在准确高效地完成复杂任务。例如，如果用户输入提示 `打开 Twitter，搜索 "Fellou AI" 并关注`，工作流可能如下所示：

```xml
<root>
  <n>在 Twitter 上关注 Fellou AI</n>
  <thought>用户想要在 Twitter 上搜索 "Fellou AI" 并关注该账户。这是一个简单的任务，可以使用浏览器代理导航到 Twitter、执行搜索并关注账户来完成。</thought>
  <agents>
    <agent name="Browser">
      <task>在 Twitter 上搜索并关注 Fellou AI</task>
      <nodes>
        <node>导航到 https://twitter.com</node>
        <node>点击搜索框</node>
        <node>在搜索框中输入文本 "Fellou AI"</node>
        <node>按 Enter 执行搜索</node>
        <node>提取页面内容以找到 Fellou AI 账户</node>
        <node>点击 Fellou AI 账户的"关注"按钮</node>
      </nodes>
    </agent>
  </agents>
</root>
```

> 您可以尝试使用浏览器扩展运行它；或在脚本中注册一个[`流回调`](callback-system.md#流回调)。类型为 `workflow` 的 `StreamCallbackMessage` 将提供工作流值供检查。

这里：

- `<n>` 标签是工作流的名称。
- `<thought>` 标签是生成工作流时的思考过程。
- `<agents>` 标签定义了此工作流需要哪些代理，`<agent name="Browser">` 指定使用浏览器代理。
- `<nodes>` 下的 `<node>` 标签定义了一系列子任务。

### 代理

代理是 Eko 的核心驱动器。每个领域都有一个代理，如浏览器代理、聊天代理等。每个代理包括一组工具、精心制作的提示和合适的 LLM。

代理遵循统一的接口，包括 `name`、`description`、`tools` 和可选的 LLM 模型配置。它们封装特定于领域的逻辑，可以为不同环境进行扩展或自定义。代理负责将高级任务分解为可操作的子任务，并选择适当的工具进行执行。

有关更多信息，请参阅[代理](../agents/overview.md)部分。

#### 规划器

规划器是负责生成工作流的特殊代理，不参与工作流的执行。

规划器分析用户的自然语言提示，确定所需的子任务，并生成结构化的基于 XML 的工作流。这个规划阶段与执行分离，允许用户在运行之前检查、修改或重用生成的计划。规划器确保工作流在逻辑上是合理的，并且子任务之间的所有依赖关系都得到尊重。

#### 工具

工具是在工作流中执行特定操作的可重用功能模块。每个工具实现标准接口，包括 `name`、`description`、`input_schema` 和 `execute` 方法。工具可以是内置的（如文件操作、浏览器自动化或命令执行）或用户自定义的。

#### MCP

MCP（模型上下文协议）是一个架构层，能够动态扩展代理能力。通过 MCP，代理可以在运行时访问额外的工具或服务，如外部 API 或插件。MCP 客户端管理与这些外部资源的通信和集成，允许灵活和可扩展的工作流。

MCP 特别适用于需要与第三方系统集成或动态加载新能力而无需重新部署核心框架的场景。

### 内存

内存机制是任务处理系统中的核心功能，用于高效管理和优化上下文信息。它提取任务中实际使用的工具，删除冗余的工具调用，压缩代理消息，并处理大量上下文消息，减少不必要的计算，优化任务执行效率。

此外，内存机制通过创建任务快照来保留关键信息和节点状态，支持任务中断和恢复，允许任务在更精简的上下文中继续执行。

### LLM

LLM（大型语言模型）集成是 Eko 规划和推理能力的核心。Eko 支持多个 LLM 提供商（如 Anthropic Claude 和 OpenAI），并允许配置模型参数、API 密钥和端点。

LLM 用于工作流规划（由规划器）和某些需要语言理解或生成的代理操作。框架提供重试和回退机制以确保强大的 LLM 交互。

## 执行模型

Eko 采用独特的双层执行模型，将规划和执行分离，从而实现可预测的自动化和自适应行为。

在规划阶段，[`Eko.generate`](/eko/docs/api/classes/Eko.html#generate) 方法将自然语言任务分解为结构化工作流，然后由 `规划器` 将其拆分为子任务和工具调用，并存储为可修改的工作流节点。

执行阶段通过 [`Eko.execute`](/eko/docs/api/classes/Eko.html#execute) 或 [`Eko.run`](/eko/docs/api/classes/Eko.html#run) 启动，系统调用每个代理的 [`Agent.run`](/eko/docs/api/classes/Agent.html#run) 方法，迭代执行工具调用并更新上下文，直到任务完成或达到迭代限制。工具由代理定义和执行，结果反馈给 LLM 或用户。最后，所有代理的结果聚合到工作流的输出中，最后一个代理的结果通常是主要结果。

有一个序列图可视化了提示 `打开 https://fellou.ai 并生成 summary.md` 的执行过程：

![序列图](https://fellou.ai/eko/docs/_astro/sequence-diagram.B4iOcmHv_Z2fyWD6.webp)
