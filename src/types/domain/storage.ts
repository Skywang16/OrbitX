export enum StorageLayer {
  Config = 'config',
  State = 'state',
  Data = 'data',
}

export enum ConfigSection {
  App = 'app',
  Appearance = 'appearance',
  Terminal = 'terminal',
  Shortcuts = 'shortcuts',
  Ai = 'ai',
}

export interface DataQuery {
  query: string
  params: Record<string, unknown>
  limit?: number
  offset?: number
  order_by?: string
  desc: boolean
}

export interface SaveOptions {
  table?: string
  overwrite: boolean
  backup: boolean
  validate: boolean
  metadata: Record<string, unknown>
}

export interface WindowState {
  x: number
  y: number
  width: number
  height: number
  maximized: boolean
}

/**
 * Tab ID - 支持 number 和 string
 */
export type TabId = number | string

/**
 * Terminal tab 数据
 */
export interface TerminalTabData {
  title: string
  shell?: string
  cwd?: string
}

/**
 * Terminal tab 状态
 */
export interface TerminalTabState {
  type: 'terminal'
  id: number
  active: boolean
  data: TerminalTabData
}

/**
 * Settings tab 数据
 */
export interface SettingsTabData {
  lastSection?: string
}

/**
 * Settings tab 状态
 */
export interface SettingsTabState {
  type: 'settings'
  id: string
  active: boolean
  data: SettingsTabData
}

/**
 * Tab 状态 - union type
 */
export type TabState = TerminalTabState | SettingsTabState

/**
 * 运行时终端状态（从后端查询）
 */
export interface RuntimeTerminalState {
  id: number
  title: string
  cwd: string
  active: boolean
  shell?: string
}

export interface UiState {
  theme: string
  fontSize: number
  sidebarWidth: number
  opacity?: number
  language?: string
}

export interface TaskNode {
  type: string
  text: string
  status?: 'pending' | 'running' | 'completed'
}

export interface AiState {
  visible: boolean
  width: number
  mode: 'chat' | 'agent'
  conversationId?: number
  selectedModelId?: string | null
}

/**
 * 会话状态 - 统一 tab 管理
 */
export interface SessionState {
  version: number
  tabs: TabState[]
  activeTabId?: TabId
  ui: UiState
  ai: AiState
  timestamp: string
}

export interface StorageEvent {
  type: 'config_changed' | 'state_saved' | 'state_loaded' | 'data_updated' | 'error'
  data: unknown
  timestamp: number
}

export interface StorageOperationResult {
  success: boolean
  error?: string
}
