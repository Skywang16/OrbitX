/**
 * Grep 搜索工具 - 简单直接的 grep 命令执行工具
 *
 * 直接接收完整的 grep 指令并执行，由 LLM 生成具体的 grep 命令参数
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { ValidationError, ToolError } from '../tool-error'
import { shellApi } from '@/api/shell'

// ===== 类型定义 =====

export interface GrepSearchParams {
  command: string
}

export interface GrepSearchResponse {
  command: string
  output: string
  executionTime: number
  success: boolean
}

/**
 * Grep 搜索工具
 */
export class GrepSearchTool extends ModifiableTool {
  constructor() {
    super(
      'grep_search',
      `Execute complete grep commands for text search. Provide the full grep command and it will be executed directly. Examples: "grep -rn 'TaskList' /Users/user/project/src", "grep -i 'import.*Component' /path/to/files"`,
      {
        type: 'object',
        properties: {
          command: {
            type: 'string',
            description:
              'Complete grep command to execute. Must start with "grep" and include all necessary options and paths. Examples: "grep -rn pattern /absolute/path", "grep -i -A 3 pattern file.txt"',
          },
        },
        required: ['command'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as GrepSearchParams
    const { command } = params

    // 验证参数
    this.validateGrepCommand(command)

    try {
      const startTime = Date.now()

      // 直接执行 grep 命令，传递当前工作目录
      const result = await shellApi.executeBackgroundCommand(command, process.cwd())

      const executionTime = Date.now() - startTime

      // 格式化输出
      const resultText = this.formatGrepResults(
        {
          command,
          output: result.stdout,
          executionTime,
          success: result.success || result.exitCode === 1, // grep 返回 1 表示没有匹配，这是正常的
        },
        result.stderr
      )

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
      throw new ToolError(`Grep command execution failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 验证 grep 命令
   */
  private validateGrepCommand(command: string): void {
    if (!command || command.trim() === '') {
      throw new ValidationError('Grep 命令不能为空')
    }

    const trimmedCommand = command.trim()

    // 检查命令是否以 grep 开头
    if (!trimmedCommand.startsWith('grep ')) {
      throw new ValidationError('命令必须以 "grep " 开头')
    }

    // 检查危险的命令注入
    const dangerousPatterns = ['$(', '`', ';', '&&', '||', '|', '>', '>>', '<', 'rm ', 'del ', 'mv ', 'cp ']

    for (const dangerous of dangerousPatterns) {
      if (trimmedCommand.includes(dangerous)) {
        throw new ValidationError(`检测到潜在危险字符或命令，请使用安全的 grep 命令: ${dangerous}`)
      }
    }
  }

  /**
   * 格式化 grep 结果
   */
  private formatGrepResults(response: GrepSearchResponse, stderr?: string): string {
    let result = `执行命令: ${response.command}\n`
    result += `执行时间: ${response.executionTime}ms\n`
    result += `执行状态: ${response.success ? '成功' : '失败'}\n\n`

    if (!response.success && stderr) {
      result += `错误信息:\n${stderr}\n\n`
    }

    if (response.output && response.output.trim()) {
      result += `搜索结果:\n${response.output}`
    } else {
      result += `未找到匹配项`
    }

    return result
  }
}

// 导出工具实例
export const grepSearchTool = new GrepSearchTool()
