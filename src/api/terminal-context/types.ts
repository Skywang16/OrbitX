/**
 * 终端上下文相关的类型定义
 */

// ===== 终端上下文类型 =====

export interface TerminalContext {
  paneId: number
  currentWorkingDirectory: string | null
  shellType: string | null
  shellIntegrationEnabled: boolean
  currentCommand: CommandInfo | null
  commandHistory: CommandInfo[]
  windowTitle: string | null
  lastActivity: string // ISO 8601 timestamp
  isActive: boolean
}

// ===== 命令信息类型 =====

export interface CommandInfo {
  command: string
  args: string[]
  startTime: string // ISO 8601 timestamp
  endTime: string | null // ISO 8601 timestamp
  exitCode: number | null
  workingDirectory: string | null
}

// ===== 终端上下文事件类型 =====

export interface TerminalContextEvent {
  type: 'activePaneChanged' | 'paneContextUpdated' | 'paneCwdChanged' | 'paneShellIntegrationChanged'
  data: unknown
}

export interface ActivePaneChangedEvent {
  type: 'activePaneChanged'
  data: {
    oldPaneId: number | null
    newPaneId: number | null
  }
}

export interface PaneContextUpdatedEvent {
  type: 'paneContextUpdated'
  data: {
    paneId: number
    context: TerminalContext
  }
}

export interface PaneCwdChangedEvent {
  type: 'paneCwdChanged'
  data: {
    paneId: number
    oldCwd: string | null
    newCwd: string
  }
}

export interface PaneShellIntegrationChangedEvent {
  type: 'paneShellIntegrationChanged'
  data: {
    paneId: number
    enabled: boolean
  }
}

// ===== 查询选项类型 =====

export interface ContextQueryOptions {
  useCache?: boolean
  timeout?: number
  allowFallback?: boolean
  includeHistory?: boolean
  maxHistoryCount?: number
}

// ===== 缓存统计类型 =====

export interface CacheStats {
  totalEntries: number
  hitCount: number
  missCount: number
  evictionCount: number
  hitRate: number
}

// ===== API 响应类型 =====

export interface TerminalContextResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
  code?: string
}
