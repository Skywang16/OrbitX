/**
 * Agent框架配置管理器
 *
 * 只管理前端Agent执行相关的配置，不涉及后端LLM配置
 */

import type { ParallelExecutionConfig } from '../types'

/**
 * Agent配置管理器
 */
export class AgentConfig {
  // ===== 执行配置 =====
  static readonly DEFAULT_TIMEOUT = 30000 // 30秒
  static readonly DEFAULT_MAX_CONCURRENCY = 5
  static readonly DEFAULT_MAX_RETRIES = 3
  static readonly DEFAULT_RETRY_DELAY = 1000 // 1秒

  // ===== 工具执行配置 =====
  static readonly TOOL_EXECUTION_TIMEOUT = 30000 // 30秒
  static readonly TOOL_OUTPUT_WAIT_TIME = 3000 // 3秒，等待终端输出
  static readonly MAX_COMMAND_LENGTH = 1000 // 最大命令长度

  // ===== 规划器配置 =====
  static readonly PLANNING_TIMEOUT = 60000 // 1分钟
  static readonly MAX_PLANNING_ATTEMPTS = 3

  // ===== 安全配置 =====
  static readonly DANGEROUS_COMMANDS = [
    'rm -rf /',
    'rm -rf *',
    'mkfs',
    'dd if=',
    'format',
    'fdisk',
    'shutdown',
    'reboot',
    'halt',
    'init 0',
    'init 6',
    ':(){ :|:& };:', // fork bomb
    'chmod -R 777 /',
    'chown -R root /',
  ]

  // ===== 动态配置存储 =====
  private static config: Record<string, unknown> = {}

  /**
   * 获取配置值
   */
  static get<T>(key: string, defaultValue: T): T {
    return (this.config[key] as T) ?? defaultValue
  }

  /**
   * 设置配置值
   */
  static set(key: string, value: unknown): void {
    this.config[key] = value
  }

  /**
   * 获取并行执行配置
   */
  static getParallelExecutionConfig(): ParallelExecutionConfig {
    return {
      maxConcurrency: this.get('parallel.maxConcurrency', this.DEFAULT_MAX_CONCURRENCY),
      stepTimeout: this.get('parallel.stepTimeout', this.DEFAULT_TIMEOUT),
      retryDelay: this.get('parallel.retryDelay', this.DEFAULT_RETRY_DELAY),
      enableDetailedLogging: this.get('parallel.enableDetailedLogging', false),
      maxRetries: this.get('parallel.maxRetries', this.DEFAULT_MAX_RETRIES),
    }
  }

  /**
   * 获取工具执行超时时间
   */
  static getToolTimeout(toolId?: string): number {
    if (toolId) {
      const customTimeout = this.get(`tool.timeout.${toolId}`, null)
      if (customTimeout) return customTimeout as number
    }
    return this.TOOL_EXECUTION_TIMEOUT
  }

  /**
   * 检查命令是否安全
   */
  static isCommandSafe(command: string): boolean {
    const lowerCommand = command.toLowerCase().trim()

    // 检查长度限制
    if (command.length > this.MAX_COMMAND_LENGTH) {
      return false
    }

    // 检查危险命令
    for (const dangerous of this.DANGEROUS_COMMANDS) {
      if (lowerCommand.includes(dangerous.toLowerCase())) {
        return false
      }
    }

    // 检查危险模式
    const dangerousPatterns = [
      /rm\s+-rf\s+\/[^/\s]*/, // rm -rf /xxx
      /rm\s+-rf\s+\*/, // rm -rf *
      />\s*\/dev\/sd[a-z]/, // 写入磁盘设备
      /dd\s+.*of=\/dev/, // dd写入设备
    ]

    for (const pattern of dangerousPatterns) {
      if (pattern.test(lowerCommand)) {
        return false
      }
    }

    return true
  }

  /**
   * 重置配置
   */
  static reset(): void {
    this.config = {}
  }

  /**
   * 加载配置
   */
  static loadConfig(config: Record<string, unknown>): void {
    this.config = { ...this.config, ...config }
  }
}
