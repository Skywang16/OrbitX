export * from '../utils/request'
export { invoke, type ApiResponse } from '../utils/request'

export { aiApi } from './ai'
export { configApi } from './config'
export { storageApi } from './storage'
export { terminalApi } from './terminal'
export { terminalContextApi } from './terminal-context'
export { shellApi } from './shell'
export { shellIntegrationApi } from './shellIntegration'
export { shortcutsApi } from './shortcuts'
export { completionApi } from './completion'
export { windowApi } from './window'
export { llmRegistryApi } from './llm-registry'
export { llmApi } from './llm'
export { filesystemApi } from './filesystem'

export type { AiApi } from './ai'
export type { ConfigApi } from './config'
export type { StorageApi } from './storage'
export type { TerminalApi } from './terminal'
export type { TerminalContextApi } from './terminal-context'
export type { ShellApi } from './shell'
export type { ShellIntegrationApi } from './shellIntegration'
export type { ShortcutsApi } from './shortcuts'
export type { CompletionApi } from './completion'
export type { WindowApi } from './window'
export type { LLMRegistryApi } from './llm-registry'
export type { LLMApi } from './llm'
export type { FilesystemApi } from './filesystem'

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
  TerminalCreateOptions,
  TerminalWriteOptions,
  TerminalResizeOptions,
  CreateTerminalWithShellOptions,
  ShellInfo,
  ShortcutsConfig,
  ShortcutBinding,
  Platform,
  ShortcutValidationResult,
  ConflictDetectionResult,
  ShortcutStatistics,
  WindowState,
} from '@/types'

export type { CompletionRequest, CompletionResponse, CompletionItem } from '@/types'
