# X-UI 组件库

现代化的Vue 3组件库，提供统一的UI组件和函数式API。

## 📁 文件结构

```text
src/ui/
├── components/          # Vue组件
│   ├── Button.vue      # 按钮组件
│   ├── Message.vue     # 消息组件
│   ├── Modal.vue       # 模态框组件
│   ├── Popconfirm.vue  # 弹出确认框组件
│   ├── SearchInput.vue # 搜索输入框组件
│   └── Switch.vue      # 开关组件
├── composables/         # 函数式API
│   ├── message-api.ts  # 消息API
│   ├── confirm-api.ts  # 确认对话框API
│   └── popover-api.ts  # 系统菜单API
├── styles/             # 样式文件
│   └── index.css       # 主样式文件
├── types/              # 类型定义
│   └── index.ts        # 类型定义文件
├── docs/               # 文档
│   └── API.md          # API参考文档
├── index.ts            # 主入口文件
└── README.md           # 说明文档
```

## 🚀 快速开始

### 全局安装

```typescript
// main.ts
import { createApp } from 'vue'
import XUI from '@/ui'

const app = createApp(App)
app.use(XUI, {
  size: 'medium',
  theme: 'light',
})
```

### 按需导入

```typescript
// 导入组件
import { XButton, XModal } from '@/ui'

// 导入函数式API
import { createMessage, confirm } from '@/ui'

// 导入类型
import type { ButtonProps, ModalProps } from '@/ui'
```

## 📦 组件列表

### 基础组件

- **XButton** - 按钮组件，支持多种样式和状态
- **XSwitch** - 开关组件，支持加载状态
- **XSearchInput** - 搜索输入框，支持防抖和清除

### 反馈组件

- **XMessage** - 消息提示组件
- **XModal** - 模态框组件，支持多种尺寸
- **XPopconfirm** - 弹出确认框组件，使用系统菜单

### 函数式API

- **createPopover** - 创建系统级弹出菜单
- **showContextMenu** - 显示右键上下文菜单
- **showPopoverAt** - 在指定位置显示菜单

## 🔧 函数式API

### 消息提示

```typescript
import { createMessage } from '@/ui'

// 基础用法
createMessage('这是一条消息')

// 便捷方法
createMessage.success('操作成功！')
createMessage.error('操作失败！')
createMessage.warning('警告信息')
createMessage.info('提示信息')
```

### 确认对话框

```typescript
import { confirm, confirmWarning, confirmDanger } from '@/ui'

// 基础确认
const result = await confirm('确定要删除吗？')
if (result) {
  // 用户点击了确定
}

// 警告确认
await confirmWarning('这是警告操作')

// 危险确认
await confirmDanger('这是危险操作')
```

## 🎨 主题系统

X-UI 完美集成现有主题系统，自动使用以下CSS变量：

```css
:root {
  --color-primary: #1890ff;
  --color-success: #52c41a;
  --color-warning: #faad14;
  --color-danger: #ff4d4f;

  --text-primary: #333;
  --color-background: #fff;
  --border-color: #d9d9d9;

  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 12px;
  --spacing-lg: 16px;

  --font-size-xs: 12px;
  --font-size-sm: 14px;
  --font-size-md: 16px;
  --font-size-lg: 18px;

  --border-radius: 6px;
}
```

## 📖 详细文档

- [API参考文档](./docs/API.md) - 完整的API文档
- [组件示例](./docs/EXAMPLES.md) - 组件使用示例和最佳实践

## 🔄 版本历史

### v1.0.0

- ✨ 初始版本发布
- ✨ 6个核心组件
- ✨ 完整的函数式API
- ✨ TypeScript支持
- ✨ 主题系统集成

## 🤝 贡献指南

1. 组件开发请在 `components/` 目录下进行
2. 函数式API请在 `composables/` 目录下开发
3. 类型定义统一在 `types/index.ts` 中管理
4. 样式文件统一在 `styles/` 目录下管理
5. 文档更新请同步更新 `docs/` 目录

## 📄 许可证

MIT License
