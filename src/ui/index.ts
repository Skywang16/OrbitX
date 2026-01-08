import type { App, Plugin } from 'vue'

import './styles/index.css'

import { XButton, XMessage, XModal, XSearchInput, XSelect, XSwitch } from './components'

import {
  confirm,
  confirmDanger,
  confirmInfo,
  confirmWarning,
  createMessage,
  type ConfirmConfig,
  type MessageConfig,
  type MessageInstance,
} from './composables'

import { createPopover, showContextMenu, showPopoverAt } from './composables/popover-api'

// 主要组件导出（推荐使用）
export { XButton, XMessage, XModal, XSearchInput, XSelect, XSwitch }

// 系统级菜单 API
export { createPopover, showContextMenu, showPopoverAt }

// 消息API
export { createMessage }
export type { MessageConfig, MessageInstance }

// 确认对话框API
export { confirm, confirmDanger, confirmInfo, confirmWarning }
export type { ConfirmConfig }

// 消息API便捷方法（已在createMessage上定义）
// createMessage.success(message, duration?)
// createMessage.error(message, duration?)
// createMessage.warning(message, duration?)
// createMessage.info(message, duration?)
// createMessage.closeAll()
// createMessage.close(id)

// 全局配置接口
export interface XUIGlobalConfig {
  // 全局尺寸
  size?: 'small' | 'medium' | 'large'
  // 全局主题
  theme?: 'light' | 'dark'
  // 国际化语言
  locale?: string
  // 全局z-index基数
  zIndex?: number
  // 消息组件配置
  message?: {
    duration?: number
    maxCount?: number
    placement?: 'top' | 'top-left' | 'top-right' | 'bottom' | 'bottom-left' | 'bottom-right'
  }
}

// 默认配置
const defaultConfig: Required<XUIGlobalConfig> = {
  size: 'medium',
  theme: 'light',
  locale: 'zh-CN',
  zIndex: 1000,
  message: {
    duration: 3000,
    maxCount: 5,
    placement: 'top-right',
  },
}

// 全局配置存储
let globalConfig: Required<XUIGlobalConfig> = { ...defaultConfig }

// 配置管理函数
export const getGlobalConfig = (): Required<XUIGlobalConfig> => globalConfig
export const setGlobalConfig = (config: Partial<XUIGlobalConfig>): void => {
  globalConfig = { ...globalConfig, ...config }
}

// 安装函数
const install = (app: App, options: Partial<XUIGlobalConfig> = {}): void => {
  // 设置全局配置
  setGlobalConfig(options)

  // 注册所有组件 - 明确指定组件名称
  app.component('XButton', XButton)
  app.component('x-button', XButton)

  app.component('XMessage', XMessage)
  app.component('x-message', XMessage)

  app.component('XModal', XModal)
  app.component('x-modal', XModal)

  app.component('XSearchInput', XSearchInput)
  app.component('x-search-input', XSearchInput)

  app.component('XSelect', XSelect)
  app.component('x-select', XSelect)

  app.component('XSwitch', XSwitch)
  app.component('x-switch', XSwitch)

  // 挂载全局方法
  app.config.globalProperties.$message = createMessage
  app.provide('xui-config', globalConfig)
}

// 组件库插件类型
type XUIPlugin = Plugin & {
  version: string
  install: (app: App, options?: Partial<XUIGlobalConfig>) => void
}

// 组件库插件
const XUI: XUIPlugin = {
  install,
  version: '1.0.0',
}

// 安装函数导出
export { install }

// 默认导出（插件）
export default XUI

export * from './types/index'

export type {
  ButtonEmits,
  ButtonProps,
  MessageEmits,
  MessageProps,
  ModalEmits,
  ModalProps,
  Placement,
  SearchInputEmits,
  SearchInputProps,
  SelectEmits,
  SelectOption,
  SelectProps,
  Size,
  SwitchEmits,
  SwitchProps,
  Theme,
} from './types/index'
