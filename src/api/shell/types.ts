/**
 * Shell模块相关的类型定义
 */

// ===== Shell相关类型 =====

export interface ShellInfo {
  name: string
  path: string
  displayName: string
  args?: string[]
}

export interface CreateTerminalWithShellOptions {
  shellName?: string
  rows: number
  cols: number
}

// ===== Shell功能相关类型 =====

export interface ShellFeatures {
  supportsColors: boolean
  supportsUnicode: boolean
  supportsTabCompletion: boolean
  supportsHistory: boolean
  supportsAliases: boolean
  supportsScripting: boolean
}

// ===== Shell配置相关类型 =====

export interface ShellConfig {
  name: string
  path: string
  args: string[]
  env: Record<string, string>
  workingDirectory?: string
}

// ===== Shell统计信息类型 =====

export interface ShellStats {
  totalShells: number
  availableShells: number
  defaultShell: string
  lastRefresh: string
}

// ===== 后台命令执行相关类型 =====

export interface BackgroundCommandResult {
  program: string
  args: string[]
  exitCode: number
  stdout: string
  stderr: string
  executionTimeMs: number
  success: boolean
}

// ===== Shell管理器统计类型 =====

export interface ShellManagerStats {
  initialized: boolean
  shellCount: number
  cacheHitRate: number
  lastUpdate: string
}

// ===== Shell验证结果类型 =====

export interface ShellValidationResult {
  valid: boolean
  path: string
  error?: string
  features?: ShellFeatures
}

// ===== Shell搜索结果类型 =====

export interface ShellSearchResult {
  shells: ShellInfo[]
  query: string
  totalFound: number
}

// ===== Shell操作结果类型 =====

export interface ShellOperationResult<T = void> {
  success: boolean
  data?: T
  error?: string
}

// ===== Shell推荐类型 =====

export interface ShellRecommendation {
  shell: ShellInfo
  score: number
  reasons: string[]
  features: ShellFeatures
}

// ===== Shell启动参数类型 =====

export interface ShellStartupArgs {
  shell: ShellInfo
  args: string[]
  env: Record<string, string>
}

// ===== Shell配置路径类型 =====

export interface ShellConfigPaths {
  shell: ShellInfo
  configFiles: string[]
  profileFiles: string[]
  historyFiles: string[]
}
