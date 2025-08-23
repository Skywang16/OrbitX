# 快速开始

这里有两种快速开始的方法：

1. [**使用浏览器扩展**](#使用浏览器扩展): 适合那些只想试用或非专业人士。
2. [**运行 Node.js 脚本**](#运行nodejs脚本): 适合想要查看或修改代码细节的专业人士。

## 使用浏览器扩展

让我们在浏览器扩展中一起运行 Eko 工作流，自动化任务 `打开 Twitter，搜索 "Fellou AI" 并关注`。

### 加载扩展

- 下载 _[预编译扩展](https://github.com/FellouAI/eko-demos/raw/refs/heads/main/browser-extension-dist/dist.zip)_（或者您也可以[自己构建](./installation.md#安装)）。将 ZIP 文件解压到合适的位置，您应该看到一个 `dist` 文件夹。
- 打开 [Chrome 浏览器](https://www.google.com/chrome/) 并导航到 `chrome://extensions/`。
- 打开 `开发者模式`（右上角的切换开关）。
- 点击 `加载已解压的扩展程序` 按钮（左上角的蓝色文本）并选择第一步中的 `dist` 文件夹。

### 配置 LLM 模型 API 密钥

- 如果从 OpenAI 或 Claude 平台获取 API 密钥不方便，可以考虑使用代理站点或服务（如 [OpenRouter](https://openrouter.ai/)），然后将 _Base URL_ 和 _API key_ 替换为相应的值。

### 让我们运行它！

打开扩展的侧边栏，输入您的提示，然后点击运行按钮：

## 运行 Node.js 脚本

首先我们需要创建一个新项目：

```bash
mkdir try-eko
cd try-eko
npm init
```

然后安装依赖：

```bash
npm add @eko-ai/eko @eko-ai/eko-nodejs ts-node
```

接着编写一个名为 `index.ts` 的脚本：

```typescript
import { Eko, Agent, LLMs } from '@eko-ai/eko'
import { BrowserAgent } from '@eko-ai/eko-nodejs'

async function run() {
  let llms: LLMs = {
    default: {
      provider: 'anthropic',
      model: 'claude-3-5-sonnet-20241022',
      apiKey: 'sk-xxx', // 替换为您的 API KEY
      config: {
        baseURL: 'https://api.anthropic.com/v1',
      },
    },
  }
  let agents: Agent[] = [new BrowserAgent()]
  let eko = new Eko({ llms, agents })
  let result = await eko.run('搜索关于马斯克的最新新闻')
  console.log('结果: ', result.result)
}

run().catch(e => {
  console.log(e)
})
```

记住设置环境变量（OpenAI/Claude 其中之一）：

```bash
export OPENAI_BASE_URL=your_value
export OPENAI_API_KEY=your_value
export ANTHROPIC_BASE_URL=your_value
export ANTHROPIC_API_KEY=your_value
```

最后运行它：

```bash
ts-node index.ts
```

## 下一步

现在您已经运行了第一个工作流，您可以：

- 了解 Eko 在不同环境中的[安装](installation.md)。
- 尝试 Eko 的不同[配置](configuration.md)。
- 学习 Eko 的[架构](../architecture/overview.md)。
- 查看[参考文档](../reference/overview.md)确认代码细节。
- 加入我们的 [Discord](https://discord.gg/XpFfk2e5): ![Discord](https://fellou.ai/eko/docs/_astro/discard.DZEwd05S_Z2jsIBU.webp)
