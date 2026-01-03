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
 * Tab ID - 统一使用 number
 */
export type TabId = number

/**
 * Terminal tab 持久化数据（含 cwd）
 */
export interface PersistedTerminalTabData {
  shell: string
  cwd: string
}

/**
 * Terminal tab 状态（持久化）
 */
export interface TerminalTabState {
  type: 'terminal'
  id: number
  isActive: boolean
  data: PersistedTerminalTabData
}

/**
 * Settings tab 持久化数据
 */
export interface PersistedSettingsTabData {
  lastSection?: string
}

/**
 * Settings tab 状态（持久化）
 */
export interface SettingsTabState {
  type: 'settings'
  id: number
  isActive: boolean
  data: PersistedSettingsTabData
}

/**
 * Tab 状态 - union type
 */
export type TabState = TerminalTabState | SettingsTabState | DiffTabState

/**
 * Diff tab 持久化数据
 */
export interface PersistedDiffTabData {
  filePath: string
  staged?: boolean
  commitHash?: string
}

/**
 * Diff tab 状态（持久化）
 */
export interface DiffTabState {
  type: 'diff'
  id: number
  isActive: boolean
  data: PersistedDiffTabData
}

/**
 * 运行时终端状态（从后端查询）
 */
export interface RuntimeTerminalState {
  id: number
  cwd: string
  shell: string
}

export interface UiState {
  theme: string
  fontSize: number
  sidebarWidth: number
  leftSidebarVisible?: boolean
  leftSidebarWidth?: number
  leftSidebarActivePanel?: 'workspace' | 'git' | null
  onboardingCompleted?: boolean
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
  selectedModelId?: string | null
}

/**
 * 会话状态 - 统一 tab 管理
 */
export interface SessionState {
  version: number
  tabs: TabState[]
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
