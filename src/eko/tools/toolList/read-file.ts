/**
 * 文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { FileNotFoundError, ValidationError, ToolError } from '../tool-error'
import { filesystemApi } from '@/api'

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
      `Read and display file contents. If the file is large, content will be truncated. Tool responses will clearly indicate if truncation occurred and provide detailed information on how to use 'offset' and 'limit' parameters to read more file content. Supports reading specific line ranges from text files. Must use absolute paths.`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description:
              'Absolute path to the file. Must be a complete path, for example: "/Users/user/project/src/main.ts", "/home/user/config.json"',
          },
          offset: {
            type: 'number',
            description:
              'Optional: 0-based line number, from which line to start reading. Used for paginated browsing of large files. Should be used together with limit. Examples: 0, 50, 100',
            minimum: 0,
          },
          limit: {
            type: 'number',
            description:
              'Optional: Maximum number of lines to read. Used together with offset for paginated browsing of large files. If omitted, reads the entire file (up to 2000 lines). Examples: 50, 100',
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
      throw new Error('offset must be greater than or equal to 0')
    }
    if (limit !== undefined && limit <= 0) {
      throw new Error('limit must be greater than 0')
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

      // 检查是否为二进制文件
      if (this.isBinaryFile(path)) {
        throw new ValidationError(`文件 ${path} 是二进制文件，无法以文本方式读取`)
      }

      // 使用Tauri API读取文件
      const rawContent = await filesystemApi.readTextFile(path)

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
      throw new ToolError(`Failed to read file: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async checkFileExists(path: string): Promise<boolean> {
    try {
      const exists = await filesystemApi.exists(path)
      return exists
    } catch (error) {
      return false
    }
  }

  private async checkIsDirectory(path: string): Promise<boolean> {
    try {
      const metadata = await filesystemApi.getMetadata(path)
      return metadata.isDir || false
    } catch {
      return false
    }
  }

  private isBinaryFile(path: string): boolean {
    // 获取文件扩展名
    const ext = path.toLowerCase().split('.').pop() || ''

    // 常见的二进制文件扩展名
    const binaryExtensions = new Set([
      // 图片文件
      'jpg',
      'jpeg',
      'png',
      'gif',
      'bmp',
      'tiff',
      'tif',
      'webp',
      'ico',
      'svg',
      // 音频文件
      'mp3',
      'wav',
      'flac',
      'aac',
      'ogg',
      'm4a',
      'wma',
      // 视频文件
      'mp4',
      'avi',
      'mkv',
      'mov',
      'wmv',
      'flv',
      'webm',
      '3gp',
      // 压缩文件
      'zip',
      'rar',
      '7z',
      'tar',
      'gz',
      'bz2',
      'xz',
      // 可执行文件
      'exe',
      'dll',
      'so',
      'dylib',
      'app',
      'deb',
      'rpm',
      'dmg',
      // 办公文档
      'doc',
      'docx',
      'xls',
      'xlsx',
      'ppt',
      'pptx',
      'pdf',
      // 字体文件
      'ttf',
      'otf',
      'woff',
      'woff2',
      'eot',
      // 数据库文件
      'db',
      'sqlite',
      'sqlite3',
      // 编译产物
      'class',
      'jar',
      'war',
      'ear',
      'pyc',
      'pyo',
      'o',
      'obj',
      // 其他二进制
      'bin',
      'dat',
      'iso',
      'img',
    ])

    return binaryExtensions.has(ext)
  }
}

export const readFileTool = new ReadFileTool()
