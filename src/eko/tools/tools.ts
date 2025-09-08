/**
 * 工具集合
 */

import type { Tool } from '@/eko-core/types'
import { globalToolRegistry } from './tool-registry'

// Import all tools
import { readFileTool } from './toolList/read-file'
import { readManyFilesTool } from './toolList/read-many-files'
import { readDirectoryTool } from './toolList/read-directory'

import { createFileTool } from './toolList/create-file'
import { editFileTool } from './toolList/edit-file'
import { shellTool } from './toolList/shell'
import { webFetchTool } from './toolList/web-fetch'

import { orbitSearchTool } from './toolList/orbit-search'
import { grepSearchTool } from './toolList/grep-search'

// 动态导入向量索引功能检查
let isVectorIndexEnabled = false

// 检查向量索引是否启用
async function checkVectorIndexEnabled(): Promise<boolean> {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const settings = await invoke('get_vector_index_app_settings')
    return (settings as { enabled: boolean }).enabled || false
  } catch (error) {
    console.warn('检查向量索引状态失败:', error)
    return false
  }
}

// 初始化向量索引状态
checkVectorIndexEnabled().then(enabled => {
  isVectorIndexEnabled = enabled
})

/**
 * 获取基础工具列表（不包含向量索引工具）
 */
function getBaseReadOnlyTools(): Tool[] {
  return [readFileTool, readManyFilesTool, readDirectoryTool, webFetchTool, grepSearchTool]
}

function getBaseAllTools(): Tool[] {
  return [
    readFileTool,
    readManyFilesTool,
    readDirectoryTool,
    createFileTool,
    editFileTool,
    shellTool,
    webFetchTool,
    grepSearchTool,
  ]
}

/**
 * Read-only tools - available in Chat mode
 */
export const readOnlyTools: Tool[] = getBaseReadOnlyTools().concat(isVectorIndexEnabled ? [orbitSearchTool] : [])

/**
 * All tools - available in Agent mode
 */
export const allTools: Tool[] = getBaseAllTools().concat(isVectorIndexEnabled ? [orbitSearchTool] : [])

/**
 * Register all tools to global registry
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
    // 条件性注册orbitSearchTool
    ...(isVectorIndexEnabled
      ? [
          {
            tool: orbitSearchTool,
            metadata: {
              description: orbitSearchTool.description,
              category: 'search',
              tags: ['search', 'semantic', 'code', 'context', 'intelligent', 'analysis'],
            },
          },
        ]
      : []),
    {
      tool: grepSearchTool,
      metadata: {
        description: grepSearchTool.description,
        category: 'search',
        tags: ['search', 'grep', 'text', 'simple', 'direct', 'command'],
      },
    },
  ]

  globalToolRegistry.registerBatch(toolsToRegister)
}

/**
 * Filter tools by mode
 */
export function getToolsForMode(mode: 'chat' | 'agent'): Tool[] {
  return mode === 'agent' ? allTools : readOnlyTools
}

// 自动注册所有工具
registerAllTools()
