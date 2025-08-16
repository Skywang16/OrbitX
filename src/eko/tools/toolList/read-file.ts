/**
 * 文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { FileNotFoundError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface ReadFileParams {
  filePath: string
  startLine?: number
  endLine?: number
}

/**
 * 文件读取工具
 */
export class ReadFileTool extends ModifiableTool {
  constructor() {
    super('read_file', '读取文件内容：查看指定文件的文本内容', {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: '要读取的文件路径',
        },
        startLine: {
          type: 'number',
          description: '开始行号（可选，从1开始）',
          minimum: 1,
        },
        endLine: {
          type: 'number',
          description: '结束行号（可选）',
          minimum: 1,
        },
      },
      required: ['filePath'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { filePath, startLine, endLine } = context.parameters as unknown as ReadFileParams

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
      const startNum = startLine || 1
      processedLines = processedLines.map((line, index) => `${(startNum + index).toString().padStart(4, ' ')}  ${line}`)

      return {
        content: [
          {
            type: 'text',
            text: processedLines.join('\n'),
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
}

// 导出工具实例
export const readFileTool = new ReadFileTool()
