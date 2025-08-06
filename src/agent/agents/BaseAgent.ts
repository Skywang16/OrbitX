/**
 * Agent基础接口定义
 *
 * 定义了所有Agent必须实现的核心接口
 */

import type { AgentExecutionContext, AgentResult } from '../types/execution'
import type { WorkflowAgent } from '../types/workflow'

/**
 * Agent基础接口
 */
export interface IAgent {
  /**
   * 执行Agent任务
   *
   * @param agent 工作流Agent定义
   * @param context 执行上下文
   * @returns Agent执行结果
   */
  execute(agent: WorkflowAgent, context: AgentExecutionContext): Promise<AgentResult>
}

/**
 * Agent基础抽象类
 */
export abstract class BaseAgent implements IAgent {
  /**
   * 子类必须实现的执行方法
   */
  abstract execute(agent: WorkflowAgent, context: AgentExecutionContext): Promise<AgentResult>

  /**
   * 通用的错误处理
   */
  protected handleError(error: unknown, agentId: string, executionTime: number): AgentResult {
    const errorMessage = error instanceof Error ? error.message : String(error)
    return {
      success: false,
      error: errorMessage,
      agentId,
      executionTime,
      metadata: {
        errorType: error instanceof Error ? error.constructor.name : 'UnknownError',
        timestamp: new Date().toISOString(),
      },
    }
  }

  /**
   * 创建成功结果
   */
  protected createSuccessResult(
    data: unknown,
    agentId: string,
    executionTime: number,
    metadata?: Record<string, unknown>
  ): AgentResult {
    return {
      success: true,
      data,
      agentId,
      executionTime,
      metadata: {
        timestamp: new Date().toISOString(),
        ...metadata,
      },
    }
  }
}
