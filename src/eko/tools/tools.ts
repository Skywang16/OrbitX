/**
 * 工具集合
 */

import type { Tool } from '../types'
import { globalToolRegistry } from './tool-registry'

// 导入所有工具
import { readFileTool } from './toolList/read-file'
import { readManyFilesTool } from './toolList/read-many-files'
import { readDirectoryTool } from './toolList/read-directory'
import { fileSystemTool } from './toolList/filesystem'
import { createFileTool } from './toolList/create-file'
import { editFileTool } from './toolList/edit-file'
import { shellTool } from './toolList/shell'
import { webFetchTool } from './toolList/web-fetch'

import { semanticSearchTool } from './toolList/semantic-search'

/**
 * 所有可用工具的数组
 */
export const allTools: Tool<unknown>[] = [
  // 文件操作工具
  readFileTool,
  readManyFilesTool,
  readDirectoryTool,
  createFileTool,
  editFileTool,

  // 文件系统工具
  fileSystemTool,

  // 系统工具
  shellTool,

  // 网络工具
  webFetchTool,

  // 搜索工具
  semanticSearchTool,
]

/**
 * 按分类组织的工具
 */
export const toolsByCategory = {
  file: [readFileTool, readManyFilesTool, readDirectoryTool, createFileTool, editFileTool],
  filesystem: [fileSystemTool],
  system: [shellTool],
  network: [webFetchTool],
  search: [semanticSearchTool],
}

/**
 * 核心工具（最常用的工具）
 */
export const coreTools: Tool[] = [
  readFileTool,
  readDirectoryTool,
  fileSystemTool,
  createFileTool,
  editFileTool,
  shellTool,
]

/**
 * 网络工具
 */
export const networkTools: Tool[] = [webFetchTool]

/**
 * 文件操作工具
 */
export const fileTools: Tool[] = [readFileTool, readManyFilesTool, readDirectoryTool, createFileTool, editFileTool]

/**
 * 文件系统工具
 */
export const fileSystemTools: Tool[] = [fileSystemTool]

/**
 * 搜索工具
 */
export const searchTools: Tool[] = [semanticSearchTool]

/**
 * 注册所有工具到全局注册表
 */
export function registerAllTools(): void {
  const toolsToRegister = [
    {
      tool: readFileTool,
      metadata: {
        description: readFileTool.description,
        category: 'file',
        tags: ['file', 'read', 'content'],
      },
    },
    {
      tool: readManyFilesTool,
      metadata: {
        description: readManyFilesTool.description,
        category: 'file',
        tags: ['file', 'read', 'batch', 'multiple'],
      },
    },
    {
      tool: readDirectoryTool,
      metadata: {
        description: readDirectoryTool.description,
        category: 'file',
        tags: ['directory', 'list', 'folder', 'filesystem'],
      },
    },
    {
      tool: fileSystemTool,
      metadata: {
        description: fileSystemTool.description,
        category: 'filesystem',
        tags: ['filesystem', 'info', 'metadata', 'permissions'],
      },
    },
    {
      tool: createFileTool,
      metadata: {
        description: createFileTool.description,
        category: 'file',
        tags: ['file', 'create', 'new'],
      },
    },
    {
      tool: editFileTool,
      metadata: {
        description: editFileTool.description,
        category: 'file',
        tags: ['file', 'edit', 'modify', 'replace', 'line'],
      },
    },
    {
      tool: shellTool,
      metadata: {
        description: shellTool.description,
        category: 'system',
        tags: ['shell', 'command', 'execute', 'terminal'],
      },
    },
    {
      tool: webFetchTool,
      metadata: {
        description: webFetchTool.description,
        category: 'network',
        tags: ['web', 'http', 'fetch', 'api'],
      },
    },
    {
      tool: semanticSearchTool,
      metadata: {
        description: semanticSearchTool.description,
        category: 'search',
        tags: ['search', 'semantic', 'ast', 'code', 'text', 'symbol', 'intelligent'],
      },
    },
  ]

  globalToolRegistry.registerBatch(toolsToRegister)
}

/**
 * 按模式筛选工具
 * - chat 模式：仅允许读取类工具，禁止任何写入/执行类工具
 * - agent 模式：允许所有工具
 */
export function getToolsForMode(mode: 'chat' | 'agent'): Tool<unknown>[] {
  return mode === 'agent' ? allTools : [...fileTools, fileSystemTool, ...networkTools, ...searchTools]
}

// 自动注册所有工具
registerAllTools()
