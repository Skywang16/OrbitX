/**
 * 批量文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@eko-ai/eko/types'
import { invoke } from '@tauri-apps/api/core'

export interface ReadManyFilesParams {
  paths: string[]
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
    super(
      'read_many_files',
      `批量读取多个文件的内容。建议一次最多读取5-10个文件，避免输出过长。每个文件会自动截断到合理长度。支持显示行号，可设置文件大小限制。会跳过无法读取的文件并在结果中标记。所有文件路径必须是绝对路径。`,
      {
        type: 'object',
        properties: {
          paths: {
            type: 'array',
            items: { type: 'string' },
            description:
              '文件绝对路径列表。所有路径必须是完整路径，例如：["/Users/user/project/src/main.ts", "/home/user/project/utils.ts"]',
            minItems: 1,
          },
          showLineNumbers: {
            type: 'boolean',
            description: '是否显示行号。示例：true、false',
            default: false,
          },
          maxFileSize: {
            type: 'number',
            description: '最大文件大小（字节）。示例：1048576、2097152',
            default: 1048576,
            minimum: 1024,
          },
        },
        required: ['paths'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const {
      paths,
      showLineNumbers = false,
      maxFileSize = 1048576,
    } = context.parameters as unknown as ReadManyFilesParams

    const results: FileReadResult[] = []
    const MAX_LINES_PER_FILE = 2000
    const MAX_LINE_LENGTH = 2000

    for (const filePath of paths) {
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

        // 应用截断逻辑
        let processedLines = lines

        // 限制行数
        let wasTruncated = false
        if (lines.length > MAX_LINES_PER_FILE) {
          processedLines = lines.slice(0, MAX_LINES_PER_FILE)
          wasTruncated = true
        }

        // 限制行长度
        processedLines = processedLines.map(line => {
          if (line.length > MAX_LINE_LENGTH) {
            return line.substring(0, MAX_LINE_LENGTH) + '... [truncated]'
          }
          return line
        })

        // 添加行号
        if (showLineNumbers) {
          processedLines = processedLines.map((line, index) => `${(index + 1).toString().padStart(4, ' ')}  ${line}`)
        }

        let processedContent = processedLines.join('\n')

        // 如果被截断，添加提示
        if (wasTruncated) {
          processedContent = `重要提示：文件内容已被截断。
状态：显示了前 ${MAX_LINES_PER_FILE} 行，总共 ${lines.length} 行。
建议：使用 read_file 工具的 offset 和 limit 参数读取完整内容。

${processedContent}`
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
