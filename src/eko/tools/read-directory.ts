/**
 * 目录读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { FileNotFoundError } from './tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface ReadDirectoryParams {
  directoryPath: string
  showHidden?: boolean
  recursive?: boolean
  maxDepth?: number
  sortBy?: 'name' | 'size' | 'modified'
  sortOrder?: 'asc' | 'desc'
}

export interface FileEntry {
  name: string
  path: string
  isDirectory: boolean
  size?: number
  modified?: string
}

/**
 * 目录读取工具
 */
export class ReadDirectoryTool extends ModifiableTool {
  constructor() {
    super('read_directory', '📁 读取目录内容：列出目录中的文件和子目录，支持递归、排序、隐藏文件显示', {
      type: 'object',
      properties: {
        directoryPath: {
          type: 'string',
          description: '要读取的目录路径',
        },
        showHidden: {
          type: 'boolean',
          description: '是否显示隐藏文件（以.开头的文件），默认false',
          default: false,
        },
        recursive: {
          type: 'boolean',
          description: '是否递归读取子目录，默认false',
          default: false,
        },
        maxDepth: {
          type: 'number',
          description: '递归的最大深度，仅在recursive为true时有效，默认3',
          default: 3,
          minimum: 1,
          maximum: 10,
        },
        sortBy: {
          type: 'string',
          enum: ['name', 'size', 'modified'],
          description: '排序方式：name(名称)、size(大小)、modified(修改时间)，默认name',
          default: 'name',
        },
        sortOrder: {
          type: 'string',
          enum: ['asc', 'desc'],
          description: '排序顺序：asc(升序)、desc(降序)，默认asc',
          default: 'asc',
        },
      },
      required: ['directoryPath'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const {
      directoryPath,
      showHidden = false,
      recursive = false,
      maxDepth = 3,
      sortBy = 'name',
      sortOrder = 'asc',
    } = context.parameters as unknown as ReadDirectoryParams

    try {
      // 检查目录是否存在
      const exists = await this.checkPathExists(directoryPath)
      if (!exists) {
        throw new FileNotFoundError(directoryPath)
      }

      // 读取目录内容
      const entries = await this.readDirectoryRecursive(directoryPath, showHidden, recursive, maxDepth, 0)

      // 排序并格式化输出
      const sortedEntries = this.sortEntries(entries, sortBy, sortOrder)
      const output = this.formatDirectoryOutput(sortedEntries, directoryPath, recursive)

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
      await invoke('plugin:fs|exists', { path })
      return true
    } catch {
      return false
    }
  }

  private async readDirectoryRecursive(
    dirPath: string,
    showHidden: boolean,
    recursive: boolean,
    maxDepth: number,
    currentDepth: number
  ): Promise<FileEntry[]> {
    const entries: FileEntry[] = []

    try {
      // 使用Tauri API读取目录
      const dirEntries = await invoke<Array<{ name: string; path: string }>>('plugin:fs|read_dir', {
        dir: dirPath,
      })

      for (const entry of dirEntries) {
        // 过滤隐藏文件
        if (!showHidden && entry.name.startsWith('.')) {
          continue
        }

        try {
          // 获取文件/目录信息
          const metadata = await invoke<{
            isDir: boolean
            size: number
            modified: number
          }>('plugin:fs|metadata', {
            path: entry.path,
          })

          const fileEntry: FileEntry = {
            name: entry.name,
            path: entry.path,
            isDirectory: metadata.isDir,
            size: metadata.size,
            modified: new Date(metadata.modified * 1000).toISOString(),
          }

          entries.push(fileEntry)

          // 递归处理子目录
          if (recursive && metadata.isDir && currentDepth < maxDepth) {
            const subEntries = await this.readDirectoryRecursive(
              entry.path,
              showHidden,
              recursive,
              maxDepth,
              currentDepth + 1
            )
            entries.push(...subEntries)
          }
        } catch (error) {
          // 跳过无法访问的文件/目录
          console.warn(`无法访问 ${entry.path}:`, error)
        }
      }
    } catch (error) {
      throw new Error(`读取目录 ${dirPath} 失败: ${error instanceof Error ? error.message : String(error)}`)
    }

    return entries
  }

  private sortEntries(entries: FileEntry[], sortBy: string, sortOrder: string): FileEntry[] {
    return entries.sort((a, b) => {
      let comparison = 0

      switch (sortBy) {
        case 'name':
          comparison = a.name.localeCompare(b.name)
          break
        case 'size':
          comparison = (a.size || 0) - (b.size || 0)
          break
        case 'modified':
          comparison = new Date(a.modified || 0).getTime() - new Date(b.modified || 0).getTime()
          break
        default:
          comparison = a.name.localeCompare(b.name)
      }

      return sortOrder === 'desc' ? -comparison : comparison
    })
  }

  private formatDirectoryOutput(entries: FileEntry[], basePath: string, recursive: boolean): string {
    if (entries.length === 0) {
      return `目录 ${basePath} 为空`
    }

    const lines: string[] = []
    lines.push(`📁 目录内容: ${basePath}`)
    lines.push(`总计: ${entries.length} 项`)
    lines.push('')

    // 按目录和文件分组
    const directories = entries.filter(e => e.isDirectory)
    const files = entries.filter(e => !e.isDirectory)

    // 显示目录
    if (directories.length > 0) {
      lines.push('📁 目录:')
      for (const dir of directories) {
        const relativePath = recursive ? dir.path.replace(basePath, '.') : dir.name
        lines.push(`  📁 ${relativePath}`)
      }
      lines.push('')
    }

    // 显示文件
    if (files.length > 0) {
      lines.push('📄 文件:')
      for (const file of files) {
        const relativePath = recursive ? file.path.replace(basePath, '.') : file.name
        const size = file.size ? this.formatFileSize(file.size) : ''
        const modified = file.modified ? new Date(file.modified).toLocaleDateString() : ''
        lines.push(`  📄 ${relativePath} ${size} ${modified}`.trim())
      }
    }

    return lines.join('\n')
  }

  private formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `(${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]})`
  }
}

// 导出工具实例
export const readDirectoryTool = new ReadDirectoryTool()
