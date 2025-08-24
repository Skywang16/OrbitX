/**
 * 工具集合
 */

import type { Tool } from '../types'
import { globalToolRegistry } from './tool-registry'

// 导入所有工具
import { readFileTool } from './toolList/read-file'
import { readManyFilesTool } from './toolList/read-many-files'
import { readDirectoryTool } from './toolList/read-directory'

import { createFileTool } from './toolList/create-file'
import { editFileTool } from './toolList/edit-file'
import { shellTool } from './toolList/shell'
import { webFetchTool } from './toolList/web-fetch'

import { orbitSearchTool } from './toolList/orbit-search'

/**
 * 只读工具 - Chat模式可以使用
 */
export const readOnlyTools: Tool[] = [readFileTool, readManyFilesTool, readDirectoryTool, webFetchTool, orbitSearchTool]

/**
 * 所有工具 - Agent模式可以使用
 */
export const allTools: Tool[] = [
  readFileTool,
  readManyFilesTool,
  readDirectoryTool,
  createFileTool,
  editFileTool,
  shellTool,
  webFetchTool,
  orbitSearchTool,
]

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
        tags: ['directory', 'list', 'folder'],
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
      tool: orbitSearchTool,
      metadata: {
        description: orbitSearchTool.description,
        category: 'search',
        tags: ['search', 'semantic', 'code', 'context', 'intelligent', 'analysis'],
      },
    },
  ]

  globalToolRegistry.registerBatch(toolsToRegister)
}

/**
 * 按模式筛选工具
 */
export function getToolsForMode(mode: 'chat' | 'agent'): Tool[] {
  return mode === 'agent' ? allTools : readOnlyTools
}

// 自动注册所有工具
registerAllTools()
