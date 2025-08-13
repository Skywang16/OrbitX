/**
 * 文件系统操作工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { FileNotFoundError } from './tool-error'
import { invoke } from '@tauri-apps/api/core'
import { formatFileTime } from '@/utils/dateFormatter'

export interface FileSystemParams {
  path: string
  operation: 'exists' | 'info' | 'type' | 'permissions'
}

export interface FileInfo {
  path: string
  exists: boolean
  isFile: boolean
  isDirectory: boolean
  size: number
  sizeFormatted: string
  created: string
  modified: string
  accessed: string
  permissions: {
    readable: boolean
    writable: boolean
    executable: boolean
  }
}

/**
 * 文件系统操作工具
 */
export class FileSystemTool extends ModifiableTool {
  constructor() {
    super(
      'filesystem',
      '🗂️ 文件信息查询：当需要检查文件是否存在、获取文件详细信息（大小、修改时间、权限）或判断文件类型时使用。不用于读取文件内容',
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description: '要操作的文件或目录路径',
          },
          operation: {
            type: 'string',
            enum: ['exists', 'info', 'type', 'permissions'],
            description:
              '操作类型：exists(仅检查文件是否存在)、info(获取完整文件信息-默认)、type(判断文件类型)、permissions(检查文件权限)',
            default: 'info',
          },
        },
        required: ['path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path, operation = 'info' } = context.parameters as unknown as FileSystemParams

    try {
      switch (operation) {
        case 'exists':
          return await this.checkExists(path)
        case 'info':
          return await this.getFileInfo(path)
        case 'type':
          return await this.getFileType(path)
        case 'permissions':
          return await this.getPermissions(path)
        default:
          throw new Error(`不支持的操作类型: ${operation}`)
      }
    } catch (error) {
      throw new Error(`文件系统操作失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async checkExists(path: string): Promise<ToolResult> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      return {
        content: [
          {
            type: 'text',
            text: `路径 ${path} ${exists ? '存在' : '不存在'}`,
          },
        ],
      }
    } catch {
      return {
        content: [
          {
            type: 'text',
            text: `路径 ${path} 不存在`,
          },
        ],
      }
    }
  }

  private async getFileInfo(path: string): Promise<ToolResult> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      if (!exists) {
        throw new FileNotFoundError(path)
      }

      const metadata = await invoke<{
        isDir: boolean
        isFile: boolean
        size: number
        created: number
        modified: number
        accessed: number
        readonly: boolean
      }>('plugin:fs|metadata', { path })

      const fileInfo: FileInfo = {
        path,
        exists: true,
        isFile: metadata.isFile,
        isDirectory: metadata.isDir,
        size: metadata.size,
        sizeFormatted: this.formatFileSize(metadata.size),
        created: formatFileTime(metadata.created),
        modified: formatFileTime(metadata.modified),
        accessed: formatFileTime(metadata.accessed),
        permissions: {
          readable: true, // 假设可读，因为我们能获取到元数据
          writable: !metadata.readonly,
          executable: false, // 需要额外检查
        },
      }

      const output = this.formatFileInfo(fileInfo)

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
      throw new Error(`获取文件信息失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async getFileType(path: string): Promise<ToolResult> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      if (!exists) {
        throw new FileNotFoundError(path)
      }

      const metadata = await invoke<{
        isDir: boolean
        isFile: boolean
      }>('plugin:fs|metadata', { path })

      let type = '未知'
      let icon = '❓'

      if (metadata.isDir) {
        type = '目录'
        icon = '📁'
      } else if (metadata.isFile) {
        type = '文件'
        icon = '📄'

        // 根据扩展名确定文件类型
        const ext = path.split('.').pop()?.toLowerCase()
        if (ext) {
          const typeInfo = this.getFileTypeByExtension(ext)
          type = typeInfo.type
          icon = typeInfo.icon
        }
      }

      return {
        content: [
          {
            type: 'text',
            text: `${icon} ${path} 是一个${type}`,
          },
        ],
      }
    } catch (error) {
      if (error instanceof FileNotFoundError) {
        throw error
      }
      throw new Error(`获取文件类型失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async getPermissions(path: string): Promise<ToolResult> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      if (!exists) {
        throw new FileNotFoundError(path)
      }

      const metadata = await invoke<{
        readonly: boolean
      }>('plugin:fs|metadata', { path })

      const permissions = {
        readable: true, // 假设可读
        writable: !metadata.readonly,
        executable: false, // 需要额外检查
      }

      const output = [
        `📋 权限信息: ${path}`,
        `可读: ${permissions.readable ? '✅' : '❌'}`,
        `可写: ${permissions.writable ? '✅' : '❌'}`,
        `可执行: ${permissions.executable ? '✅' : '❌'}`,
      ].join('\n')

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
      throw new Error(`获取权限信息失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private formatFileInfo(info: FileInfo): string {
    const lines = [
      `📋 文件信息: ${info.path}`,
      `存在: ${info.exists ? '✅' : '❌'}`,
      `类型: ${info.isDirectory ? '📁 目录' : '📄 文件'}`,
      `大小: ${info.sizeFormatted} (${info.size} 字节)`,
      `创建时间: ${info.created}`,
      `修改时间: ${info.modified}`,
      `访问时间: ${info.accessed}`,
      `权限:`,
      `  可读: ${info.permissions.readable ? '✅' : '❌'}`,
      `  可写: ${info.permissions.writable ? '✅' : '❌'}`,
      `  可执行: ${info.permissions.executable ? '✅' : '❌'}`,
    ]

    return lines.join('\n')
  }

  private formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
  }

  private getFileTypeByExtension(ext: string): { type: string; icon: string } {
    const typeMap: Record<string, { type: string; icon: string }> = {
      // 代码文件
      js: { type: 'JavaScript文件', icon: '📜' },
      ts: { type: 'TypeScript文件', icon: '📜' },
      vue: { type: 'Vue组件文件', icon: '💚' },
      py: { type: 'Python文件', icon: '🐍' },
      java: { type: 'Java文件', icon: '☕' },
      cpp: { type: 'C++文件', icon: '⚙️' },
      c: { type: 'C文件', icon: '⚙️' },
      rs: { type: 'Rust文件', icon: '🦀' },
      go: { type: 'Go文件', icon: '🐹' },

      // 配置文件
      json: { type: 'JSON配置文件', icon: '⚙️' },
      yaml: { type: 'YAML配置文件', icon: '⚙️' },
      yml: { type: 'YAML配置文件', icon: '⚙️' },
      toml: { type: 'TOML配置文件', icon: '⚙️' },
      xml: { type: 'XML文件', icon: '📋' },

      // 文档文件
      md: { type: 'Markdown文档', icon: '📝' },
      txt: { type: '文本文件', icon: '📄' },
      pdf: { type: 'PDF文档', icon: '📕' },
      doc: { type: 'Word文档', icon: '📘' },
      docx: { type: 'Word文档', icon: '📘' },

      // 图片文件
      png: { type: 'PNG图片', icon: '🖼️' },
      jpg: { type: 'JPEG图片', icon: '🖼️' },
      jpeg: { type: 'JPEG图片', icon: '🖼️' },
      gif: { type: 'GIF图片', icon: '🖼️' },
      svg: { type: 'SVG矢量图', icon: '🎨' },

      // 其他
      zip: { type: 'ZIP压缩包', icon: '📦' },
      tar: { type: 'TAR归档', icon: '📦' },
      gz: { type: 'GZIP压缩文件', icon: '📦' },
    }

    return typeMap[ext] || { type: '文件', icon: '📄' }
  }
}

// 导出工具实例
export const fileSystemTool = new FileSystemTool()
