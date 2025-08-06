/**
 * 规划器组件
 *
 * 负责将自然语言任务转换为JSON格式的工作流定义
 */

import { llmManager } from '../llm/LLMProvider'
import type { LLMCallOptions } from '../types/llm'
import type { ToolDefinition } from '../types/tool'
import type { WorkflowDefinition } from '../types/workflow'
import { MemoryManager } from '../core/MemoryManager'
import { promptEngine } from '../prompt/PromptEngine'

/**
 * 规划结果
 */
export interface PlanningResult {
  success: boolean
  workflow?: WorkflowDefinition
  error?: string
  reasoning?: string
  prompt: string
  rawResponse: string
}

/**
 * Planner 接口，定义了规划器的公共契约
 */
export interface IPlanner {
  planTask(
    userInput: string,
    options?: {
      model?: string
      availableTools?: ToolDefinition[]
      includeThought?: boolean
    }
  ): Promise<PlanningResult>

  replanTask(
    newUserInput: string,
    previousResult: PlanningResult,
    options?: {
      model?: string
      availableTools?: ToolDefinition[]
    }
  ): Promise<PlanningResult>
}

/**
 * 规划器类
 */
export class Planner implements IPlanner {
  private memoryManager: MemoryManager

  constructor() {
    this.memoryManager = new MemoryManager()
  }

  /**
   * 规划新任务
   */
  async planTask(
    userInput: string,
    options?: {
      model?: string
      availableTools?: ToolDefinition[]
      includeThought?: boolean
    }
  ): Promise<PlanningResult> {
    const prompt = this.generatePlanningPrompt(userInput, options?.availableTools)
    return this._executePlanning(prompt, options)
  }

  /**
   * 重新规划任务
   */
  async replanTask(
    newUserInput: string,
    previousResult: PlanningResult,
    options?: {
      model?: string
      availableTools?: ToolDefinition[]
    }
  ): Promise<PlanningResult> {
    const prompt = this.generateReplanningPrompt(newUserInput, previousResult, options?.availableTools)
    return this._executePlanning(prompt, options)
  }

  /**
   * 执行规划的核心逻辑
   */
  private async _executePlanning(prompt: string, options?: { model?: string }): Promise<PlanningResult> {
    try {
      const llmOptions: LLMCallOptions = {
        model: options?.model || 'claude-3-sonnet',
        temperature: 0.1,
        maxTokens: 4000,
      }

      const response = await llmManager.call(prompt, llmOptions)
      const parseResult = this.parseWorkflowJSON(response.content)

      if (parseResult.success && parseResult.workflow) {
        return {
          success: true,
          workflow: parseResult.workflow,
          reasoning: parseResult.workflow.thought || '',
          prompt: prompt,
          rawResponse: response.content,
        }
      } else {
        return {
          success: false,
          error: parseResult.error || '工作流解析失败',
          prompt: prompt,
          rawResponse: response.content,
        }
      }
    } catch (error) {
      const errorMessage = `规划失败: ${error instanceof Error ? error.message : String(error)}`
      return {
        success: false,
        error: errorMessage,
        prompt: prompt,
        rawResponse: '',
      }
    }
  }

  /**
   * 使用 PromptEngine 生成初次规划的提示词
   */
  private generatePlanningPrompt(userInput: string, availableTools: ToolDefinition[] = []): string {
    const hasTools = availableTools.length > 0
    const toolsJson = hasTools
      ? JSON.stringify(
          availableTools.map(t => ({
            id: t.id,
            name: t.name,
            description: t.description,
            parameters: t.parameters,
          })),
          null,
          2
        )
      : ''

    return promptEngine.generate('planner-main', {
      variables: {
        userInput,
        hasTools,
        tools: toolsJson,
        timestamp: Date.now(),
      },
    })
  }

  /**
   * 使用 PromptEngine 生成重规划的提示词
   */
  private generateReplanningPrompt(
    newUserInput: string,
    previousResult: PlanningResult,
    availableTools: ToolDefinition[] = []
  ): string {
    const hasTools = availableTools.length > 0
    const toolsJson = hasTools
      ? JSON.stringify(
          availableTools.map(t => ({
            id: t.id,
            name: t.name,
            description: t.description,
            parameters: t.parameters,
          })),
          null,
          2
        )
      : ''

    return promptEngine.generate('planner-replan', {
      variables: {
        newUserInput,
        hasTools,
        tools: toolsJson,
        previousPlan: previousResult.rawResponse,
      },
    })
  }

  /**
   * 解析JSON工作流
   */
  private parseWorkflowJSON(jsonContent: string): {
    success: boolean
    workflow?: WorkflowDefinition
    error?: string
  } {
    try {
      const jsonMatch = jsonContent.match(/```json\s*([\s\S]*?)\s*```/i)
      let jsonStr = jsonContent

      if (jsonMatch && jsonMatch[1]) {
        jsonStr = jsonMatch[1]
      } else {
        const firstBrace = jsonStr.indexOf('{')
        const lastBrace = jsonStr.lastIndexOf('}')
        if (firstBrace !== -1 && lastBrace > firstBrace) {
          jsonStr = jsonStr.substring(firstBrace, lastBrace + 1)
        } else {
          return { success: false, error: '未找到有效的JSON内容' }
        }
      }

      const workflow = JSON.parse(jsonStr) as WorkflowDefinition

      if (!workflow.taskId) workflow.taskId = `task_${Date.now()}`
      if (!workflow.name) workflow.name = '未命名工作流'
      if (!workflow.agents) workflow.agents = []

      return { success: true, workflow }
    } catch (error) {
      return {
        success: false,
        error: `JSON解析错误: ${error instanceof Error ? error.message : String(error)}`,
      }
    }
  }

  /**
   * 获取Memory管理器
   */
  getMemoryManager(): MemoryManager {
    return this.memoryManager
  }
}
