/**
 * Agent基础接口定义
 * 完全重构版本，基于AgentContext的纯净架构
 */

import { AgentContext, AgentExecutionStatus } from '../context/AgentContext'
import { ChatMessageRole } from '../types/memory'

/**
 * Agent执行结果
 */
export interface AgentResult {
  success: boolean
  data?: unknown
  error?: string
  executionTime: number
  metadata?: Record<string, unknown>
}

/**
 * Agent基础接口 - 完全基于上下文
 */
export interface IAgent {
  /**
   * 使用AgentContext执行任务
   */
  executeWithContext(agentContext: AgentContext): Promise<AgentResult>
}

/**
 * Agent基础类
 * 完全重构版本，纯净的基于AgentContext的架构
 */
export abstract class BaseAgent implements IAgent {
  protected maxRetries: number = 3
  protected retryDelay: number = 1000 // 1秒

  /**
   * 子类必须实现的核心执行方法
   */
  abstract executeWithContext(agentContext: AgentContext): Promise<AgentResult>

  /**
   * 带重试的执行包装器
   */
  public async executeWithRetry(agentContext: AgentContext): Promise<AgentResult> {
    const startTime = Date.now()

    try {
      // 更新Agent状态
      agentContext.updateStatus(AgentExecutionStatus.EXECUTING)

      // 记录开始执行
      await agentContext.addMessage(ChatMessageRole.SYSTEM, `开始执行任务: ${agentContext.agent.task}`)

      const result = await this.retryExecution(agentContext)

      // 执行成功
      agentContext.updateStatus(AgentExecutionStatus.COMPLETED)
      agentContext.resetErrorCount()

      // 记录成功结果
      await agentContext.addMessage(
        ChatMessageRole.ASSISTANT,
        `任务执行成功: ${typeof result.data === 'string' ? result.data : JSON.stringify(result.data)}`
      )

      return result
    } catch (error) {
      const executionTime = Date.now() - startTime

      // 记录错误
      agentContext.recordError(error instanceof Error ? error : new Error(String(error)))

      // 记录错误消息
      await agentContext.addMessage(
        ChatMessageRole.SYSTEM,
        `任务执行失败: ${error instanceof Error ? error.message : String(error)}`
      )

      return this.handleError(error, executionTime)
    }
  }

  /**
   * 重试执行逻辑
   */
  private async retryExecution(agentContext: AgentContext): Promise<AgentResult> {
    let lastError: unknown

    for (let attempt = 1; attempt <= this.maxRetries; attempt++) {
      try {
        const result = await this.executeWithContext(agentContext)

        if (result.success) {
          return result
        } else {
          lastError = new Error(result.error || 'Unknown execution error')
          if (attempt < this.maxRetries) {
            await this.delay(this.retryDelay * attempt) // 指数退避
            await agentContext.addMessage(
              ChatMessageRole.SYSTEM,
              `第${attempt}次尝试失败，${this.retryDelay * attempt}ms后重试...`
            )
          }
        }
      } catch (error) {
        lastError = error
        if (attempt < this.maxRetries) {
          await this.delay(this.retryDelay * attempt)
          await agentContext.addMessage(
            ChatMessageRole.SYSTEM,
            `第${attempt}次尝试异常，${this.retryDelay * attempt}ms后重试: ${error instanceof Error ? error.message : String(error)}`
          )
        }
      }
    }

    throw lastError || new Error('All retry attempts failed')
  }

  /**
   * 通用的错误处理
   */
  protected handleError(error: unknown, executionTime: number): AgentResult {
    const errorMessage = error instanceof Error ? error.message : String(error)
    return {
      success: false,
      error: errorMessage,
      executionTime,
      metadata: {
        errorType: error instanceof Error ? error.constructor.name : 'UnknownError',
        timestamp: new Date().toISOString(),
        retryCount: this.maxRetries,
      },
    }
  }

  /**
   * 创建成功结果
   */
  protected createSuccessResult(data: unknown, executionTime: number, metadata?: Record<string, unknown>): AgentResult {
    return {
      success: true,
      data,
      executionTime,
      metadata: {
        timestamp: new Date().toISOString(),
        ...metadata,
      },
    }
  }

  /**
   * 延迟工具方法
   */
  protected delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

  /**
   * 设置重试配置
   */
  public setRetryConfig(maxRetries: number, retryDelay: number): void {
    this.maxRetries = maxRetries
    this.retryDelay = retryDelay
  }
}
