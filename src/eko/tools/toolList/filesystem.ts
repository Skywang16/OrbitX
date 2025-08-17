/**
 * 文件系统操作工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { FileNotFoundError } from '../tool-error'
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
      `文件系统信息工具。
输入示例: {"path": "./src/main.ts"}
输出示例: {
  "content": [{
    "type": "text",
    "text": "文件信息: ./src/main.ts\\n\\n类型: 文件\\n大小: 1.2 KB (1234 bytes)\\n创建时间: 2024-12-15 10:30:45\\n修改时间: 2024-12-15 14:22:10\\n权限: 可读 可写"
  }]
}`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description: '文件或目录路径。示例："./src/main.ts"、"./package.json"、"./src"',
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

      if (metadata.isDir) {
        type = '目录'
      } else if (metadata.isFile) {
        type = '文件'

        // 根据扩展名确定文件类型
        const ext = path.split('.').pop()?.toLowerCase()
        if (ext) {
          const typeInfo = this.getFileTypeByExtension(ext)
          type = typeInfo.type
        }
      }

      return {
        content: [
          {
            type: 'text',
            text: `${path} 是一个${type}`,
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

  private getFileTypeByExtension(ext: string): { type: string } {
    const typeMap: Record<string, { type: string }> = {
      // 代码文件
      js: { type: 'JavaScript文件' },
      ts: { type: 'TypeScript文件' },
      vue: { type: 'Vue组件文件' },
      py: { type: 'Python文件' },
      java: { type: 'Java文件' },
      cpp: { type: 'C++文件' },
      c: { type: 'C文件' },
      rs: { type: 'Rust文件' },
      go: { type: 'Go文件' },

      // 配置文件
      json: { type: 'JSON配置文件' },
      yaml: { type: 'YAML配置文件' },
      yml: { type: 'YAML配置文件' },
      toml: { type: 'TOML配置文件' },
      xml: { type: 'XML文件' },

      // 文档文件
      md: { type: 'Markdown文档' },
      txt: { type: '文本文件' },
      pdf: { type: 'PDF文档' },
      doc: { type: 'Word文档' },
      docx: { type: 'Word文档' },

      // 图片文件
      png: { type: 'PNG图片' },
      jpg: { type: 'JPEG图片' },
      jpeg: { type: 'JPEG图片' },
      gif: { type: 'GIF图片' },
      svg: { type: 'SVG矢量图' },

      // 其他
      zip: { type: 'ZIP压缩包' },
      tar: { type: 'TAR归档' },
      gz: { type: 'GZIP压缩文件' },
    }

    return typeMap[ext] || { type: '文件' }
  }
}

// 导出工具实例
export const fileSystemTool = new FileSystemTool()
