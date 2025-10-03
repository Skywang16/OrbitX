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
 * 终端持久化状态（存 MessagePack）
 *
 * 设计原则：
 * - 只存恢复终端所需的最小信息
 * - id 直接用后端 pane_id（数字）
 * - 不存 cwd，启动时从后端 ShellIntegration 查询
 */
export interface TerminalState {
  /** 终端ID（后端 pane_id） */
  id: number
  /** 终端标题 */
  title: string
  /** 是否为活跃终端 */
  active: boolean
  /** Shell 类型 */
  shell?: string
}

export interface RuntimeTerminalState extends TerminalState {
  cwd: string
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
  // 注意：任务状态现在由TaskManager管理，不再存储在session中
}

export interface SessionState {
  version: number
  terminals: TerminalState[]
  activeTabId?: number | string
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
