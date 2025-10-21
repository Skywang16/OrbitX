/**
 * UI业务领域类型定义 - 简洁版本
 */

import type { Size } from '../core'

// ===== 标签页类型 =====

export enum TabType {
  TERMINAL = 'terminal',
  SETTINGS = 'settings',
}

export interface TabItem {
  id: number
  title?: string
  type: TabType
  closable?: boolean
  shell?: string
  path?: string
  data?: { section?: string; paneId?: number | null; [key: string]: unknown }
}

// ===== 应用设置类型 =====

export interface AppSettings {
  theme: {
    mode: ThemeMode
    terminalTheme: string
  }
  terminal: {
    fontFamily: string
    fontSize: number
    cursorStyle: string
    cursorBlink: boolean
    scrollback: number
  }
  window: {
    opacity: number
    alwaysOnTop: boolean
    startMaximized: boolean
  }
  general: {
    language: string
    autoSave: boolean
    confirmOnExit: boolean
  }
}

// ===== 组件基础类型 =====

export type ThemeMode = 'light' | 'dark' | 'auto'
export type Placement = 'top' | 'bottom' | 'left' | 'right'

export interface SelectOption {
  label: string
  value: string | number
  disabled?: boolean
}

// ===== 按钮组件类型 =====

export interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
  size?: Size
  disabled?: boolean
  loading?: boolean
  type?: 'button' | 'submit'
}

// ===== 开关组件类型 =====

export interface SwitchProps {
  modelValue: boolean
  disabled?: boolean
  size?: Size
}

// ===== 模态框组件类型 =====

export interface ModalProps {
  visible?: boolean
  title?: string
  size?: 'small' | 'medium' | 'large'
  closable?: boolean
  maskClosable?: boolean
}

// ===== 消息组件类型 =====

export interface MessageProps {
  visible: boolean
  message: string
  type?: 'success' | 'warning' | 'error' | 'info'
  duration?: number
  closable?: boolean
}

// ===== 选择器组件类型 =====

export interface SelectProps {
  modelValue?: string | number | null
  options: SelectOption[]
  placeholder?: string
  disabled?: boolean
  clearable?: boolean
  size?: Size
}
