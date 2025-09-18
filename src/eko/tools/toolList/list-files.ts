/**
 * list_files 工具
 *
 * 非递归列出指定目录下的直接文件与子目录名称，
 * 支持忽略模式与 .gitignore（同级目录）过滤，目录优先排序。
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { filesystemApi, terminalContextApi } from '@/api'
import { ValidationError, ToolError } from '../tool-error'

export interface ListFilesParams {
  path: string
  recursive?: boolean
}

export class ListFilesTool extends ModifiableTool {
  constructor() {
    super(
      'list_files',
      `List files and directories within the specified directory. If recursive is true, lists all files and directories recursively. If recursive is false or not provided, only lists the top-level contents. Do not use this tool to confirm the existence of files you may have created.`,
      {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'Directory path (relative or absolute)' },
          recursive: { type: 'boolean', description: 'List recursively if true' },
        },
        required: ['path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as ListFilesParams

    const inputPath = (params.path || '').toString().trim()
    if (!inputPath) throw new ValidationError('Directory path cannot be empty')
    const dir = await resolveToAbsolute(inputPath)

    const recursive = params.recursive === true
    // 统一走后端命令，完整 .gitignore 语义由后端实现
    let list: string[]
    try {
      list = await filesystemApi.listDirectory(dir, recursive)
    } catch (e) {
      throw new ToolError(`Failed to read directory: ${e instanceof Error ? e.message : String(e)}`)
    }

    const header = `Directory listing for ${dir} (${recursive ? 'recursive' : 'non-recursive'}, ${list.length} entries):`
    const lines = list

    return {
      content: [
        {
          type: 'text',
          text: `${header}\n${lines.join('\n')}`,
        },
      ],
      extInfo: {
        path: dir,
        count: list.length,
        respectGitIgnore: true,
        includeHidden: true,
        ignoredPatterns: [],
      },
    }
  }
}

export const listFilesTool = new ListFilesTool()

// ===== 工具函数 =====
function isAbsolutePath(p: string): boolean {
  return p.startsWith('/')
}

async function resolveToAbsolute(input: string): Promise<string> {
  if (isAbsolutePath(input)) return input
  try {
    const cwd = await terminalContextApi.getCurrentWorkingDirectory()
    if (!cwd) throw new Error('No active terminal CWD')
    return normalizePath(`${cwd}/${input}`)
  } catch (e) {
    throw new ValidationError(
      `Cannot resolve relative path '${input}'. Please provide an absolute path or set an active terminal with a working directory.`
    )
  }
}

// 规范化路径（支持 .. 和 .）
function normalizePath(p: string): string {
  const isAbs = p.startsWith('/')
  const parts = p.split('/').filter(Boolean)
  const stack: string[] = []
  for (const part of parts) {
    if (part === '.') continue
    if (part === '..') {
      if (stack.length > 0) stack.pop()
    } else {
      stack.push(part)
    }
  }
  return (isAbs ? '/' : '') + stack.join('/')
}
// 递归收集逻辑已移除，统一交由后端
