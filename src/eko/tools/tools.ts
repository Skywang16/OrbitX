/**
 * 工具集合
 */

import type { Tool } from '../types'
import { globalToolRegistry } from './tool-registry'

// 导入所有工具
import { readFileTool } from './read-file'
import { readManyFilesTool } from './read-many-files'
import { writeFileTool } from './write-file'
import { shellTool } from './shell'
import { webFetchTool } from './web-fetch'
import { webSearchTool } from './web-search'
import { memoryTool } from './memoryTool'

/**
 * 所有可用工具的数组
 */
export const allTools: Tool[] = [
  // 文件操作工具
  readFileTool,
  readManyFilesTool,
  writeFileTool,

  // 系统工具
  shellTool,

  // 网络工具
  webFetchTool,
  webSearchTool,

  // 内存管理工具
  memoryTool,
]

/**
 * 按分类组织的工具
 */
export const toolsByCategory = {
  file: [readFileTool, readManyFilesTool, writeFileTool],
  system: [shellTool],
  network: [webFetchTool, webSearchTool],
  memory: [memoryTool],
}

/**
 * 核心工具（最常用的工具）
 */
export const coreTools: Tool[] = [readFileTool, writeFileTool, shellTool, memoryTool]

/**
 * 网络工具
 */
export const networkTools: Tool[] = [webFetchTool, webSearchTool]

/**
 * 文件操作工具
 */
export const fileTools: Tool[] = [readFileTool, readManyFilesTool, writeFileTool]

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
        version: '1.0.0',
        tags: ['file', 'read', 'content'],
      },
    },
    {
      tool: readManyFilesTool,
      metadata: {
        description: readManyFilesTool.description,
        category: 'file',
        version: '1.0.0',
        tags: ['file', 'read', 'batch', 'multiple'],
      },
    },
    {
      tool: writeFileTool,
      metadata: {
        description: writeFileTool.description,
        category: 'file',
        version: '1.0.0',
        tags: ['file', 'write', 'create'],
      },
    },
    {
      tool: shellTool,
      metadata: {
        description: shellTool.description,
        category: 'system',
        version: '1.0.0',
        tags: ['shell', 'command', 'execute', 'terminal'],
      },
    },
    {
      tool: webFetchTool,
      metadata: {
        description: webFetchTool.description,
        category: 'network',
        version: '1.0.0',
        tags: ['web', 'http', 'fetch', 'api'],
      },
    },
    {
      tool: webSearchTool,
      metadata: {
        description: webSearchTool.description,
        category: 'network',
        version: '1.0.0',
        tags: ['web', 'search', 'internet', 'information'],
      },
    },
    {
      tool: memoryTool,
      metadata: {
        description: memoryTool.description,
        category: 'memory',
        version: '1.0.0',
        tags: ['memory', 'storage', 'cache', 'data'],
      },
    },
  ]

  globalToolRegistry.registerBatch(toolsToRegister)
}

/**
 * 获取工具by名称
 */
export function getToolByName(name: string): Tool | undefined {
  return allTools.find(tool => tool.name === name)
}

/**
 * 获取工具by分类
 */
export function getToolsByCategory(category: keyof typeof toolsByCategory): Tool[] {
  return toolsByCategory[category] || []
}

/**
 * 搜索工具
 */
export function searchTools(query: string): Tool[] {
  const lowerQuery = query.toLowerCase()
  return allTools.filter(
    tool => tool.name.toLowerCase().includes(lowerQuery) || tool.description.toLowerCase().includes(lowerQuery)
  )
}

/**
 * 获取工具统计信息
 */
export function getToolsStats(): {
  total: number
  byCategory: Record<string, number>
  mostUsed: string[]
} {
  const total = allTools.length
  const byCategory: Record<string, number> = {}

  for (const [category, tools] of Object.entries(toolsByCategory)) {
    byCategory[category] = tools.length
  }

  // 假设的使用频率排序（实际应用中可以根据真实使用数据排序）
  const mostUsed = ['read_file', 'write_file', 'shell', 'memory', 'web_fetch', 'read_many_files', 'web_search']

  return {
    total,
    byCategory,
    mostUsed,
  }
}

// 自动注册所有工具
registerAllTools()

