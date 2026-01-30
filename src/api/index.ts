/**
 * API 统一导出文件
 *
 * 将所有子模块的API实例和类型统一导出，
 * 提供便捷的 '@/api' 导入方式
 */

export { aiApi } from './ai'
export { agentApi } from './agent'
export { agentTerminalApi } from './agent-terminal'
export { appApi } from './app'
export { vectorDbApi } from './vector-db'
export { completionApi } from './completion'
export { configApi } from './config'
export { dockApi } from './dock'
export { filesystemApi } from './filesystem'
export { fileWatcherApi } from './file-watcher'
export { gitApi } from './git'
export { llmApi } from './llm'
export { llmRegistryApi } from './llm-registry'
export { mcpApi } from './mcp'
export { nodeApi } from './node'
export { shellApi } from './shell'
export { shellIntegrationApi } from './shellIntegration'
export { shortcutsApi } from './shortcuts'
export { settingsApi } from './settings'
export { storageApi } from './storage'
export { terminalApi } from './terminal'
export { terminalContextApi } from './terminal-context'
export { windowApi } from './window'
export { workspaceApi } from './workspace'
export { codeApi } from './code'

export type * from './ai/types'
export type * from './agent/types'
export type * from './completion/types'
export type * from './git/types'
export type * from './mcp/types'
export type * from './shortcuts/types'
export type * from './storage/types'
export type * from './terminal-context/types'
export type { CodeDefinition } from './code'
export type { TabEntry } from './dock'

// 从config导出但排除与terminal重复的类型
export type { AppConfig, ConfigFileInfo } from './config/types'
// 从shell导出但排除与terminal重复的类型
export type { ShellInfo, BackgroundCommandResult } from './shell/types'
// 从terminal导出所有类型
export type * from './terminal/types'

export type { AgentApi } from './agent'
export type { AppApi } from './app'
export type { FilesystemApi } from './filesystem'
export type { LLMApi } from './llm'
export type { LLMRegistryApi } from './llm-registry'
export type { NodeApi, NodeVersionInfo } from './node'
export type { ShellIntegrationApi } from './shellIntegration'
export type { WindowApi } from './window'
