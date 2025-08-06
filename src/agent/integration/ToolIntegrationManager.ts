/**
 * 工具集成管理器
 *
 * 负责将新的混合工具系统集成到现有的Agent框架中
 * 提供平滑的迁移和兼容性支持
 */

import type { AgentExecutionContext, AgentResult } from '../types/execution'
import type { WorkflowAgent } from '../types/workflow'
import { NewToolAgent, type ToolAgentConfig } from '../agents/NewToolAgent'
// import { globalToolManager } from '../tools'

/**
 * 集成配置
 */
export interface IntegrationConfig {
  enableHybridTools?: boolean
  enableLegacyFallback?: boolean
  migrationMode?: 'gradual' | 'immediate' | 'legacy_only'
  toolManagerConfig?: ToolAgentConfig
  performanceMonitoring?: boolean
}

/**
 * 执行统计
 */
interface IntegrationStats {
  totalExecutions: number
  hybridExecutions: number
  legacyExecutions: number
  successRate: number
  averageExecutionTime: number
  errorsByType: Record<string, number>
}

/**
 * 工具集成管理器
 */
export class ToolIntegrationManager {
  private hybridAgent: NewToolAgent
  private config: Required<IntegrationConfig>
  private stats: IntegrationStats
  private initialized = false

  constructor(config: IntegrationConfig = {}) {
    this.config = {
      enableHybridTools: config.enableHybridTools ?? true,
      enableLegacyFallback: config.enableLegacyFallback ?? true,
      migrationMode: config.migrationMode || 'gradual',
      toolManagerConfig: config.toolManagerConfig || {},
      performanceMonitoring: config.performanceMonitoring ?? true,
    }

    this.hybridAgent = new NewToolAgent(this.config.toolManagerConfig)
    this.stats = this.initializeStats()
  }

  /**
   * 初始化集成管理器
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return
    }

    console.log('[ToolIntegration] Initializing hybrid tool system...')

    // 验证工具可用性
    const availableTools = this.hybridAgent.getAvailableTools()
    console.log(`[ToolIntegration] Found ${availableTools.length} hybrid tools`)

    // 设置监控
    if (this.config.performanceMonitoring) {
      this.setupPerformanceMonitoring()
    }

    this.initialized = true
    console.log('[ToolIntegration] Hybrid tool system initialized successfully')
  }

  /**
   * 执行工具调用 - 主入口
   */
  async executeToolCall(agent: WorkflowAgent, context: AgentExecutionContext): Promise<AgentResult> {
    if (!this.initialized) {
      await this.initialize()
    }

    const startTime = Date.now()

    try {
      // 根据配置决定执行策略
      const shouldUseHybrid = this.shouldUseHybridExecution(agent, context)

      let result: AgentResult

      if (shouldUseHybrid && this.config.enableHybridTools) {
        result = await this.executeHybridTool(agent, context)
        this.stats.hybridExecutions++
      } else if (this.config.enableLegacyFallback) {
        result = await this.executeLegacyTool(agent, context)
        this.stats.legacyExecutions++
      } else {
        throw new Error('Neither hybrid tools nor legacy fallback are enabled')
      }

      // 更新统计
      this.updateExecutionStats(result, Date.now() - startTime)

      return result
    } catch (error) {
      const errorResult: AgentResult = {
        success: false,
        error: `Tool integration failed: ${error instanceof Error ? error.message : String(error)}`,
        agentId: agent.id,
        executionTime: Date.now() - startTime,
        metadata: {
          integrationError: true,
          errorType: 'integration_failure',
          originalError: error,
        },
      }

      this.updateExecutionStats(errorResult, Date.now() - startTime)
      return errorResult
    }
  }

  /**
   * 执行混合工具
   */
  private async executeHybridTool(agent: WorkflowAgent, context: AgentExecutionContext): Promise<AgentResult> {
    try {
      const result = await this.hybridAgent.execute(agent, context)

      // 添加集成标识
      result.metadata = {
        ...result.metadata,
        executionMode: 'hybrid',
        integrationVersion: '2.0',
      }

      return result
    } catch (error) {
      // 如果启用了回退机制，尝试使用传统方法
      if (this.config.enableLegacyFallback) {
        console.warn(`[ToolIntegration] Hybrid execution failed, falling back to legacy: ${error}`)
        return this.executeLegacyTool(agent, context)
      }
      throw error
    }
  }

  /**
   * 执行传统工具（兼容性回退）
   */
  private async executeLegacyTool(agent: WorkflowAgent, context: AgentExecutionContext): Promise<AgentResult> {
    // 这里可以调用原有的工具系统作为回退
    // 暂时返回一个基础实现
    return {
      success: false,
      error: 'Legacy tool execution not implemented in this refactored version',
      agentId: agent.id,
      executionTime: 0,
      metadata: {
        executionMode: 'legacy',
        note: 'This is a placeholder for legacy tool execution',
      },
    }
  }

  /**
   * 决定是否使用混合执行
   */
  private shouldUseHybridExecution(agent: WorkflowAgent, context: AgentExecutionContext): boolean {
    switch (this.config.migrationMode) {
      case 'immediate':
        return true

      case 'legacy_only':
        return false

      case 'gradual':
      default:
        return this.isAgentEligibleForHybrid(agent, context)
    }
  }

  /**
   * 检查Agent是否适合混合执行
   */
  private isAgentEligibleForHybrid(agent: WorkflowAgent, context: AgentExecutionContext): boolean {
    // 检查是否有工具调用配置
    if (!agent.toolCall) {
      return false
    }

    // 检查工具是否在混合系统中可用
    const isToolAvailable = this.hybridAgent.isToolAvailable(agent.toolCall.toolId)
    if (!isToolAvailable) {
      return false
    }

    // 检查参数有效性
    const validation = this.hybridAgent.validateToolParameters(agent.toolCall.toolId, agent.toolCall.parameters || {})

    return validation.valid
  }

  /**
   * 获取工具建议
   */
  getToolSuggestions(context: { category?: string; task?: string; capabilities?: string[] }) {
    return this.hybridAgent.getSuggestedTools(context)
  }

  /**
   * 获取可用工具列表
   */
  getAvailableTools() {
    return this.hybridAgent.getAvailableTools()
  }

  /**
   * 获取执行统计
   */
  getIntegrationStats(): IntegrationStats {
    return { ...this.stats }
  }

  /**
   * 获取工具执行统计
   */
  getToolExecutionStats(toolId?: string) {
    return this.hybridAgent.getExecutionStats(toolId)
  }

  /**
   * 更新配置
   */
  updateConfig(newConfig: Partial<IntegrationConfig>): void {
    Object.assign(this.config, newConfig)

    if (newConfig.toolManagerConfig) {
      this.hybridAgent.updateConfig(newConfig.toolManagerConfig)
    }
  }

  /**
   * 检查系统健康状态
   */
  getHealthStatus(): {
    status: 'healthy' | 'degraded' | 'unhealthy'
    issues: string[]
    stats: IntegrationStats
  } {
    const issues: string[] = []
    let status: 'healthy' | 'degraded' | 'unhealthy' = 'healthy'

    // 检查成功率
    if (this.stats.successRate < 0.8) {
      issues.push(`Low success rate: ${(this.stats.successRate * 100).toFixed(1)}%`)
      status = 'degraded'
    }

    if (this.stats.successRate < 0.5) {
      status = 'unhealthy'
    }

    // 检查执行时间
    if (this.stats.averageExecutionTime > 10000) {
      // 10秒
      issues.push(`High average execution time: ${this.stats.averageExecutionTime}ms`)
      if (status === 'healthy') status = 'degraded'
    }

    // 检查错误率
    const errorRate = Object.values(this.stats.errorsByType).reduce((a, b) => a + b, 0) / this.stats.totalExecutions
    if (errorRate > 0.2) {
      issues.push(`High error rate: ${(errorRate * 100).toFixed(1)}%`)
      if (status === 'healthy') status = 'degraded'
    }

    return {
      status,
      issues,
      stats: this.stats,
    }
  }

  /**
   * 重置统计信息
   */
  resetStats(): void {
    this.stats = this.initializeStats()
  }

  /**
   * 初始化统计信息
   */
  private initializeStats(): IntegrationStats {
    return {
      totalExecutions: 0,
      hybridExecutions: 0,
      legacyExecutions: 0,
      successRate: 0,
      averageExecutionTime: 0,
      errorsByType: {},
    }
  }

  /**
   * 更新执行统计
   */
  private updateExecutionStats(result: AgentResult, executionTime: number): void {
    this.stats.totalExecutions++

    // 更新成功率
    const totalSuccesses = this.stats.successRate * (this.stats.totalExecutions - 1) + (result.success ? 1 : 0)
    this.stats.successRate = totalSuccesses / this.stats.totalExecutions

    // 更新平均执行时间
    this.stats.averageExecutionTime =
      (this.stats.averageExecutionTime * (this.stats.totalExecutions - 1) + executionTime) / this.stats.totalExecutions

    // 更新错误统计
    if (!result.success && result.error) {
      const errorType = (result.metadata?.errorType as string) || 'unknown'
      this.stats.errorsByType[errorType] = (this.stats.errorsByType[errorType] || 0) + 1
    }
  }

  /**
   * 设置性能监控
   */
  private setupPerformanceMonitoring(): void {
    // 定期输出统计信息
    setInterval(() => {
      if (this.stats.totalExecutions > 0) {
        console.log('[ToolIntegration] Performance Stats:', {
          totalExecutions: this.stats.totalExecutions,
          hybridExecutions: this.stats.hybridExecutions,
          legacyExecutions: this.stats.legacyExecutions,
          successRate: `${(this.stats.successRate * 100).toFixed(1)}%`,
          avgExecutionTime: `${this.stats.averageExecutionTime.toFixed(0)}ms`,
        })
      }
    }, 60000) // 每分钟输出一次
  }
}

/**
 * 全局集成管理器实例
 */
export const globalIntegrationManager = new ToolIntegrationManager({
  enableHybridTools: true,
  enableLegacyFallback: true,
  migrationMode: 'gradual',
  performanceMonitoring: true,
})

/**
 * 便捷的工具执行函数
 */
export async function executeIntegratedTool(
  agent: WorkflowAgent,
  context: AgentExecutionContext
): Promise<AgentResult> {
  return globalIntegrationManager.executeToolCall(agent, context)
}
