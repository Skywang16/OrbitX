/**
 * UI业务领域类型定义
 */

// ===== 标签页类型 =====

export enum TabType {
  TERMINAL = 'terminal',
  SETTINGS = 'settings',
}

// 终端标签页的 UI 数据（运行时使用，不含 cwd）
export interface TerminalTabData {
  shell: string
}

// 设置标签页的 UI 数据
export interface SettingsTabData {
  section: string
}

// 标签页基础结构：通用字段 + 类型化的私有数据
export interface TabItem<TData = unknown> {
  id: number
  type: TabType
  closable: boolean
  data: TData
}

// 类型别名（方便使用）
export type TerminalTabItem = TabItem<TerminalTabData>
export type SettingsTabItem = TabItem<SettingsTabData>

// 联合类型（实际使用的类型）
export type AnyTabItem = TerminalTabItem | SettingsTabItem

// ===== 组件基础类型 =====

export type ThemeMode = 'light' | 'dark' | 'auto'
export type Placement = 'top' | 'bottom' | 'left' | 'right'

export interface SelectOption {
  label: string
  value: string | number
  disabled?: boolean
}
