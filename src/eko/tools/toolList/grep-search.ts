/**
 * Grep 搜索工具 - 简单直接的文本搜索工具
 *
 * 相比 orbit-search 的复杂语义搜索，这个工具提供简单直接的 grep 命令执行
 * 让 LLM 能够直接使用 grep 的强大功能进行文本搜索
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { ValidationError, ToolError } from '../tool-error'
import { terminalApi } from '@/api'
import { useTerminalStore } from '@/stores/Terminal'
import stripAnsi from 'strip-ansi'

// ===== 类型定义 =====

export interface GrepSearchParams {
  pattern: string
  path: string
  options?: string
}

export interface GrepMatch {
  filePath: string
  lineNumber?: number
  line: string
  context?: {
    before?: string[]
    after?: string[]
  }
}

export interface GrepSearchResponse {
  pattern: string
  searchPath: string
  command: string
  totalMatches: number
  matches: GrepMatch[]
  executionTime: number
  truncated: boolean
}

/**
 * Grep 搜索工具
 *
 * 核心特性：
 * 1. 直接执行 grep 命令，充分利用 grep 的强大功能
 * 2. 支持常用的 grep 选项：递归搜索、忽略大小写、显示行号等
 * 3. 安全验证：防止危险的命令注入
 * 4. 结果格式化：将 grep 输出转换为结构化数据
 * 5. 性能优化：限制结果数量，避免输出过长
 */
export class GrepSearchTool extends ModifiableTool {
  private readonly maxLineLength = 1000
  private readonly defaultTimeout = 30000 // 30秒

  constructor() {
    super(
      'grep_search',
      `Execute grep commands for text search in files and directories. This tool provides direct access to grep functionality, allowing LLM to use grep's powerful pattern matching capabilities. Supports regular expressions, recursive search, case-insensitive search, and context lines. Much simpler and more direct than orbit_search for basic text searching tasks.`,
      {
        type: 'object',
        properties: {
          pattern: {
            type: 'string',
            description:
              'Search pattern (can be literal text or regular expression). Examples: "function", "class.*Component", "TODO|FIXME"',
          },
          path: {
            type: 'string',
            description:
              'Absolute path to search in (file or directory). Must be a complete path, for example: "/Users/user/project/src", "/home/user/workspace/file.ts"',
          },
          options: {
            type: 'string',
            description:
              'Grep command options. Examples: "-r" for recursive, "-i" for ignore case, "-n" for line numbers, "-C 2" for context lines, "-E" for extended regex, "-w" for word boundaries. Default: "-rn" (recursive with line numbers)',
          },
        },
        required: ['pattern', 'path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as GrepSearchParams
    const { pattern, path, options = '-rn' } = params

    // 验证参数
    this.validateGrepParameters(params)

    try {
      const startTime = Date.now()

      // 构建 grep 命令
      const grepCommand = this.buildGrepCommand(pattern, path, options)

      // 执行命令
      const rawOutput = await this.executeGrepCommand(grepCommand)

      // 解析结果
      const searchResult = this.parseGrepOutput(rawOutput, pattern, path, grepCommand)
      searchResult.executionTime = Date.now() - startTime

      // 格式化输出
      const resultText = this.formatSearchResults(searchResult)

      return {
        content: [
          {
            type: 'text',
            text: resultText,
          },
        ],
      }
    } catch (error) {
      if (error instanceof ValidationError || error instanceof ToolError) {
        throw error
      }
      throw new ToolError(`Grep search failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 验证参数
   */
  private validateGrepParameters(params: GrepSearchParams): void {
    const { pattern, path, options } = params

    if (!pattern || pattern.trim() === '') {
      throw new ValidationError('搜索模式不能为空')
    }

    if (!path || path.trim() === '') {
      throw new ValidationError('搜索路径不能为空')
    }

    // 检查危险的模式
    const dangerousPatterns = ['$(', '`', '|', ';', '&&', '||', '>', '>>', '<']

    for (const dangerous of dangerousPatterns) {
      if (pattern.includes(dangerous)) {
        throw new ValidationError(`检测到潜在危险字符，请使用安全的搜索模式: ${dangerous}`)
      }
    }

    // 验证选项安全性
    if (options) {
      this.validateGrepOptions(options)
    }
  }

  /**
   * 构建 grep 命令
   */
  private buildGrepCommand(pattern: string, path: string, options: string): string {
    let command = 'grep'

    // 添加选项
    if (options && options.trim()) {
      command += ` ${options.trim()}`
    }

    // 使用单引号包围模式以防止shell解释
    const escapedPattern = pattern.replace(/'/g, "'\"'\"'")
    const escapedPath = path.replace(/'/g, "'\"'\"'")

    command = `${command} '${escapedPattern}' '${escapedPath}'`

    return command
  }

  /**
   * 验证 grep 选项的安全性
   */
  private validateGrepOptions(options: string): void {
    const dangerousOptions = ['--', '|', ';', '&&', '||', '>', '>>', '<', '$', '`']

    for (const dangerous of dangerousOptions) {
      if (options.includes(dangerous)) {
        throw new ValidationError(`检测到危险的 grep 选项: ${dangerous}`)
      }
    }
  }

  /**
   * 执行 grep 命令
   */
  private async executeGrepCommand(command: string): Promise<string> {
    const terminalStore = useTerminalStore()

    // 获取当前活跃的终端
    const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)

    if (!activeTerminal || !activeTerminal.backendId) {
      throw new ToolError('没有可用的活跃终端')
    }

    const terminalId = activeTerminal.backendId

    return new Promise<string>((resolve, reject) => {
      let outputBuffer = ''
      let timeoutId: NodeJS.Timeout
      let isCompleted = false

      // 设置超时
      timeoutId = setTimeout(() => {
        if (!isCompleted) {
          isCompleted = true
          cleanup()
          reject(new ToolError(`Grep 命令执行超时 (${this.defaultTimeout}ms): ${command}`))
        }
      }, this.defaultTimeout)

      // 清理函数
      const cleanup = () => {
        if (timeoutId) {
          clearTimeout(timeoutId)
        }
        terminalStore.unregisterTerminalCallbacks(activeTerminal.id, callbacks)
      }

      // 命令完成检测
      const detectCommandCompletion = (output: string): boolean => {
        if (!output || output.trim() === '') return false

        const cleanOutput = stripAnsi(output).replace(/\r/g, '')
        const lines = cleanOutput.split('\n').filter(line => line.trim() !== '')
        if (lines.length === 0) return false

        const lastLine = lines[lines.length - 1].trim()

        // 检测提示符模式
        const promptPatterns = [
          /.*[@#$%>]\s*$/,
          /.*\$\s*$/,
          /.*%\s*$/,
          /.*#\s*$/,
          /.*>\s*$/,
          /.*@.*:\s*[~/].*[$%#>]\s*$/,
        ]

        return promptPatterns.some(pattern => pattern.test(lastLine))
      }

      // 终端输出监听回调
      const callbacks = {
        onOutput: (data: string) => {
          outputBuffer += data

          // 检测命令是否完成
          const isComplete = detectCommandCompletion(data) || detectCommandCompletion(outputBuffer)

          if (isComplete && !isCompleted) {
            isCompleted = true
            cleanup()

            // 清理输出并返回
            const cleanOutput = this.cleanGrepOutput(outputBuffer, command)
            resolve(cleanOutput)
          }
        },
        onExit: (exitCode: number | null) => {
          if (!isCompleted) {
            isCompleted = true
            cleanup()

            if (exitCode === 0 || exitCode === 1) {
              // grep 返回 1 表示没有匹配，这是正常的
              const cleanOutput = this.cleanGrepOutput(outputBuffer, command)
              resolve(cleanOutput)
            } else {
              reject(new ToolError(`Grep 命令执行失败，退出码: ${exitCode}`))
            }
          }
        },
      }

      // 注册监听器
      terminalStore.registerTerminalCallbacks(activeTerminal.id, callbacks)

      // 执行命令
      setTimeout(() => {
        terminalApi
          .writeToTerminal({
            paneId: terminalId,
            data: `${command}\n`,
          })
          .catch(error => {
            if (!isCompleted) {
              isCompleted = true
              cleanup()
              reject(new ToolError(`Failed to execute grep command: ${error.message}`))
            }
          })
      }, 100)
    })
  }

  /**
   * 清理 grep 输出
   */
  private cleanGrepOutput(output: string, _command: string): string {
    if (!output) return ''

    // 清理 ANSI 序列
    const cleanedOutput = stripAnsi(output)
    const lines = cleanedOutput.split('\n')
    const cleanLines: string[] = []
    let foundCommand = false

    for (const line of lines) {
      const trimmed = line.trim()

      // 跳过空行
      if (!trimmed) continue

      // 跳过提示符
      if (trimmed.match(/^[$#%>]\s*$/) || trimmed.match(/.*[@#$%>:]\s*$/)) {
        continue
      }

      // 跳过包含命令的行
      if (trimmed.includes('grep') && !foundCommand) {
        foundCommand = true
        continue
      }

      cleanLines.push(line) // 保留原始格式，包括缩进
    }

    return cleanLines.join('\n')
  }

  /**
   * 解析 grep 输出
   */
  private parseGrepOutput(output: string, pattern: string, searchPath: string, command: string): GrepSearchResponse {
    const matches: GrepMatch[] = []
    const maxResults = 100 // 固定最大结果数
    let truncated = false

    if (!output || output.trim() === '') {
      return {
        pattern,
        searchPath,
        command,
        totalMatches: 0,
        matches: [],
        executionTime: 0,
        truncated: false,
      }
    }

    const lines = output.split('\n').filter(line => line.trim() !== '')
    let lineCount = 0

    for (const line of lines) {
      if (lineCount >= maxResults) {
        truncated = true
        break
      }

      // 解析 grep 输出格式
      const match = this.parseGrepLine(line, searchPath)
      if (match) {
        matches.push(match)
        lineCount++
      }
    }

    return {
      pattern,
      searchPath,
      command,
      totalMatches: matches.length,
      matches,
      executionTime: 0, // 将在调用处设置
      truncated,
    }
  }

  /**
   * 解析单行 grep 输出
   */
  private parseGrepLine(line: string, basePath: string): GrepMatch | null {
    if (!line || line.trim() === '') return null

    // grep 输出格式：
    // 1. 带行号：filename:lineNumber:content
    // 2. 不带行号：filename:content
    // 3. 递归搜索：path/filename:lineNumber:content

    const colonIndex = line.indexOf(':')
    if (colonIndex === -1) return null

    let filePath = ''
    let lineNumber: number | undefined
    let content = ''

    // 尝试解析带行号的格式
    const lineNumberMatch = line.match(/^(.+?):(\d+):(.*)$/)
    if (lineNumberMatch) {
      filePath = lineNumberMatch[1]
      lineNumber = parseInt(lineNumberMatch[2], 10)
      content = lineNumberMatch[3]
    } else {
      // 不带行号的格式
      const simpleMatch = line.match(/^(.+?):(.*)$/)
      if (simpleMatch) {
        filePath = simpleMatch[1]
        content = simpleMatch[2]
      } else {
        return null
      }
    }

    // 处理相对路径
    if (!filePath.startsWith('/')) {
      filePath = `${basePath}/${filePath}`.replace(/\/+/g, '/')
    }

    // 限制行长度
    if (content.length > this.maxLineLength) {
      content = content.substring(0, this.maxLineLength) + '... [truncated]'
    }

    return {
      filePath,
      lineNumber,
      line: content,
    }
  }

  /**
   * 格式化搜索结果
   */
  private formatSearchResults(response: GrepSearchResponse): string {
    if (response.totalMatches === 0) {
      return `未找到匹配项: "${response.pattern}"\n搜索路径: ${response.searchPath}\n执行命令: ${response.command}`
    }

    let result = `搜索模式: "${response.pattern}"\n`
    result += `搜索路径: ${response.searchPath}\n`
    result += `执行命令: ${response.command}\n`
    result += `找到 ${response.totalMatches} 个匹配项`

    if (response.truncated) {
      result += ` (结果已截断)`
    }

    result += `\n执行时间: ${response.executionTime}ms\n\n`

    // 按文件分组显示结果
    const fileGroups = new Map<string, GrepMatch[]>()

    for (const match of response.matches) {
      if (!fileGroups.has(match.filePath)) {
        fileGroups.set(match.filePath, [])
      }
      fileGroups.get(match.filePath)!.push(match)
    }

    // 显示每个文件的匹配结果
    for (const [filePath, matches] of fileGroups) {
      result += `📁 ${filePath} (${matches.length} 个匹配)\n`

      for (const match of matches.slice(0, 10)) {
        // 每个文件最多显示10个匹配
        if (match.lineNumber !== undefined) {
          result += `  ${match.lineNumber}: ${match.line}\n`
        } else {
          result += `  ${match.line}\n`
        }
      }

      if (matches.length > 10) {
        result += `  ... 还有 ${matches.length - 10} 个匹配\n`
      }

      result += '\n'
    }

    return result.trim()
  }
}

// 导出工具实例
export const grepSearchTool = new GrepSearchTool()
