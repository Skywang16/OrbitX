/**
 * 目录读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@eko-ai/eko/types'
import { FileNotFoundError } from '../tool-error'
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
      `列出目录中的文件和子目录。显示目录下的文件和文件夹，目录名以"/"结尾区分。会自动过滤隐藏文件。只显示第一层内容，不递归遍历子目录。如果目录内容过多会显示前500项并提示总数。必须使用绝对路径。`,
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

      // 读取目录内容
      const entries = await this.readDirectorySimple(path)

      // 格式化输出
      const output = this.formatDirectoryOutput(entries)

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

  private async readDirectorySimple(dirPath: string): Promise<FileEntry[]> {
    const entries: FileEntry[] = []

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
    } catch (error) {
      throw new Error(`读取目录 ${dirPath} 失败: ${error instanceof Error ? error.message : String(error)}`)
    }

    return entries
  }

  private formatDirectoryOutput(entries: FileEntry[]): string {
    if (entries.length === 0) {
      return `目录为空`
    }

    const MAX_DISPLAY_ITEMS = 500 // 增加到500项
    const lines: string[] = []

    // 按目录和文件分组
    const directories = entries.filter(e => e.isDirectory)
    const files = entries.filter(e => !e.isDirectory)

    const totalItems = directories.length + files.length
    const shouldTruncate = totalItems > MAX_DISPLAY_ITEMS

    // 计算显示数量
    let displayDirs = directories
    let displayFiles = files

    if (shouldTruncate) {
      const dirCount = Math.min(directories.length, Math.floor(MAX_DISPLAY_ITEMS * 0.4)) // 40%给目录
      const fileCount = Math.min(files.length, MAX_DISPLAY_ITEMS - dirCount)

      displayDirs = directories.slice(0, dirCount)
      displayFiles = files.slice(0, fileCount)
    }

    // 显示目录
    for (const dir of displayDirs) {
      lines.push(`目录: ${dir.name}/`)
    }

    // 显示文件
    for (const file of displayFiles) {
      lines.push(`文件: ${file.name}`)
    }

    // 如果被截断，添加LLM友好的提示
    if (shouldTruncate) {
      lines.push('')
      lines.push(`重要提示：目录列表已被截断。`)
      lines.push(`状态：显示了 ${displayDirs.length + displayFiles.length} 项，总共 ${totalItems} 项。`)
      lines.push(`建议：目录包含更多文件。使用 read_file 工具检查感兴趣的特定文件。`)
    }

    return lines.join('\n')
  }
}

// 导出工具实例
export const readDirectoryTool = new ReadDirectoryTool()
