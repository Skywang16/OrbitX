/**
 * API 模块主入口
 *
 * 统一导出所有 API 接口，提供简洁的访问方式
 */

// 重新导出请求相关功能
export * from '../utils/request'

// 导出各个功能模块的 API 实例
export { aiApi } from './ai'
export { configApi } from './config'
export { storageApi } from './storage'
export { terminalApi } from './terminal'
export { shellApi } from './shell'
export { shortcutsApi } from './shortcuts'
export { completionApi } from './completion'
export { windowApi } from './window'

// 导出所有类型（避免冲突）
export type { AiApi } from './ai'
export type { ConfigApi } from './config'
export type { StorageApi } from './storage'
export type { TerminalApi } from './terminal'
export type { ShellApi } from './shell'
export type { ShortcutsApi } from './shortcuts'
export type { CompletionApi } from './completion'
export type { WindowApi } from './window'

// 重新导出需要的类型
export type {
  AIModelConfig,
  AISettings,
  AIStats,
  AIHealthStatus,
  Conversation,
  Message,
  AnalyzeCodeParams,
  AnalysisResult,
  CodeSymbol,
  WebFetchRequest,
  WebFetchResponse,
} from './ai'

export type { AppConfig, ConfigFileInfo } from './config'

export { ConfigApiError } from './config/types'

export type { ShellInfo } from './shell'

export type {
  TerminalCreateOptions,
  TerminalWriteOptions,
  TerminalResizeOptions,
  CreateTerminalWithShellOptions,
} from './terminal'

export type {
  ShortcutsConfig,
  ShortcutBinding,
  ShortcutCategory,
  Platform,
  ShortcutValidationResult,
  ConflictDetectionResult,
  ShortcutStatistics,
  ShortcutSearchOptions,
  ShortcutOperationOptions,
  ShortcutFormatOptions,
} from './shortcuts'

export type { CompletionRequest, CompletionResponse, CompletionStats, CompletionEngineStatus } from './completion'

export type {
  WindowState,
  CompleteWindowState,
  DirectoryOptions,
  PathInfo,
  PlatformInfo,
  WindowStateBatchRequest,
  WindowStateBatchResponse,
} from './window'
