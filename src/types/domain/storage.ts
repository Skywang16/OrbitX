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

export type EditorSplitDirection = 'row' | 'column'

export type GroupId = string

export const createGroupId = (prefix: string): GroupId => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `${prefix}:${crypto.randomUUID()}`
  }
  return `${prefix}:${Date.now()}-${Math.random().toString(16).slice(2)}`
}

export interface GroupLeafNode {
  type: 'leaf'
  id: string
  groupId: GroupId
}

export interface GroupSplitNode {
  type: 'split'
  id: string
  direction: EditorSplitDirection
  ratio: number
  first: GroupNode
  second: GroupNode
}

export type GroupNode = GroupLeafNode | GroupSplitNode

/**
 * Tab ID - 统一使用 string（稳定标识，不与运行时 paneId 绑定）
 */
export type TabId = string

export const createTabId = (prefix: string): TabId => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `${prefix}:${crypto.randomUUID()}`
  }
  return `${prefix}:${Date.now()}-${Math.random().toString(16).slice(2)}`
}

/**
 * Tab 上下文（Context）是一等公民：所有“归属/工作区/仓库/运行时资源”的信息都在这里。
 * 业务层只看 context，不看 tab.type / tab.data。
 */
export type TabContext =
  | { kind: 'none' }
  | { kind: 'terminal'; paneId: number }
  | { kind: 'workspace'; path: string }
  | { kind: 'git'; repoPath: string }

export interface BaseTabState<TType extends string, TContext extends TabContext, TData> {
  type: TType
  id: TabId
  isActive: boolean
  context: TContext
  data: TData
}

/**
 * Terminal tab 状态（持久化）
 */
export interface TerminalTabData {
  cwd?: string
  shellName?: string
}

export type TerminalTabState = BaseTabState<'terminal', { kind: 'terminal'; paneId: number }, TerminalTabData>

/**
 * Settings tab 持久化数据
 */
export interface PersistedSettingsTabData {
  lastSection?: string
}

/**
 * Settings tab 状态（持久化）
 */
export type SettingsTabState = BaseTabState<'settings', { kind: 'none' }, PersistedSettingsTabData>

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
export type DiffTabState = BaseTabState<'diff', { kind: 'git'; repoPath: string }, PersistedDiffTabData>

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

export interface TabGroupState {
  id: GroupId
  tabs: TabState[]
  activeTabId: TabId | null
}

export interface WorkspaceState {
  root: GroupNode
  groups: Record<GroupId, TabGroupState>
  activeGroupId: GroupId
}

/**
 * 会话状态 - VSCode 风格分区（Group）+ 每区 tabs
 */
export interface SessionState {
  version: number
  workspace: WorkspaceState
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
