/**
 * 存储业务领域类型定义
 */

// ===== 存储层类型 =====

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

// ===== 查询和保存类型 =====

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

// ===== 会话状态类型 =====

export interface WindowState {
  x: number
  y: number
  width: number
  height: number
  maximized: boolean
}

export interface TerminalState {
  id: string
  title: string
  cwd: string
  active: boolean
  shell?: string
}

export interface UiState {
  theme: string
  fontSize: number
  sidebarWidth: number
}

export interface AiState {
  visible: boolean
  width: number
  mode: 'chat' | 'agent'
  conversationId?: number
}

export interface SessionState {
  version: number
  terminals: TerminalState[]
  activeTabId?: string
  ui: UiState
  ai: AiState
  timestamp: string
}

// ===== 事件类型 =====

export interface StorageEvent {
  type: 'config_changed' | 'state_saved' | 'state_loaded' | 'data_updated' | 'error'
  data: unknown
  timestamp: number
}

// ===== 操作结果类型 =====

export interface StorageOperationResult {
  success: boolean
  error?: string
}
