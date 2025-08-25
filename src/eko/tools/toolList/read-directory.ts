/**
 * 目录读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@eko-ai/eko/types'
import { FileNotFoundError, ToolError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface ReadDirectoryParams {
  path: string
}

export interface FileEntry {
  name: string
  path: string
  isDirectory: boolean
}

/**
 * 目录读取工具
 */
export class ReadDirectoryTool extends ModifiableTool {
  constructor() {
    super(
      'read_directory',
      `递归列出目录中的所有文件和子目录，最多5层深度。以树形结构显示，目录以"/"结尾，会自动过滤隐藏文件。输出为LLM友好的树形格式。必须使用绝对路径。`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description:
              '目录的绝对路径。必须是完整路径，例如："/Users/user/project/src"、"/home/user/workspace/components"',
          },
        },
        required: ['path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path } = context.parameters as unknown as ReadDirectoryParams

    try {
      // 检查目录是否存在
      const exists = await this.checkPathExists(path)
      if (!exists) {
        throw new FileNotFoundError(path)
      }

      // 递归读取目录内容（最多5层）
      const entries = await this.readDirectoryRecursive(path, 0, 5)

      // 格式化为树形输出
      const output = await this.formatTreeOutput(path, entries)

      return {
        content: [
          {
            type: 'text',
            text: output,
          },
        ],
      }
    } catch (error) {
      if (error instanceof FileNotFoundError) {
        throw error
      }
      throw new ToolError(`读取目录失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async checkPathExists(path: string): Promise<boolean> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      return exists
    } catch (error) {
      return false
    }
  }

  private async readDirectoryRecursive(dirPath: string, currentDepth: number, maxDepth: number): Promise<FileEntry[]> {
    const entries: FileEntry[] = []

    if (currentDepth >= maxDepth) {
      return entries
    }

    try {
      // 使用Tauri API读取目录
      const dirEntries = await invoke<
        Array<{
          name: string
          isDirectory: boolean
          isFile: boolean
          isSymlink: boolean
        }>
      >('plugin:fs|read_dir', {
        path: dirPath,
      })

      for (const entry of dirEntries) {
        // 过滤隐藏文件
        if (entry.name.startsWith('.')) {
          continue
        }

        // 构建完整路径
        const fullPath = `${dirPath}/${entry.name}`.replace(/\/+/g, '/')

        const fileEntry: FileEntry = {
          name: entry.name,
          path: fullPath,
          isDirectory: entry.isDirectory,
        }

        entries.push(fileEntry)
      }

      // 对目录排序，目录在前，文件在后
      entries.sort((a, b) => {
        if (a.isDirectory && !b.isDirectory) return -1
        if (!a.isDirectory && b.isDirectory) return 1
        return a.name.localeCompare(b.name)
      })
    } catch (error) {
      // 如果读取失败，返回已有的entries
      console.warn(`读取目录 ${dirPath} 失败: ${error instanceof Error ? error.message : String(error)}`)
    }

    return entries
  }

  private async formatTreeOutput(rootPath: string, entries: FileEntry[]): Promise<string> {
    if (entries.length === 0) {
      return `目录为空`
    }

    const lines: string[] = []
    const rootName = rootPath.split('/').pop() || rootPath
    lines.push(`${rootName}/`)

    const totalItems = await this.buildTree(rootPath, entries, lines, '', 0, 5)

    // 添加LLM友好的提示
    const MAX_DISPLAY_ITEMS = 1000
    if (totalItems > MAX_DISPLAY_ITEMS) {
      lines.push('')
      lines.push(`重要提示：目录结构已被截断（最多5层深度）。`)
      lines.push(`状态：显示了部分内容，实际项目可能包含更多文件。`)
      lines.push(`建议：如需查看特定文件，请使用 read_file 工具。`)
    }

    return lines.join('\n')
  }

  private async buildTree(
    _currentPath: string,
    entries: FileEntry[],
    lines: string[],
    prefix: string,
    currentDepth: number,
    maxDepth: number
  ): Promise<number> {
    if (currentDepth >= maxDepth) {
      return 0
    }

    let totalItems = entries.length

    for (let i = 0; i < entries.length; i++) {
      const entry = entries[i]
      const isLast = i === entries.length - 1
      const currentPrefix = isLast ? '└── ' : '├── '
      const nextPrefix = prefix + (isLast ? '    ' : '│   ')

      if (entry.isDirectory) {
        lines.push(`${prefix}${currentPrefix}${entry.name}/`)

        // 递归读取子目录
        try {
          const subEntries = await this.readDirectoryRecursive(entry.path, currentDepth + 1, maxDepth)
          if (subEntries.length > 0) {
            const subTotal = await this.buildTree(entry.path, subEntries, lines, nextPrefix, currentDepth + 1, maxDepth)
            totalItems += subTotal
          }
        } catch (error) {
          // 忽略无法读取的目录
        }
      } else {
        lines.push(`${prefix}${currentPrefix}${entry.name}`)
      }
    }

    return totalItems
  }
}

// 导出工具实例
export const readDirectoryTool = new ReadDirectoryTool()
