# 可用代理

Eko 为不同环境提供各种内置代理，包括：

- [浏览器扩展](#浏览器扩展)
- [Node.js](#nodejs)
- [Web](#web)

## 浏览器扩展

### BrowserAgent

`Browser`: 使用浏览器代理来操作浏览器。

```typescript
import { Eko } from "@eko-ai/eko";
import { BrowserAgent } from "@eko-ai/eko-extension";

let eko = new Eko({
  llms: {
    default: {
      provider: "anthropic",
      model: "claude-3-7-sonnet",
      apiKey: "your_api_key"
    },
  },
  agents: [new BrowserAgent()],
});

let result = await eko.run(`
  打开 Twitter，搜索 "Fellou AI" 并关注`);
```

该代理内置以下工具：

- `navigate_to`: 导航到特定 url
- `current_page`: 获取当前网页的信息（url、标题）
- `go_back`: 在浏览器历史记录中后退
- `input_text`: 向元素输入文本
- `click_element`: 通过索引点击元素
- `scroll_to_element`: 滚动到元素
- `scroll_mouse_wheel`: 在当前位置滚动鼠标滚轮
- `hover_to_element`: 鼠标悬停在元素上
- `extract_page_content`: 提取当前网页的文本内容
- `get_select_options`: 从原生下拉元素获取所有选项
- `select_option`: 选择原生下拉选项
- `get_all_tabs`: 获取当前浏览器的所有标签页
- `switch_tab`: 切换到指定的标签页
- `wait`: 等待指定的持续时间

## Node.js

### BrowserAgent

`Browser`: 使用浏览器代理来操作浏览器。

```typescript
import { Eko } from "@eko-ai/eko";
import { BrowserAgent } from "@eko-ai/eko-nodejs";

let eko = new Eko({
  llms: {
    default: {
      provider: "anthropic",
      model: "claude-3-7-sonnet",
      apiKey: "your_api_key"
    },
  },
  agents: [new BrowserAgent()],
});

let result = await eko.run(`
  打开 Twitter，搜索 "Fellou AI" 并关注`);
```

### FileAgent

`File`: 使用文件代理来操作本地文件。

```typescript
import { Eko } from "@eko-ai/eko";
import { FileAgent } from "@eko-ai/eko-nodejs";

let eko = new Eko({
  llms: {
    default: {
      provider: "anthropic",
      model: "claude-3-7-sonnet",
      apiKey: "your_api_key"
    },
  },
  agents: [new FileAgent()],
});

let result = await eko.run(`
  在桌面创建一个 test.txt 并写入 hello eko。`);
```

该代理内置以下工具：

- `file_list`: 获取指定目录中的文件列表。
- `file_read`: 读取文件内容。用于读取文件或检查文件内容。
- `file_write`: 覆盖或追加内容到文件。
- `file_str_replace`: 替换文件中的指定字符串。
- `file_find_by_name`: 在指定目录中按名称模式查找文件。

## Web

### BrowserAgent

`Browser`: 使用浏览器代理来操作当前页面。

```typescript
import { Eko } from "@eko-ai/eko";
import { BrowserAgent } from "@eko-ai/eko-web";

let eko = new Eko({
  llms: {
    default: {
      provider: "anthropic",
      model: "claude-3-7-sonnet",
      apiKey: "your_api_key"
    },
  },
  agents: [new BrowserAgent()],
});

let result = await eko.run(`
  找到文本框并输入 hello eko`);
```
