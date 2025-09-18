/**
 * 工具集合
 */

import type { Tool } from '@/eko-core/types'
import { globalToolRegistry } from './tool-registry'

// Import all tools
import { readFileTool } from './toolList/read-file'
import { readManyFilesTool } from './toolList/read-many-files'

import { writeToFileTool } from './toolList/write-to-file'
import { editFileTool } from './toolList/edit-file'
import { applyDiffTool } from './toolList/apply-diff'
import { insertContentTool } from './toolList/insert-content'
import { listFilesTool } from './toolList/list-files'
import { listCodeDefinitionNamesTool } from './toolList/list-code-definition-names'
import { shellTool } from './toolList/shell'
import { webFetchTool } from './toolList/web-fetch'

import { orbitSearchTool } from './toolList/orbit-search'

/**
 * 获取基础工具列表（不包含向量索引工具）
 */
function getBaseReadOnlyTools(): Tool[] {
  return [readFileTool, readManyFilesTool, listFilesTool, listCodeDefinitionNamesTool, webFetchTool, orbitSearchTool]
}

function getBaseAllTools(): Tool[] {
  return [
    readFileTool,
    readManyFilesTool,
    listFilesTool,
    listCodeDefinitionNamesTool,
    editFileTool,
    applyDiffTool,
    insertContentTool,
    writeToFileTool,
    shellTool,
    webFetchTool,
    orbitSearchTool,
  ]
}

/**
 * Read-only tools - available in Chat mode
 */
export const readOnlyTools: Tool[] = getBaseReadOnlyTools()

/**
 * All tools - available in Agent mode
 */
export const allTools: Tool[] = getBaseAllTools()

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
      tool: listFilesTool,
      metadata: {
        description: listFilesTool.description,
        category: 'file',
        tags: ['directory', 'list', 'non-recursive', 'gitignore', 'ignore'],
      },
    },

    {
      tool: listCodeDefinitionNamesTool,
      metadata: {
        description: listCodeDefinitionNamesTool.description,
        category: 'code',
        tags: ['code', 'definitions', 'symbols', 'ts', 'js'],
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
      tool: applyDiffTool,
      metadata: {
        description: applyDiffTool.description,
        category: 'file',
        tags: ['file', 'diff', 'multi-file', 'hunk', 'preview', 'approval'],
      },
    },
    {
      tool: insertContentTool,
      metadata: {
        description: insertContentTool.description,
        category: 'file',
        tags: ['file', 'insert', 'line', 'append', 'preview', 'approval'],
      },
    },
    {
      tool: writeToFileTool,
      metadata: {
        description: writeToFileTool.description,
        category: 'file',
        tags: ['file', 'write', 'overwrite', 'create', 'preview', 'approval'],
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
        tags: ['search', 'semantic', 'vector', 'ai', 'code', 'orbit'],
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
