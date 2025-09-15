/**
 * 批量文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { filesystemApi } from '@/api'

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
      `Batch read contents of multiple files. Recommended to read at most 5-10 files at once to avoid overly long output. Each file will be automatically truncated to a reasonable length. Supports showing line numbers and setting file size limits. Will skip unreadable files and mark them in results. All file paths must be absolute paths.`,
      {
        type: 'object',
        properties: {
          paths: {
            type: 'array',
            items: { type: 'string' },
            description:
              'List of absolute file paths. All paths must be complete paths, for example: ["/Users/user/project/src/main.ts", "/home/user/project/utils.ts"]',
            minItems: 1,
          },
          showLineNumbers: {
            type: 'boolean',
            description: 'Whether to show line numbers. Examples: true, false',
            default: false,
          },
          maxFileSize: {
            type: 'number',
            description: 'Maximum file size (bytes). Examples: 1048576, 2097152',
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
          const metadata = await filesystemApi.getMetadata(filePath)
          fileSize = metadata.size

          if (metadata.size && metadata.size > maxFileSize) {
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
        const rawContent = await filesystemApi.readTextFile(filePath)
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
          processedContent = `Important note: File content has been truncated.
Status: Showing first ${MAX_LINES_PER_FILE} lines out of ${lines.length} total lines.
Suggestion: Use read_file tool with offset and limit parameters to read complete content.

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
        resultText += `Success ${result.path} (${result.size} bytes, ${result.lines} lines)\n`
        resultText += `${'─'.repeat(50)}\n`
        resultText += `${result.content}\n\n`
      } else {
        resultText += `Failed ${result.path}: ${result.error}\n\n`
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

export const readManyFilesTool = new ReadManyFilesTool()
