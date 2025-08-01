# X-UI API 参考

基于新的文件结构的完整API参考文档。

## 文件结构

```text
src/ui/
├── components/     # Vue组件
├── composables/    # 函数式API
├── styles/         # 样式文件
├── types/          # 类型定义
└── docs/           # 文档
```

## 组件导出

```typescript
// 组件导出
export {
  XButton, // 按钮组件
  XMessage, // 消息组件
  XModal, // 模态框组件
  XPopover, // 弹出框组件
  XSearchInput, // 搜索输入框组件
  XSwitch, // 开关组件
}
```

## 函数式API导出

```typescript
// 消息API
export { createMessage }
export type { MessageConfig, MessageInstance }

// 消息API方法
createMessage(config: string | MessageConfig): MessageInstance
createMessage.success(message: string, duration?: number): MessageInstance
createMessage.error(message: string, duration?: number): MessageInstance
createMessage.warning(message: string, duration?: number): MessageInstance
createMessage.info(message: string, duration?: number): MessageInstance
createMessage.closeAll(): void
createMessage.close(id: string): void

// 确认对话框API
export { confirm, confirmWarning, confirmDanger, confirmInfo }
export type { ConfirmConfig }

// 确认对话框API方法
confirm(config: string | ConfirmConfig): Promise<boolean>
confirmWarning(message: string, title?: string): Promise<boolean>
confirmDanger(message: string, title?: string): Promise<boolean>
confirmInfo(message: string, title?: string): Promise<boolean>
```

## 配置和工具函数

```typescript
// 全局配置
export interface XUIGlobalConfig {
  size?: 'small' | 'medium' | 'large'
  theme?: 'light' | 'dark'
  locale?: string
  zIndex?: number
  message?: {
    duration?: number
    maxCount?: number
    placement?: 'top' | 'top-left' | 'top-right' | 'bottom' | 'bottom-left' | 'bottom-right'
  }
}

// 配置管理函数
export const getGlobalConfig = (): Required<XUIGlobalConfig>
export const setGlobalConfig = (config: Partial<XUIGlobalConfig>): void
```

## 插件和安装函数

```typescript
// 安装函数
export { install }

// 默认导出（插件）
export default XUI
```

## 类型定义导出

```typescript
// 组件Props类型
export type {
  ButtonProps, // 按钮属性
  SwitchProps, // 开关属性
  ModalProps, // 模态框属性
  PopoverProps, // 弹出框属性
  SearchInputProps, // 搜索输入框属性
  MessageProps, // 消息属性
}

// 组件Events类型
export type {
  ButtonEmits, // 按钮事件
  SwitchEmits, // 开关事件
  ModalEmits, // 模态框事件
  PopoverEmits, // 弹出框事件
  SearchInputEmits, // 搜索输入框事件
  MessageEmits, // 消息事件
}

// 基础类型
export type {
  Size, // 尺寸类型
  Theme, // 主题类型
  Placement, // 位置类型
}
```

## 使用方式总结

### 1. 全局安装

```typescript
import XUI from '@/ui'
app.use(XUI, { size: 'medium' })
```

### 2. 按需导入

```typescript
import { XButton, createMessage } from '@/ui'
```

### 3. 类型导入

```typescript
import type { ButtonProps, XUIGlobalConfig } from '@/ui'
```

### 4. 组件使用

```vue
<template>
  <x-button variant="primary" @click="handleClick">按钮</x-button>
</template>
```

### 5. 函数式API使用

```typescript
// 消息提示
createMessage.success('操作成功！')

// 确认对话框
const result = await confirm('确定要删除吗？')
if (result) {
  // 用户点击了确定
}

// 不同类型的确认对话框
await confirmWarning('这是警告操作')
await confirmDanger('这是危险操作')
```

## 完整导入示例

```typescript
// 完整导入所有功能
import XUI, {
  // 组件
  XButton,
  XModal,
  XSwitch,
  XPopover,
  XSearchInput,
  XMessage,

  // 函数式API
  createMessage,

  // 配置函数
  setGlobalConfig,
  getGlobalConfig,

  // 安装函数
  install,

  // 类型
  type ButtonProps,
  type ModalProps,
  type XUIGlobalConfig,
  type MessageConfig,
} from '@/ui'

// 使用默认导出安装
app.use(XUI)

// 或使用命名导出安装
app.use(install, { theme: 'dark' })
```
