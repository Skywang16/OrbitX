/**
 * 终端会话相关类型定义
 */

// ===== Shell信息类型 =====

export interface ShellInfo {
  name: string // shell名称，如 "zsh", "bash"
  path: string // shell可执行文件路径
  displayName: string // 显示名称，如 "Z Shell (zsh)"
}

// ===== 终端会话类型 =====

export interface TerminalSession {
  id: string
  backendId: number | null
  title: string
  isActive: boolean
  shellInfo?: ShellInfo // 当前使用的shell信息
}

// ===== 终端输出类型 =====

export interface TerminalOutput {
  id: number
  data: string
}

// ===== 终端状态类型 =====

export interface TerminalState {
  waitingForInput: boolean
  lastActivity: number
}

export interface TerminalStateEvent {
  id: number
  waitingForInput: boolean
}

// ===== Shell管理器状态类型 =====

export interface ShellManagerState {
  availableShells: ShellInfo[]
  isLoading: boolean
  error: string | null
}
