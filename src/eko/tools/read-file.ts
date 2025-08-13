/**
 * 文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { FileNotFoundError } from './tool-error'
import { invoke } from '@tauri-apps/api/core'
import { formatFileTime } from '@/utils/dateFormatter'

export interface ReadFileParams {
  filePath: string
  showLineNumbers?: boolean
  startLine?: number
  endLine?: number
}

/**
 * 文件读取工具
 */
export class ReadFileTool extends ModifiableTool {
  constructor() {
    super(
      'read_file',
      '📖 读取文件内容：当需要查看文件具体内容时使用。适用于查看代码、配置文件、文档等文本文件。支持行号显示、指定行范围读取',
      {
        type: 'object',
        properties: {
          filePath: {
            type: 'string',
            description: '要读取的文件路径（必须是文件，不是目录）',
          },
          showLineNumbers: {
            type: 'boolean',
            description: '是否显示行号，默认true',
            default: true,
          },
          startLine: {
            type: 'number',
            description: '开始读取的行号（从1开始），可选',
            minimum: 1,
          },
          endLine: {
            type: 'number',
            description: '结束读取的行号，可选',
            minimum: 1,
          },
        },
        required: ['filePath'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { filePath, showLineNumbers = true, startLine, endLine } = context.parameters as unknown as ReadFileParams

    // 验证参数
    if (startLine && endLine && startLine > endLine) {
      throw new Error('开始行号不能大于结束行号')
    }

    try {
      // 首先检查文件是否存在
      const exists = await this.checkFileExists(filePath)
      if (!exists) {
        throw new FileNotFoundError(filePath)
      }

      // 检查是否为目录
      const isDirectory = await this.checkIsDirectory(filePath)
      if (isDirectory) {
        throw new Error(`路径 ${filePath} 是一个目录，请使用 read_directory 工具读取目录内容`)
      }

      // 使用Tauri API读取文件
      const rawContent = await invoke<ArrayBuffer>('plugin:fs|read_text_file', {
        path: filePath,
      })

      // 确保内容不为空
      if (rawContent === null || rawContent === undefined) {
        throw new Error('文件内容为空或无法读取')
      }

      // 将ArrayBuffer转换为字符串
      const content = new TextDecoder('utf-8').decode(rawContent)

      // 处理文件内容
      const lines = content.split('\n')
      let processedLines = lines

      // 应用行范围过滤
      if (startLine || endLine) {
        const start = startLine ? startLine - 1 : 0
        const end = endLine ? endLine : lines.length
        processedLines = lines.slice(start, end)
      }

      // 添加行号
      if (showLineNumbers) {
        const startNum = startLine || 1
        processedLines = processedLines.map(
          (line, index) => `${(startNum + index).toString().padStart(4, ' ')}  ${line}`
        )
      }

      // 添加文件信息头部
      const fileInfo = await this.getFileInfo(filePath)
      const header = `📖 文件: ${filePath} (${fileInfo.size}, 修改时间: ${fileInfo.modified})\n${'='.repeat(60)}\n`

      return {
        content: [
          {
            type: 'text',
            text: header + processedLines.join('\n'),
          },
        ],
      }
    } catch (error) {
      if (error instanceof FileNotFoundError) {
        throw error
      }
      throw new Error(`读取文件失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async checkFileExists(path: string): Promise<boolean> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      return exists
    } catch (error) {
      return false
    }
  }

  private async checkIsDirectory(path: string): Promise<boolean> {
    try {
      const metadata = await invoke<{ isDir: boolean }>('plugin:fs|metadata', { path })
      return metadata.isDir
    } catch {
      return false
    }
  }

  private async getFileInfo(path: string): Promise<{ size: string; modified: string }> {
    try {
      const metadata = await invoke<{ size: number; modified: number }>('plugin:fs|metadata', { path })
      return {
        size: this.formatFileSize(metadata.size),
        modified: formatFileTime(metadata.modified),
      }
    } catch {
      return { size: '未知', modified: '未知' }
    }
  }

  private formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
  }
}

// 导出工具实例
export const readFileTool = new ReadFileTool()
