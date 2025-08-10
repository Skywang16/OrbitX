# 浏览器代理

## 概述

**浏览器代理**是 Eko 中的内置代理，旨在以类似人类的方式与网页交互。它使自动化工作流能够执行浏览器操作，如导航、元素交互、内容提取等。浏览器代理在多种环境中可用，包括浏览器扩展、Node.js（通过 Playwright）和 web（沙盒 DOM 自动化）。

**工具：**

- 导航：`navigate_to`、`go_back`、`switch_tab`、`get_all_tabs`
- 元素交互：`input_text`、`click_element`、`hover_to_element`、`scroll_to_element`、`select_option`、`get_select_options`
- 内容提取：`extract_page_content`、`screenshot_and_html`
- 滚动：`scroll_mouse_wheel`
- 实用工具：`wait`

**支持的环境：**

- 浏览器扩展（Chrome）
- Node.js（通过 Playwright 的无头或可见浏览器）
- Web（沙盒，限于当前页面）

## 架构

浏览器代理实现为一组可扩展的类：

- [`BaseBrowserAgent`](../../../packages/eko-core/src/agent/browser/browser_base.ts)：  
  扩展 `Agent`，定义浏览器代理的核心接口，包括截图、导航、标签管理和 js 脚本的执行方法。
  
- [`BaseBrowserLabelsAgent`](../../../packages/eko-core/src/agent/browser/browser_labels.ts)：  
  扩展 `BaseBrowserAgent` 以提供使用索引元素的元素级交互、截图和 HTML 提取以及基于工具的操作。这是扩展/web/Node.js 环境中浏览器自动化的主要基础。
  
- [`BaseBrowserScreenAgent`](../../../packages/eko-core/src/agent/browser/browser_screen.ts)：  
  扩展 `BaseBrowserAgent` 以提供基于坐标的鼠标和键盘自动化接口，适用于基于屏幕的自动化场景。

每个环境（扩展、Node.js、web）通过扩展 `BaseBrowserLabelsAgent` 并提供特定于环境的导航、脚本执行和截图逻辑来实现自己的 `BrowserAgent` 类。

## 特性

- **截图和 DOM 提取：**  
  代理将页面截图与交互元素的伪 HTML 表示相结合。每个可操作元素都被索引并在截图中进行视觉标记，实现强大的元素识别和交互。
  
- **元素索引和交互：**  
  所有交互元素（按钮、输入、链接等）都被分配唯一索引。诸如 `click_element`、`input_text` 和 `hover_to_element` 等工具在这些索引上操作，确保精确可靠的自动化。
  
- **基于工具的操作模型：**  
  所有浏览器操作都作为工具公开（例如，`navigate_to`、`input_text`、`extract_page_content`）。代理通过按顺序调用这些工具来执行工作流，允许模块化、可解释和可扩展的自动化。
  
- **多环境支持：**  
  相同的高级接口在浏览器扩展、Node.js 和 web 环境中可用，具有特定于环境的低级操作实现。
  
- **错误处理和健壮性：**  
  代理包括内置错误处理、回退策略，并支持等待、重试和在找不到元素或操作失败时的替代方法。

## 内置工具和方法

浏览器代理为浏览器自动化公开以下工具和方法：

- **导航**
  
  - `navigate_to(url: string)`: 导航到特定 URL。
  - `go_back()`: 返回到历史记录中的上一页。
  - `switch_tab(index: number)`: 通过索引切换到不同的浏览器标签页。
  - `get_all_tabs()`: 检索所有打开标签页的列表。

- **元素交互**
  
  - `input_text(index: number, text: string, enter?: boolean)`: 通过索引向输入元素输入文本。可选择在输入后按 Enter。
  - `click_element(index: number, num_clicks?: number, button?: "left" | "right" | "middle")`: 通过索引点击元素，可选择点击次数和鼠标按钮。
  - `hover_to_element(index: number)`: 通过索引将鼠标指针移动到元素上。
  - `scroll_to_element(index: number)`: 滚动页面以将指定元素带入视图。
  - `select_option(index: number, option_value: string)`: 通过索引从下拉/选择元素中选择选项。
  - `get_select_options(index: number)`: 通过索引检索选择元素的可用选项。

- **内容提取**
  
  - `extract_page_content(variable_name?: string)`: 提取当前页面的完整 HTML/文本内容。
  - `screenshot_and_html()`: 捕获可见页面的截图并提取结构化元素信息。

- **滚动**
  
  - `scroll_mouse_wheel(amount: number)`: 按指定量垂直滚动页面。

- **实用工具**
  
  - `wait(milliseconds: number)`: 等待指定的持续时间（用于等待内容加载）。

- **错误处理**: 代理包括内置错误处理、重试和回退策略，以实现强大的自动化。

## 使用示例

虽然 `BrowserAgent` 主要由 Eko 框架使用，但您也可以单独创建 `BrowserAgent` 并调试其方法。以下是不同环境中浏览器代理的示例使用模式。

### Node.js（使用 Playwright）

```typescript
import { BrowserAgent } from 'eko/agents/browser';
import { createPlaywrightEnv } from 'eko/envs/playwright';

const env = await createPlaywrightEnv();
const agent = new BrowserAgent({ env });

await agent.navigate_to('https://example.com');
await agent.input_text(2, 'hello world', true); // 向索引为 2 的输入框输入并按 Enter
await agent.click_element(3); // 点击索引为 3 的按钮
const content = await agent.extract_page_content();
console.log(content);
```

### 浏览器扩展

```typescript
import { BrowserAgent } from 'eko/agents/browser';

const agent = new BrowserAgent();

await agent.navigate_to('https://example.com');
await agent.click_element(1); // 点击索引为 1 的元素
await agent.wait(1000); // 等待 1 秒
const screenshot = await agent.screenshot_and_html();
console.log(screenshot);
```

### Web（沙盒 DOM 自动化）

```typescript
import { BrowserAgent } from 'eko/agents/browser';

const agent = new BrowserAgent();

await agent.input_text(0, 'search query');
await agent.click_element(1); // 点击搜索按钮
await agent.scroll_to_element(5); // 滚动到索引为 5 的元素
```

**注意：**

- 元素索引由代理的 DOM 提取确定，每次页面加载可能会有所不同。
- 始终使用 `screenshot_and_html` 或类似方法返回的最新元素列表来确定有效索引。
- 对于高级场景，扩展 `BaseBrowserLabelsAgent` 或 `BaseBrowserScreenAgent` 以实现自定义逻辑。

### 自定义浏览器

```typescript
import { AgentContext, BaseBrowserLabelsAgent } from "@eko-ai/eko";

export class E2bBrowser extends BaseBrowserLabelsAgent {
  protected screenshot(agentContext: AgentContext): Promise<{ imageBase64: string; imageType: "image/jpeg" | "image/png"; }> {
    throw new Error("方法未实现。");
  }
  protected navigate_to(agentContext: AgentContext, url: string): Promise<{ url: string; title?: string; }> {
    throw new Error("方法未实现。");
  }
  protected get_all_tabs(agentContext: AgentContext): Promise<Array<{ tabId: number; url: string; title: string; }>> {
    throw new Error("方法未实现。");
  }
  protected switch_tab(agentContext: AgentContext, tabId: number): Promise<{ tabId: number; url: string; title: string; }> {
    throw new Error("方法未实现。");
  }
  protected execute_script(agentContext: AgentContext, func: (...args: any[]) => void, args: any[]): Promise<any> {
    throw new Error("方法未实现。");
  }
}
```

## API 参考

- 类和方法摘要：
  - [BaseBrowserAgent](/eko/docs/api/classes/BaseBrowserAgent.html)
  - [BaseBrowserLabelsAgent](/eko/docs/api/classes/BaseBrowserLabelsAgent.html)
  - [BaseBrowserScreenAgent](/eko/docs/api/classes/BaseBrowserScreenAgent.html)

## 相关链接

- 可用工具
- 多代理概述
- API 文档
