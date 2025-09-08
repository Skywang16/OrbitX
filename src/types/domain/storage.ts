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
  opacity?: number
  language?: string
}

export interface AiState {
  visible: boolean
  width: number
  mode: 'chat' | 'agent'
  conversationId?: number
  selectedModelId?: string | null
  // 与后端对齐的向量索引设置
  vectorIndexEnabled: boolean
  vectorIndexWorkspaces: string[]
}

export interface SessionState {
  version: number
  terminals: TerminalState[]
  activeTabId?: string
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
