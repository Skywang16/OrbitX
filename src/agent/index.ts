/**
 * Agent框架
 */

// 类型导出（用于TypeScript用户）
export type { ExecutionResult, ExecutionEvent } from './types/execution'
export type { ExecutionCallback, ProgressCallback, ProgressMessage, CallbackOptions } from './types/callbacks'
export { AgentError, ToolError, WorkflowError } from './types/errors'

// 导出上下文管理
export { TaskContext } from './context/TaskContext'
export { AgentContext, AgentExecutionStatus } from './context/AgentContext'
export type { AgentSnapshot, AgentExecutionStats } from './context/AgentContext'

// 导出核心管理器
export { MemoryManager } from './core/MemoryManager'
export type { MemoryManagerConfig } from './core/MemoryManager'
export { TaskSnapshotManager } from './core/TaskSnapshotManager'
export type { TaskSnapshot, SnapshotConfig } from './core/TaskSnapshotManager'
export { CallbackManager } from './core/CallbackManager'

// 导出Agent基类
export { BaseAgent } from './agents/BaseAgent'
export type { IAgent, AgentResult } from './agents/BaseAgent'
export { ToolAgent } from './agents/ToolAgent'

// 导出更多类型定义
export * from './types/agent'
export * from './types/workflow'
export * from './types/memory'

import { Planner } from './planning/Planner'
import { ExecutionEngine } from './execution/ExecutionEngine'
import type { IExecutionEngine } from './types/execution'
import type { IPlanner } from './planning/Planner'
import type { ExecutionCallback, ProgressCallback } from './types/callbacks'
import { CallbackManager } from './core/CallbackManager'
import { getAllTerminalTools } from './tools/TerminalToolKit'
import { promptEngine } from './prompt/PromptEngine'
import { llmManager } from './llm/LLMProvider'

/**
 * Agent框架配置
 */
export interface AgentFrameworkConfig {
  // 预留配置接口
}

/** Agent框架 */
export class AgentFramework {
  private planner: IPlanner
  private engine: IExecutionEngine
  private callbackManager: CallbackManager

  constructor(
    config: AgentFrameworkConfig = {},
    planner?: IPlanner,
    engine?: IExecutionEngine,
    callbackManager?: CallbackManager
  ) {
    this.planner = planner || new Planner()
    this.callbackManager = callbackManager || new CallbackManager()
    this.engine = engine || new ExecutionEngine(config, this.callbackManager)
  }

  /**
   * 执行任务
   */
  async execute(
    taskDescription: string,
    options?: {
      model?: string
      onProgress?: (message: { type: string; content: string; data?: unknown }) => void
    }
  ) {
    try {
      // Agent开始思考和规划
      options?.onProgress?.({ type: 'thinking', content: '正在理解任务...' })

      const availableTools = getAllTerminalTools()
      const planResult = await this.planner.planTask(taskDescription, {
        model: options?.model,
        includeThought: true,
        availableTools,
      })

      if (!planResult.success || !planResult.workflow) {
        return {
          success: false,
          error: `抱歉，我无法理解或完成这个任务: ${planResult.error || '任务描述可能不够清晰'}`,
        }
      }

      options?.onProgress?.({
        type: 'planning',
        content: `我理解了，需要执行${planResult.workflow.agents.length}个步骤`,
        data: { plan: planResult.workflow.name },
      })

      // Agent开始自主执行
      options?.onProgress?.({ type: 'working', content: '开始执行任务...' })

      const executionResult = await this.engine.execute(planResult.workflow, {}, async event => {
        // 只显示用户关心的进度，不暴露内部细节
        if (event.type === 'agent_start') {
          options?.onProgress?.({
            type: 'progress',
            content: '正在处理...',
            data: { step: event.agentId },
          })
        }
      })

      if (!executionResult.success) {
        return {
          success: false,
          result: executionResult.result,
          error: `任务执行遇到问题: ${executionResult.result}`,
        }
      }

      // 任务执行成功，基于规划思路和实际结果生成友好回答
      options?.onProgress?.({ type: 'progress', content: '正在整理回答...', data: { stage: 'summarizing' } })

      try {
        const summaryPrompt = promptEngine.generate('result-summary', {
          variables: {
            userInput: taskDescription,
            planningThought: planResult.workflow.thought,
            executionResult: executionResult.result,
          },
        })

        const llmResponse = await llmManager.call(summaryPrompt, {
          model: options?.model,
        })

        if (llmResponse.content?.trim()) {
          return {
            success: true,
            result: llmResponse.content.trim(),
            error: undefined,
          }
        }
      } catch (error) {
        // 静默处理总结失败，返回原始结果
      }

      // 如果生成失败，返回原始结果
      return {
        success: true,
        result: executionResult.result,
        error: undefined,
      }
    } catch (error) {
      return {
        success: false,
        error: `抱歉，执行任务时出现了问题: ${error instanceof Error ? error.message : String(error)}`,
      }
    }
  }

  /**
   * 获取回调管理器实例
   * 用于注册自定义回调函数
   */
  getCallbackManager(): CallbackManager {
    return this.callbackManager
  }

  /**
   * 注册执行回调
   */
  onExecution(callback: ExecutionCallback): void {
    this.callbackManager.onExecution(callback)
  }

  /**
   * 注册进度回调
   */
  onProgress(callback: ProgressCallback): void {
    this.callbackManager.onProgress(callback)
  }
}

// 主要导出
export default AgentFramework
