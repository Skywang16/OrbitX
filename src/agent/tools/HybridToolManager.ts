/**
 * 混合工具管理器 - 完全重构版本
 *
 * 结合Function Calling和内置工具的智能混合架构
 * 让LLM自主决策使用最佳工具执行方式
 */

import type { LLMProvider, LLMCallOptions, LLMResponse } from '../types/llm'
import { llmManager } from '../llm/LLMProvider'
import { promptEngine } from '../prompt/PromptEngine'

export type ToolType = 'builtin' | 'function_calling' | 'hybrid'
export type ExecutionStrategy = 'prefer_builtin' | 'prefer_function_calling' | 'intelligent_auto'

export interface HybridToolManagerConfig {
  enableLLMAssistedDecision?: boolean
  decisionConfidenceThreshold?: number
  maxLLMDecisionRetries?: number
  enableDecisionLogging?: boolean
}

export interface ToolDefinition {
  id: string
  name: string
  description: string
  category?: string
  type: ToolType
  parameters: ToolParameter[]
  functionCallSchema?: FunctionCallSchema
  builtinImplementation?: (params: Record<string, unknown>, context: ExecutionContext) => Promise<ToolResult>
  metadata?: Record<string, unknown>
}

export interface ToolParameter {
  name: string
  type: 'string' | 'number' | 'boolean' | 'object' | 'array'
  description: string
  required?: boolean
  default?: unknown
  enum?: unknown[]
  properties?: Record<string, ToolParameter>
  items?: ToolParameter
}

export interface FunctionCallSchema {
  type: 'function'
  function: {
    name: string
    description: string
    parameters: {
      type: 'object'
      properties: Record<string, unknown>
      required?: string[]
    }
  }
}

export interface ExecutionContext {
  agentId: string
  sessionId?: string
  workflowId?: string
  parameters: Record<string, unknown>
  metadata?: Record<string, unknown>
}

export interface ToolResult {
  success: boolean
  data?: unknown
  error?: string
  executionMethod?: 'builtin' | 'function_calling'
  executionTime?: number
  metadata?: Record<string, unknown>
}

export interface DecisionMetrics {
  builtinScore: number
  functionCallingScore: number
  confidenceLevel: number
  decisionFactors: string[]
  chosenStrategy: 'builtin' | 'function_calling'
  reasonSummary: string
}

/**
 * 混合工具管理器核心类
 */
export class HybridToolManager {
  private tools = new Map<string, ToolDefinition>()
  private llmProvider: LLMProvider
  private strategy: ExecutionStrategy = 'intelligent_auto'
  private executionStats = new Map<string, ExecutionStats>()
  private config: HybridToolManagerConfig

  constructor(llmProvider?: LLMProvider, config?: HybridToolManagerConfig) {
    this.llmProvider = llmProvider || llmManager.getProvider()!
    this.config = {
      enableLLMAssistedDecision: true,
      decisionConfidenceThreshold: 0.3,
      maxLLMDecisionRetries: 2,
      enableDecisionLogging: true,
      ...config,
    }
  }

  /**
   * 注册混合工具
   */
  registerTool(tool: ToolDefinition): void {
    this.tools.set(tool.id, tool)
    this.initializeStats(tool.id)
  }

  /**
   * 批量注册工具
   */
  registerTools(tools: ToolDefinition[]): void {
    tools.forEach(tool => this.registerTool(tool))
  }

  /**
   * 智能执行工具 - 核心方法
   */
  async execute(toolId: string, context: ExecutionContext): Promise<ToolResult> {
    const tool = this.tools.get(toolId)
    if (!tool) {
      throw new Error(`Tool ${toolId} not found`)
    }

    const startTime = Date.now()

    try {
      // 智能决策执行方式
      const decision = await this.makeExecutionDecision(tool, context)
      let result: ToolResult

      if (decision.chosenStrategy === 'builtin') {
        result = await this.executeBuiltin(tool, context)
      } else {
        result = await this.executeFunctionCalling(tool, context)
      }

      // 增加决策和性能信息
      result.executionMethod = decision.chosenStrategy
      result.executionTime = Date.now() - startTime
      result.metadata = {
        ...result.metadata,
        decision: decision,
        strategy: this.strategy,
      }

      // 更新统计
      this.updateExecutionStats(toolId, result)

      return result
    } catch (error) {
      const errorResult: ToolResult = {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        executionTime: Date.now() - startTime,
        metadata: { toolId, strategy: this.strategy },
      }

      this.updateExecutionStats(toolId, errorResult)
      return errorResult
    }
  }

  /**
   * 智能决策核心算法
   */
  private async makeExecutionDecision(tool: ToolDefinition, context: ExecutionContext): Promise<DecisionMetrics> {
    // 强制策略检查
    if (this.strategy === 'prefer_builtin' && tool.builtinImplementation) {
      return this.createDecisionMetrics('builtin', 1.0, ['forced_builtin_strategy'], 'Forced builtin by strategy')
    }

    if (this.strategy === 'prefer_function_calling' && tool.functionCallSchema) {
      return this.createDecisionMetrics(
        'function_calling',
        1.0,
        ['forced_function_calling_strategy'],
        'Forced function calling by strategy'
      )
    }

    // 智能自动决策
    return this.performIntelligentDecision(tool, context)
  }

  /**
   * 智能决策算法 (结合统计学和LLM辅助)
   */
  private async performIntelligentDecision(tool: ToolDefinition, context: ExecutionContext): Promise<DecisionMetrics> {
    const factors = this.analyzeTool(tool, context)
    const stats = this.executionStats.get(tool.id)

    let builtinScore = 0
    let functionCallingScore = 0
    const decisionFactors: string[] = []

    // 内置工具优势场景
    if (factors.hasBuiltinImplementation) {
      builtinScore += 0.4
      decisionFactors.push('has_builtin_implementation')
    }

    if (factors.isTerminalOperation) {
      builtinScore += 0.5 // 终端操作强烈偏向内置
      decisionFactors.push('terminal_operation_advantage')
    }

    if (factors.requiresRealTimeExecution) {
      builtinScore += 0.3
      decisionFactors.push('realtime_execution_needed')
    }

    if (stats && stats.builtinSuccessRate > 0.8) {
      builtinScore += 0.2
      decisionFactors.push('high_builtin_success_rate')
    }

    // Function Calling优势场景
    if (factors.hasComplexParameters) {
      functionCallingScore += 0.3
      decisionFactors.push('complex_parameters')
    }

    if (factors.requiresNaturalLanguageProcessing) {
      functionCallingScore += 0.4
      decisionFactors.push('nlp_processing_needed')
    }

    if (factors.contextAwareProcessing) {
      functionCallingScore += 0.3
      decisionFactors.push('context_aware_processing')
    }

    if (!factors.hasBuiltinImplementation) {
      functionCallingScore += 0.6 // 没有内置实现时强烈偏向Function Calling
      decisionFactors.push('no_builtin_available')
    }

    // 动态权重调整
    if (factors.userPreference === 'speed') {
      builtinScore *= 1.2
      decisionFactors.push('speed_preference')
    } else if (factors.userPreference === 'intelligence') {
      functionCallingScore *= 1.2
      decisionFactors.push('intelligence_preference')
    }

    // 归一化分数
    const totalScore = builtinScore + functionCallingScore
    if (totalScore > 0) {
      builtinScore = builtinScore / totalScore
      functionCallingScore = functionCallingScore / totalScore
    }

    // 统计学决策
    let chosenStrategy: 'builtin' | 'function_calling' =
      builtinScore > functionCallingScore ? 'builtin' : 'function_calling'
    const confidenceLevel = Math.abs(builtinScore - functionCallingScore)

    // 当置信度较低时，使用LLM辅助决策
    if (confidenceLevel < 0.3 && this.config.enableLLMAssistedDecision) {
      try {
        const llmRecommendation = await this.getLLMDecisionRecommendation(tool, {
          builtinScore,
          functionCallingScore,
          decisionFactors,
          stats,
        })
        if (llmRecommendation) {
          chosenStrategy = llmRecommendation
          decisionFactors.push('llm_assisted_decision')
        }
      } catch (error) {
        // eslint-disable-next-line no-console
        console.warn(`LLM-assisted decision failed, using statistical decision: ${error}`)
      }
    }

    const reasonSummary = this.generateDecisionReason(
      chosenStrategy,
      builtinScore,
      functionCallingScore,
      decisionFactors
    )

    return {
      builtinScore,
      functionCallingScore,
      confidenceLevel,
      decisionFactors,
      chosenStrategy,
      reasonSummary,
    }
  }

  /**
   * 分析工具特征
   */
  private analyzeTool(tool: ToolDefinition, context: ExecutionContext) {
    return {
      hasBuiltinImplementation: !!tool.builtinImplementation,
      isTerminalOperation: tool.category === 'terminal',
      requiresRealTimeExecution: this.isRealTimeOperation(tool),
      hasComplexParameters: this.hasComplexParameters(tool),
      requiresNaturalLanguageProcessing: this.requiresNLP(tool),
      contextAwareProcessing: this.needsContextAwareness(tool, context),
      userPreference: this.getUserPreference(context),
    }
  }

  /**
   * 执行内置工具
   */
  private async executeBuiltin(tool: ToolDefinition, context: ExecutionContext): Promise<ToolResult> {
    if (!tool.builtinImplementation) {
      throw new Error(`Tool ${tool.id} has no builtin implementation`)
    }

    const result = await tool.builtinImplementation(context.parameters, context)
    return {
      ...result,
      executionMethod: 'builtin',
    }
  }

  /**
   * 执行Function Calling
   */
  private async executeFunctionCalling(tool: ToolDefinition, context: ExecutionContext): Promise<ToolResult> {
    if (!tool.functionCallSchema) {
      throw new Error(`Tool ${tool.id} has no function call schema`)
    }

    const prompt = this.buildFunctionCallPrompt(tool, context)
    const options: LLMCallOptions = {
      tools: [tool],
      toolChoice: { type: 'function', function: { name: tool.name } },
      temperature: 0.1,
      maxTokens: 4000,
    }

    try {
      const llmResponse = await this.llmProvider.call(prompt, options)
      return this.processFunctionCallResponse(llmResponse)
    } catch (error) {
      throw new Error(`Function calling failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 构建Function Call提示
   */
  private buildFunctionCallPrompt(tool: ToolDefinition, context: ExecutionContext): string {
    // 准备参数信息
    const parametersInfo = tool.parameters
      .map(param => {
        return `- ${param.name} (${param.type}${param.required ? ', 必需' : ', 可选'}): ${param.description}`
      })
      .join('\n')

    // 检查是否为终端工具
    const isTerminalTool = tool.category === 'terminal' || tool.metadata?.terminalSpecific === true

    try {
      return promptEngine.generate('tool-function-call', {
        variables: {
          toolName: tool.name,
          toolDescription: tool.description,
          parametersInfo,
          isTerminalTool,
          parameters: JSON.stringify(context.parameters, null, 2),
        },
      })
    } catch (error) {
      console.warn(`Tool function call template not found, using fallback: ${error}`)
    }
  }

  /**
   * 处理Function Call响应
   */
  private processFunctionCallResponse(response: LLMResponse): ToolResult {
    if (response.toolCalls && response.toolCalls.length > 0) {
      const toolCall = response.toolCalls[0]

      try {
        const result = JSON.parse(toolCall.function.arguments)
        return {
          success: true,
          data: result,
          executionMethod: 'function_calling',
          metadata: {
            toolCallId: toolCall.id,
            llmResponse: response.content,
            functionName: toolCall.function.name,
          },
        }
      } catch (error) {
        return {
          success: false,
          error: `Failed to parse function call result: ${error}`,
          executionMethod: 'function_calling',
          metadata: {
            rawResponse: response.content,
            toolCall: toolCall,
          },
        }
      }
    }

    // 如果没有工具调用，尝试解析文本响应
    return this.parseTextResponse(response.content)
  }

  /**
   * 解析文本响应
   */
  private parseTextResponse(content: string): ToolResult {
    try {
      // 尝试提取JSON
      const jsonMatch = content.match(/\{[\s\S]*\}/)
      if (jsonMatch) {
        const data = JSON.parse(jsonMatch[0])
        return {
          success: true,
          data,
          executionMethod: 'function_calling',
          metadata: { parsedFromText: true },
        }
      }
    } catch (error) {
      // JSON解析失败，返回文本内容
    }

    return {
      success: true,
      data: { textResponse: content },
      executionMethod: 'function_calling',
      metadata: { responseType: 'text' },
    }
  }

  // 辅助方法
  private isRealTimeOperation(tool: ToolDefinition): boolean {
    const realTimeCategories = ['terminal', 'system', 'file', 'network']
    return realTimeCategories.includes(tool.category || '')
  }

  private hasComplexParameters(tool: ToolDefinition): boolean {
    return tool.parameters.some(p => p.type === 'object' || p.type === 'array') || tool.parameters.length > 5
  }

  private requiresNLP(tool: ToolDefinition): boolean {
    const nlpCategories = ['text', 'analysis', 'ai', 'language']
    return (
      nlpCategories.includes(tool.category || '') ||
      tool.description.includes('自然语言') ||
      tool.description.includes('智能分析')
    )
  }

  private needsContextAwareness(tool: ToolDefinition, context: ExecutionContext): boolean {
    return (
      !!(context.metadata && Object.keys(context.metadata).length > 0) ||
      tool.parameters.some(p => p.name.includes('context'))
    )
  }

  private getUserPreference(context: ExecutionContext): 'speed' | 'intelligence' | 'balanced' {
    return (context.metadata?.preference as 'speed' | 'intelligence' | 'balanced') || 'balanced'
  }

  private createDecisionMetrics(
    strategy: 'builtin' | 'function_calling',
    confidence: number,
    factors: string[],
    reason: string
  ): DecisionMetrics {
    return {
      builtinScore: strategy === 'builtin' ? confidence : 1 - confidence,
      functionCallingScore: strategy === 'function_calling' ? confidence : 1 - confidence,
      confidenceLevel: confidence,
      decisionFactors: factors,
      chosenStrategy: strategy,
      reasonSummary: reason,
    }
  }

  private generateDecisionReason(
    strategy: 'builtin' | 'function_calling',
    builtinScore: number,
    functionCallingScore: number,
    factors: string[]
  ): string {
    const score = strategy === 'builtin' ? builtinScore : functionCallingScore
    return `选择${strategy === 'builtin' ? '内置执行' : '函数调用'}方式 (得分: ${score.toFixed(3)})，主要因素: ${factors.join(', ')}`
  }

  private initializeStats(toolId: string): void {
    this.executionStats.set(toolId, {
      totalExecutions: 0,
      builtinExecutions: 0,
      functionCallingExecutions: 0,
      builtinSuccessRate: 0,
      functionCallingSuccessRate: 0,
      averageExecutionTime: 0,
    })
  }

  private updateExecutionStats(toolId: string, result: ToolResult): void {
    const stats = this.executionStats.get(toolId)!
    stats.totalExecutions++

    if (result.executionMethod === 'builtin') {
      stats.builtinExecutions++
      if (result.success) {
        stats.builtinSuccessRate =
          (stats.builtinSuccessRate * (stats.builtinExecutions - 1) + 1) / stats.builtinExecutions
      } else {
        stats.builtinSuccessRate = (stats.builtinSuccessRate * (stats.builtinExecutions - 1)) / stats.builtinExecutions
      }
    } else if (result.executionMethod === 'function_calling') {
      stats.functionCallingExecutions++
      if (result.success) {
        stats.functionCallingSuccessRate =
          (stats.functionCallingSuccessRate * (stats.functionCallingExecutions - 1) + 1) /
          stats.functionCallingExecutions
      } else {
        stats.functionCallingSuccessRate =
          (stats.functionCallingSuccessRate * (stats.functionCallingExecutions - 1)) / stats.functionCallingExecutions
      }
    }

    if (result.executionTime) {
      stats.averageExecutionTime =
        (stats.averageExecutionTime * (stats.totalExecutions - 1) + result.executionTime) / stats.totalExecutions
    }
  }

  /**
   * 获取工具列表
   */
  getTools(): ToolDefinition[] {
    return Array.from(this.tools.values())
  }

  /**
   * 获取执行统计
   */
  getExecutionStats(toolId?: string): ExecutionStats | Map<string, ExecutionStats> {
    if (toolId) {
      return this.executionStats.get(toolId) || this.createEmptyStats()
    }
    return this.executionStats
  }

  /**
   * 设置执行策略
   */
  setStrategy(strategy: ExecutionStrategy): void {
    this.strategy = strategy
  }

  private createEmptyStats(): ExecutionStats {
    return {
      totalExecutions: 0,
      builtinExecutions: 0,
      functionCallingExecutions: 0,
      builtinSuccessRate: 0,
      functionCallingSuccessRate: 0,
      averageExecutionTime: 0,
    }
  }

  /**
   * LLM辅助决策方法
   */
  private async getLLMDecisionRecommendation(
    tool: ToolDefinition,
    decisionData: {
      builtinScore: number
      functionCallingScore: number
      decisionFactors: string[]
      stats?: ExecutionStats
    }
  ): Promise<'builtin' | 'function_calling' | null> {
    try {
      // 准备决策因子信息
      const decisionFactors = decisionData.decisionFactors
        .map(factor => {
          return this.getFactorDescription(factor)
        })
        .join('\n- ')

      // 准备历史数据
      const historicalData = decisionData.stats
        ? `
内置执行成功率: ${(decisionData.stats.builtinSuccessRate * 100).toFixed(1)}%
函数调用成功率: ${(decisionData.stats.functionCallingSuccessRate * 100).toFixed(1)}%
平均执行时间: ${decisionData.stats.averageExecutionTime}ms
总执行次数: ${decisionData.stats.totalExecutions}`
        : '暂无历史数据'

      const prompt = promptEngine.generate('tool-decision', {
        variables: {
          toolName: tool.name,
          toolDescription: tool.description,
          toolType: tool.type,
          toolCategory: tool.category || 'general',
          decisionFactors: `- ${decisionFactors}`,
          hasHistoricalData: !!decisionData.stats,
          historicalData,
        },
      })

      const response = await this.llmProvider.call(prompt, {
        temperature: 0.1,
        maxTokens: 100,
      })

      const decision = response.content.toLowerCase().trim()
      if (decision.includes('builtin')) {
        return 'builtin'
      } else if (decision.includes('function_calling')) {
        return 'function_calling'
      }

      return null
    } catch (error) {
      // eslint-disable-next-line no-console
      console.warn(`LLM decision recommendation failed: ${error}`)
      return null
    }
  }

  /**
   * 获取决策因子描述
   */
  private getFactorDescription(factor: string): string {
    const descriptions: Record<string, string> = {
      has_builtin_implementation: '工具有内置实现',
      terminal_operation_advantage: '终端操作优势 (内置更快)',
      realtime_execution_needed: '需要实时执行',
      high_builtin_success_rate: '内置实现成功率高',
      complex_parameters: '参数复杂 (LLM处理更好)',
      nlp_processing_needed: '需要自然语言处理',
      context_aware_processing: '需要上下文感知处理',
      no_builtin_available: '无内置实现可用',
      speed_preference: '用户偏好速度',
      intelligence_preference: '用户偏好智能处理',
      llm_assisted_decision: '使用了LLM辅助决策',
    }
    return descriptions[factor] || factor
  }
}

interface ExecutionStats {
  totalExecutions: number
  builtinExecutions: number
  functionCallingExecutions: number
  builtinSuccessRate: number
  functionCallingSuccessRate: number
  averageExecutionTime: number
}
