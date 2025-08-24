/**
 * 文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@eko-ai/eko/types'
import { FileNotFoundError, ValidationError, ToolError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface ReadFileParams {
  path: string
  offset?: number // 0-based行号，从哪一行开始读取
  limit?: number // 读取多少行
}

/**
 * 文件读取工具
 */
export class ReadFileTool extends ModifiableTool {
  constructor() {
    super(
      'read_file',
      `读取文件内容并显示。如果文件较大，内容会被截断。工具响应会明确指示是否发生了截断，并提供如何使用'offset'和'limit'参数读取更多文件内容的详细信息。支持文本文件的特定行范围读取。必须使用绝对路径。`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description:
              '文件的绝对路径。必须是完整路径，例如："/Users/user/project/src/main.ts"、"/home/user/config.json"',
          },
          offset: {
            type: 'number',
            description:
              '可选：0-based行号，从哪一行开始读取。用于分页浏览大文件。需要与limit一起使用。示例：0、50、100',
            minimum: 0,
          },
          limit: {
            type: 'number',
            description:
              '可选：最大读取行数。与offset一起使用可分页浏览大文件。如果省略，读取整个文件（最多2000行）。示例：50、100',
            minimum: 1,
          },
        },
        required: ['path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path, offset, limit } = context.parameters as unknown as ReadFileParams

    // 验证参数
    if (offset !== undefined && offset < 0) {
      throw new Error('offset必须大于等于0')
    }
    if (limit !== undefined && limit <= 0) {
      throw new Error('limit必须大于0')
    }

    try {
      // 首先检查文件是否存在
      const exists = await this.checkFileExists(path)
      if (!exists) {
        throw new FileNotFoundError(path)
      }

      // 检查是否为目录
      const isDirectory = await this.checkIsDirectory(path)
      if (isDirectory) {
        throw new ValidationError(`路径 ${path} 是一个目录，请使用 read_directory 工具读取目录内容`)
      }

      // 使用Tauri API读取文件
      const rawContent = await invoke<ArrayBuffer>('plugin:fs|read_text_file', {
        path: path,
      })

      // 确保内容不为空
      if (rawContent === null || rawContent === undefined) {
        throw new ToolError('文件内容为空或无法读取')
      }

      // 将ArrayBuffer转换为字符串
      const content = new TextDecoder('utf-8').decode(rawContent)

      // 处理文件内容
      const lines = content.split('\n')
      const originalLineCount = lines.length

      // 设置默认值
      const DEFAULT_MAX_LINES = 2000
      const MAX_LINE_LENGTH = 2000

      const startLine = offset || 0
      const effectiveLimit = limit === undefined ? DEFAULT_MAX_LINES : limit
      const endLine = Math.min(startLine + effectiveLimit, originalLineCount)
      const actualStartLine = Math.min(startLine, originalLineCount)

      // 选择行范围
      const selectedLines = lines.slice(actualStartLine, endLine)

      // 处理过长的行
      const processedLines = selectedLines.map((line, index) => {
        if (line.length > MAX_LINE_LENGTH) {
          line = line.substring(0, MAX_LINE_LENGTH) + '... [truncated]'
        }
        // 添加行号 (1-based显示)
        const lineNum = actualStartLine + index + 1
        return `${lineNum.toString().padStart(4, ' ')}  ${line}`
      })

      // 返回结果，不添加额外信息
      const resultText = processedLines.join('\n')

      return {
        content: [
          {
            type: 'text',
            text: resultText,
          },
        ],
      }
    } catch (error) {
      if (error instanceof FileNotFoundError || error instanceof ValidationError || error instanceof ToolError) {
        throw error
      }
      throw new ToolError(`读取文件失败: ${error instanceof Error ? error.message : String(error)}`)
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
