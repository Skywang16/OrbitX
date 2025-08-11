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
 * 按模式筛选工具
 * - chat 模式：仅允许读取类工具，禁止任何写入/执行类工具
 * - agent 模式：允许所有工具
 */
export function getToolsForMode(mode: 'chat' | 'agent'): Tool[] {
  if (mode === 'agent') return allTools

  // 只读集合：文件读取、多文件读取、网络获取/搜索
  return [readFileTool, readManyFilesTool, webFetchTool, webSearchTool]
}

// 自动注册所有工具
registerAllTools()
