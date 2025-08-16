/**
 * 批量文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { invoke } from '@tauri-apps/api/core'

export interface ReadManyFilesParams {
  filePaths: string[]
  showLineNumbers?: boolean
  maxFileSize?: number
}

export interface FileReadResult {
  path: string
  success: boolean
  content?: string
  error?: string
  size?: number
  lines?: number
}

/**
 * 批量文件读取工具
 */
export class ReadManyFilesTool extends ModifiableTool {
  constructor() {
    super('read_many_files', '批量读取文件：一次性读取多个文件的内容', {
      type: 'object',
      properties: {
        filePaths: {
          type: 'array',
          items: { type: 'string' },
          description: '要读取的文件路径列表',
          minItems: 1,
        },
        showLineNumbers: {
          type: 'boolean',
          description: '是否显示行号，默认false',
          default: false,
        },
        maxFileSize: {
          type: 'number',
          description: '最大文件大小（字节），默认1MB',
          default: 1048576,
          minimum: 1024,
        },
      },
      required: ['filePaths'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const {
      filePaths,
      showLineNumbers = false,
      maxFileSize = 1048576,
    } = context.parameters as unknown as ReadManyFilesParams

    const results: FileReadResult[] = []

    for (const filePath of filePaths) {
      try {
        // 尝试检查文件大小（如果权限允许）
        let fileSize: number | undefined = undefined
        try {
          const metadata = await invoke<{ size: number }>('plugin:fs|metadata', { path: filePath })
          fileSize = metadata.size

          if (metadata.size > maxFileSize) {
            results.push({
              path: filePath,
              success: false,
              error: `文件过大 (${metadata.size} bytes > ${maxFileSize} bytes)`,
              size: metadata.size,
            })
            continue
          }
        } catch (metadataError) {
          // 如果无法获取metadata，跳过大小检查，继续读取文件
        }

        // 读取文件内容
        const rawContent = await invoke<ArrayBuffer>('plugin:fs|read_text_file', { path: filePath })
        const content = new TextDecoder('utf-8').decode(rawContent)
        const lines = content.split('\n')

        let processedContent = content
        if (showLineNumbers) {
          processedContent = lines
            .map((line, index) => `${(index + 1).toString().padStart(4, ' ')}  ${line}`)
            .join('\n')
        }

        results.push({
          path: filePath,
          success: true,
          content: processedContent,
          size: fileSize,
          lines: lines.length,
        })
      } catch (error) {
        results.push({
          path: filePath,
          success: false,
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }

    // 格式化输出
    let resultText = `批量文件读取结果 (${results.length} 个文件):\n\n`

    for (const result of results) {
      if (result.success) {
        resultText += `成功 ${result.path} (${result.size} bytes, ${result.lines} lines)\n`
        resultText += `${'─'.repeat(50)}\n`
        resultText += `${result.content}\n\n`
      } else {
        resultText += `失败 ${result.path}: ${result.error}\n\n`
      }
    }

    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    }
  }
}

// 导出工具实例
export const readManyFilesTool = new ReadManyFilesTool()
