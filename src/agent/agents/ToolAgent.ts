/**
 * 工具Agent
 */

import type { ExecutionContext } from '../tools/HybridToolManager'
import { BaseAgent, AgentResult } from './BaseAgent'
import { AgentContext } from '../context/AgentContext'
import { HybridToolManager, globalToolManager } from '../tools'

export interface ToolAgentConfig {
  toolManager?: HybridToolManager
  enableFunctionCalling?: boolean
  enableBuiltinTools?: boolean
  decisionStrategy?: 'prefer_builtin' | 'prefer_function_calling' | 'intelligent_auto'
  maxExecutionTime?: number
  enableExecutionStats?: boolean
}

/**
 * 工具Agent实现
 * 增强版本：继承BaseAgent，支持上下文管理
 */
export class ToolAgent extends BaseAgent {
  private toolManager: HybridToolManager
  private config: Required<ToolAgentConfig>

  constructor(config: ToolAgentConfig = {}) {
    super()
    this.config = {
      toolManager: config.toolManager || globalToolManager,
      enableFunctionCalling: config.enableFunctionCalling ?? true,
      enableBuiltinTools: config.enableBuiltinTools ?? true,
      decisionStrategy: config.decisionStrategy || 'intelligent_auto',
      maxExecutionTime: config.maxExecutionTime || 120000, // 2分钟
      enableExecutionStats: config.enableExecutionStats ?? true,
    }

    this.toolManager = this.config.toolManager
    this.toolManager.setStrategy(this.config.decisionStrategy)
  }

  /**
   * 使用AgentContext执行工具调用
   */
  public async executeWithContext(agentContext: AgentContext): Promise<AgentResult> {
    const startTime = Date.now()

    try {
      // 验证工具调用配置
      if (!agentContext.agent.toolCall) {
        return this.handleError(new Error('No tool call specified in agent configuration'), Date.now() - startTime)
      }

      const { toolId, parameters } = agentContext.agent.toolCall

      // 构建执行上下文
      const executionContext: ExecutionContext = {
        agentId: agentContext.agent.id,
        sessionId: agentContext.taskContext.taskId,
        workflowId: agentContext.taskContext.taskId,
        parameters: parameters || {},
        metadata: {
          agentConfig: agentContext.agent,
          taskContext: agentContext.taskContext,
          agentContext,
          preferences: {
            strategy: this.config.decisionStrategy,
            maxExecutionTime: this.config.maxExecutionTime,
          },
        },
      }

      // 记录工具调用开始
      await agentContext.addMessage('system', `开始调用工具: ${toolId}`)

      // 执行工具调用
      const result = await this.toolManager.execute(toolId, executionContext)

      // 记录工具调用结果
      const resultString = typeof result === 'string' ? result : JSON.stringify(result)
      await agentContext.addMessage('assistant', `工具调用完成: ${resultString}`)

      return this.createSuccessResult(result, Date.now() - startTime, {
        toolId,
        parameters,
        strategy: this.config.decisionStrategy,
      })
    } catch (error) {
      // 记录错误
      await agentContext.addMessage('system', `工具调用失败: ${error instanceof Error ? error.message : String(error)}`)

      return this.handleError(error, Date.now() - startTime)
    }
  }
}
