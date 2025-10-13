# 工具函数清单

本文档列出项目中已有的工具函数、Composables和常用第三方库。开发新功能前请先查看这里。

## 📁 目录结构

```
src/
├── utils/              # 通用工具函数
├── composables/        # Vue组合式函数
└── eko-core/common/    # 核心工具库
```

## 🕐 时间处理

### 位置：`src/utils/dateFormatter.ts`

**依赖**：使用 `dayjs` 库（已安装）

#### 可用函数

```typescript
import {
  formatTime, // 格式化为 HH:mm
  formatDateTime, // 格式化为 YYYY-MM-DD HH:mm:ss
  formatDate, // 格式化为 YYYY-MM-DD
  formatRelativeTime, // 相对时间（昨天、3天前等）
  formatSessionTime, // 会话时间（刚刚、5分钟前等）
  getRelativeTime, // dayjs的fromNow
  isValidDate, // 验证日期有效性
} from '@/utils/dateFormatter'
```

#### 示例

```typescript
// 显示时间 09:30
formatTime(new Date())

// 显示完整时间 2024-03-15 09:30:45
formatDateTime(Date.now())

// 相对时间显示 "3天前"
formatRelativeTime(timestamp)

// 会话时间 "刚刚" / "5分钟前"
formatSessionTime(message.createdAt)
```

## 💾 本地存储

### 位置：`src/utils/storage.ts`

类型安全的localStorage封装

#### 使用方法

```typescript
import { createStorage } from '@/utils/storage'

// 创建存储实例
const userStorage = createStorage<UserData>('user-data')

// 保存数据
userStorage.save({ name: 'John', age: 30 })

// 读取数据
const data = userStorage.load() // 返回 UserData | null

// 检查是否存在
if (userStorage.exists()) {
  // ...
}

// 删除数据
userStorage.remove()
```

## 🎨 主题相关

### 位置：`src/utils/`

- `themeApplier.ts` - 主题应用逻辑
- `themeConverter.ts` - 主题格式转换
- `terminalTheme.ts` - 终端主题配置

## 🧩 Vue工具

### VueUse库

项目已安装`@vueuse/core`，提供大量通用组合式函数：

```typescript
import { 
  useDebounce,      // 防抖
  useThrottle,      // 节流
  useLocalStorage,  // localStorage响应式
  useClipboard,     // 剪贴板
  useEventListener, // 事件监听
  onClickOutside,   // 点击外部
  useDark,          // 暗黑模式
  useToggle,        // 切换状态
} from '@vueuse/core'
```

**文档**：https://vueuse.org/

### 业务Composables（非通用工具）

项目中的业务相关composables：

- `useConfig` - 配置管理
- `useLLMRegistry` - LLM注册表
- `useStepProcessor` - 步骤处理器（Agent专用）
- `useTerminalOutput` - 终端输出
- `useTerminalSearch` - 终端搜索
- `useTerminalSelection` - 终端选择
- `useShellIntegration` - Shell集成
- `useShortcuts` - 快捷键

> **注意**：这些是业务逻辑封装，不属于通用工具。

## 🛠️ 核心工具

### 位置：`src/eko-core/common/utils.ts`

#### 异步工具

```typescript
// 延迟执行
await sleep(1000) // 延迟1秒

// 带超时的Promise
await call_timeout(asyncFunction, 5000) // 5秒超时
```

#### UUID生成

```typescript
const id = uuidv4() // 生成UUID
```

#### 字符串处理

```typescript
// 截取字符串
sub('很长的文本', 10) // "很长的文本..."
sub('很长的文本', 10, false) // "很长的文本"
```

#### XML处理

```typescript
// 修复不完整的XML标签
fixXmlTag('<root><item>content')
// 返回: '<root><item>content</item></root>'
```

#### 工具相关

```typescript
// 转换工具schema
convertToolSchema(tool)
convertTools(tools)

// 合并工具列表
mergeTools(tools1, tools2)
```

#### 消息处理

```typescript
// 创建LLM消息
createTextMessage('user', 'Hello')
createToolCallMessage(toolCalls)
createToolResultMessage(toolCall, result)

// 提取消息内容
extractTextFromMessage(message)
extractToolCallsFromMessage(message)
```

## 📦 第三方库

### 已安装的常用库

#### UI & Vue

- `@vueuse/core` - Vue组合式工具集
- `pinia` - 状态管理
- `vue-i18n` - 国际化

#### 工具库

- `dayjs` - 轻量级日期处理库
  - 已集成插件：relativeTime, locale(zh-cn, en)
- `lodash-es` - 工具函数库

  ```typescript
  import { debounce, throttle, cloneDeep } from 'lodash-es'
  ```

- `uuid` - UUID生成

  ```typescript
  import { v4 as uuidv4 } from 'uuid'
  ```

- `strip-ansi` - 移除ANSI转义码
  ```typescript
  import stripAnsi from 'strip-ansi'
  const clean = stripAnsi(coloredText)
  ```

#### 解析 & 验证

- `marked` - Markdown渲染

  ```typescript
  import { marked } from 'marked'
  const html = marked(markdown)
  ```

- `zod` - 类型验证

  ```typescript
  import { z } from 'zod'
  ```

- `ajv` - JSON Schema验证

#### Tauri插件

- `@tauri-apps/api` - Tauri核心API
- `@tauri-apps/plugin-fs` - 文件系统
- `@tauri-apps/plugin-http` - HTTP请求
- `@tauri-apps/plugin-process` - 进程管理
- `@tauri-apps/plugin-opener` - 打开文件/URL

#### 终端相关

- `@xterm/xterm` - 终端模拟器核心
- `@xterm/addon-fit` - 自适应大小
- `@xterm/addon-search` - 搜索功能
- `@xterm/addon-web-links` - 链接识别

## 🔍 如何查找

### 1. 搜索关键词

在IDE中全局搜索相关功能：

```bash
# 搜索函数名
Cmd/Ctrl + Shift + F

# 常见关键词
- format: 格式化相关
- parse: 解析相关
- validate: 验证相关
- convert: 转换相关
- create: 创建相关
```

### 2. 查看导入

看其他类似功能的文件是如何导入的：

```typescript
// 搜索 import 语句
import { formatTime } from '@/utils/dateFormatter'
```

### 3. 查看package.json

检查是否已安装所需的第三方库

## 📝 使用建议

### 优先级

1. **项目已有工具** - 优先使用项目封装的工具函数
2. **已安装的第三方库** - 利用已有依赖
3. **新增工具** - 确实没有才考虑新增

### 新增工具时

如果需要新增工具函数：

1. **放对位置**
   - 通用工具 → `src/utils/`
   - Vue相关 → `src/composables/`
   - 核心逻辑 → `src/eko-core/common/`

2. **命名规范**
   - 函数：驼峰命名，动词开头 `formatTime`, `parseData`
   - Composables：`use`前缀 `useConfig`, `useTheme`

3. **添加类型**

   ```typescript
   export function formatTime(date: Date | string | number): string {
     // ...
   }
   ```

4. **更新此文档** - 新增工具后记得更新这个清单

## ⚠️ 常见陷阱

### 不要重新实现已有功能

```typescript
// ❌ 不要
function formatTimestamp(ts: number): string {
  const date = new Date(ts)
  return `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()}`
}

// ✅ 使用已有
import { formatDate } from '@/utils/dateFormatter'
formatDate(timestamp)
```

### 不要重复安装已有库

```bash
# ❌ 不要
npm install moment  # 项目已有dayjs

# ✅ 使用已有
import dayjs from 'dayjs'
```

### 统一使用相同的库

```typescript
// ❌ 不统一
import moment from 'moment' // 文件A
import dayjs from 'dayjs' // 文件B
const formatted = new Date().toLocaleDateString() // 文件C

// ✅ 统一
import { formatDate } from '@/utils/dateFormatter' // 所有文件
```

## 🔄 保持更新

此文档应该随项目发展不断更新：

- ✅ 新增工具函数时更新
- ✅ 安装新依赖时更新
- ✅ 重构工具时更新
- ✅ 发现遗漏时补充

**最后更新**：2025-09-30
