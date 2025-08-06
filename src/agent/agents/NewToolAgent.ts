/**
 * 新一代混合工具Agent
 *
 * 基于重构后的混合工具管理器实现的Agent
 * 支持智能决策使用Function Calling或内置工具
 */

import type { AgentExecutionContext, AgentResult } from '../types/execution'
import type { WorkflowAgent } from '../types/workflow'
import type { ExecutionContext } from '../tools/HybridToolManager'
import type { IAgent } from './BaseAgent'
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
 * 混合工具Agent实现
 */
export class NewToolAgent implements IAgent {
  private toolManager: HybridToolManager
  private config: Required<ToolAgentConfig>

  constructor(config: ToolAgentConfig = {}) {
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
   * 执行工具调用
   */
  async execute(agent: WorkflowAgent, context: AgentExecutionContext): Promise<AgentResult> {
    try {
      // 验证工具调用配置
      if (!agent.toolCall) {
        return {
          success: false,
          error: 'No tool call specified in agent configuration',
          agentId: agent.id,
          executionTime: 0,
        }
      }

      const { toolId, parameters } = agent.toolCall

      // 构建执行上下文
      const executionContext: ExecutionContext = {
        agentId: context.agentId,
        sessionId: context.sessionId,
        workflowId: context.workflowId,
        parameters: parameters || {},
        metadata: {
          ...context.metadata,
          agentConfig: agent,
          preferences: {
            strategy: this.config.decisionStrategy,
            maxExecutionTime: this.config.maxExecutionTime,
          },
        },
      }

      // 执行工具调用（带超时控制）
      const startTime = Date.now()
      const result = (await this.executeWithTimeout(toolId, executionContext)) as unknown
      const executionTime = Date.now() - startTime

      // 构建Agent执行结果
      const toolResult = result as any
      const agentResult: AgentResult = {
        success: toolResult.success,
        data: toolResult.data,
        error: toolResult.error,
        agentId: agent.id,
        executionTime,
        metadata: {
          ...toolResult.metadata,
          toolExecution: {
            toolId,
            executionMethod: toolResult.executionMethod,
            executionTime: toolResult.executionTime,
            decision: toolResult.metadata?.decision,
          },
        },
      }

      // 记录统计信息
      if (this.config.enableExecutionStats) {
        this.recordExecutionStats(agent, executionContext, result)
      }

      return agentResult
    } catch (error) {
      return {
        success: false,
        error: `Tool execution failed: ${error instanceof Error ? error.message : String(error)}`,
        agentId: agent.id,
        executionTime: Date.now(),
        metadata: {
          errorType: 'execution_error',
          originalError: error,
        },
      }
    }
  }

  /**
   * 带超时的工具执行
   */
  private async executeWithTimeout(toolId: string, context: ExecutionContext): Promise<unknown> {
    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        reject(new Error(`Tool execution timeout after ${this.config.maxExecutionTime}ms`))
      }, this.config.maxExecutionTime)

      this.toolManager
        .execute(toolId, context)
        .then(result => {
          clearTimeout(timeoutId)
          resolve(result)
        })
        .catch(error => {
          clearTimeout(timeoutId)
          reject(error)
        })
    })
  }

  /**
   * 记录执行统计
   */
  private recordExecutionStats(_agent: WorkflowAgent, _context: ExecutionContext, _result: any): void {
    // 静默执行，不输出调试日志
  }

  /**
   * 获取可用工具列表
   */
  getAvailableTools() {
    return this.toolManager.getTools()
  }

  /**
   * 获取工具执行统计
   */
  getExecutionStats(toolId?: string) {
    return this.toolManager.getExecutionStats(toolId)
  }

  /**
   * 更新配置
   */
  updateConfig(config: Partial<ToolAgentConfig>): void {
    Object.assign(this.config, config)

    if (config.decisionStrategy) {
      this.toolManager.setStrategy(config.decisionStrategy)
    }
  }

  /**
   * 检查工具是否可用
   */
  isToolAvailable(toolId: string): boolean {
    return this.toolManager.getTools().some(tool => tool.id === toolId)
  }

  /**
   * 验证工具参数
   */
  validateToolParameters(
    toolId: string,
    parameters: Record<string, any>
  ): {
    valid: boolean
    errors?: string[]
  } {
    const tool = this.toolManager.getTools().find(t => t.id === toolId)

    if (!tool) {
      return { valid: false, errors: [`Tool ${toolId} not found`] }
    }

    const errors: string[] = []

    // 检查必需参数
    const requiredParams = tool.parameters.filter(p => p.required)
    for (const param of requiredParams) {
      if (!(param.name in parameters) || parameters[param.name] === undefined || parameters[param.name] === null) {
        errors.push(`Required parameter '${param.name}' is missing`)
      }
    }

    // 检查参数类型
    for (const param of tool.parameters) {
      const value = parameters[param.name]
      if (value !== undefined && value !== null) {
        if (!this.validateParameterType(value, param.type)) {
          errors.push(`Parameter '${param.name}' must be of type ${param.type}`)
        }
      }
    }

    return { valid: errors.length === 0, errors: errors.length > 0 ? errors : undefined }
  }

  /**
   * 验证参数类型
   */
  private validateParameterType(value: any, expectedType: string): boolean {
    switch (expectedType) {
      case 'string':
        return typeof value === 'string'
      case 'number':
        return typeof value === 'number' && !isNaN(value)
      case 'boolean':
        return typeof value === 'boolean'
      case 'object':
        return typeof value === 'object' && value !== null && !Array.isArray(value)
      case 'array':
        return Array.isArray(value)
      default:
        return true // 未知类型，默认通过
    }
  }

  /**
   * 获取工具建议
   * 基于当前上下文和历史执行情况推荐最适合的工具
   */
  getSuggestedTools(context: { category?: string; task?: string; capabilities?: string[] }): Array<{
    toolId: string
    score: number
    reason: string
  }> {
    const tools = this.toolManager.getTools()
    const suggestions = []

    for (const tool of tools) {
      let score = 0
      const reasons = []

      // 基于分类匹配
      if (context.category && tool.category === context.category) {
        score += 0.5
        reasons.push('category match')
      }

      // 基于任务描述匹配
      if (context.task) {
        const taskLower = context.task.toLowerCase()
        if (tool.description.toLowerCase().includes(taskLower) || tool.name.toLowerCase().includes(taskLower)) {
          score += 0.3
          reasons.push('task relevance')
        }
      }

      // 基于能力匹配
      if (context.capabilities && tool.metadata?.requiredCapabilities) {
        const requiredCapabilities = tool.metadata.requiredCapabilities as string[]
        const matchedCapabilities = context.capabilities.filter(cap => requiredCapabilities.includes(cap))
        if (matchedCapabilities.length > 0) {
          score += 0.4 * (matchedCapabilities.length / context.capabilities.length)
          reasons.push('capability match')
        }
      }

      // 基于历史成功率
      const stats = this.toolManager.getExecutionStats(tool.id) as unknown as any
      if (stats && typeof stats === 'object' && stats.totalExecutions > 0) {
        const successRate = (stats.builtinExecutions + stats.functionCallingExecutions) / stats.totalExecutions
        score += 0.2 * successRate
        reasons.push('good success rate')
      }

      if (score > 0) {
        suggestions.push({
          toolId: tool.id,
          score,
          reason: reasons.join(', '),
        })
      }
    }

    return suggestions.sort((a, b) => b.score - a.score).slice(0, 5)
  }
}
