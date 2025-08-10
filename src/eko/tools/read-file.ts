/**
 * 文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { FileNotFoundError } from './tool-error'
import { invoke } from '@tauri-apps/api/core'

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
    super('read_file', '📖 读取文件内容：查看任何文件的内容，支持行号显示、指定行范围', {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: '要读取的文件路径',
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
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { filePath, showLineNumbers = true, startLine, endLine } = context.parameters as unknown as ReadFileParams

    // 验证参数
    if (startLine && endLine && startLine > endLine) {
      throw new Error('开始行号不能大于结束行号')
    }

    try {
      // 直接使用Tauri API读取文件
      const content = await invoke<string>('plugin:fs|read_text_file', {
        path: filePath,
      })

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

      return {
        content: [
          {
            type: 'text',
            text: processedLines.join('\n'),
          },
        ],
      }
    } catch (error) {
      throw new FileNotFoundError(filePath)
    }
  }
}

// 导出工具实例
export const readFileTool = new ReadFileTool()
