/**
 * 目录读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { FileNotFoundError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface ReadDirectoryParams {
  directoryPath: string
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
    super('read_directory', '列出目录内容：查看指定目录中的文件和子目录', {
      type: 'object',
      properties: {
        directoryPath: {
          type: 'string',
          description: '目录路径',
        },
      },
      required: ['directoryPath'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { directoryPath } = context.parameters as unknown as ReadDirectoryParams

    try {
      // 检查目录是否存在
      const exists = await this.checkPathExists(directoryPath)
      if (!exists) {
        throw new FileNotFoundError(directoryPath)
      }

      // 读取目录内容
      const entries = await this.readDirectorySimple(directoryPath)

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
      throw new Error(`读取目录失败: ${error instanceof Error ? error.message : String(error)}`)
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

    const lines: string[] = []

    // 按目录和文件分组
    const directories = entries.filter(e => e.isDirectory)
    const files = entries.filter(e => !e.isDirectory)

    // 显示目录
    for (const dir of directories) {
      lines.push(`目录: ${dir.name}/`)
    }

    // 显示文件
    for (const file of files) {
      lines.push(`文件: ${file.name}`)
    }

    return lines.join('\n')
  }
}

// 导出工具实例
export const readDirectoryTool = new ReadDirectoryTool()
