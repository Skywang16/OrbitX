/**
 * 自主Agent框架
 *
 * 用户只需要描述任务，Agent自主完成规划、决策、执行
 *
 * @example
 * ```typescript
 * import { AgentFramework } from '@/agent'
 *
 * const agent = new AgentFramework()
 *
 * // 用户只需要描述任务，Agent自主完成一切
 * const result = await agent.execute("查看当前目录下的文件")
 * const result = await agent.execute("创建一个React项目并安装依赖")
 * const result = await agent.execute("分析系统性能并生成报告")
 * ```
 */

// 类型导出（用于TypeScript用户）
export type { ExecutionResult, ExecutionEvent } from './types/execution'
export type { ExecutionCallback, ProgressCallback, ProgressMessage, CallbackOptions } from './types/callbacks'
export { AgentError, ToolError, WorkflowError } from './types/errors'

// 内部组件导出（仅用于调试和扩展，生产环境不建议直接使用）
export { Planner } from './planning/Planner'
export { ExecutionEngine } from './execution/ExecutionEngine'
export { llmManager } from './llm/LLMProvider'
export { CallbackManager, globalCallbackManager } from './core/CallbackManager'

import { Planner } from './planning/Planner'
import { ExecutionEngine } from './execution/ExecutionEngine'
import type { IExecutionEngine } from './types/execution'
import type { IPlanner } from './planning/Planner'
import type { ExecutionCallback, ProgressCallback } from './types/callbacks'
import { CallbackManager } from './core/CallbackManager'
import { getBuiltinTools } from './tools/builtin'
import { promptEngine } from './prompt/PromptEngine'
import { llmManager } from './llm/LLMProvider'

/**
 * Agent框架配置
 */
export interface AgentFrameworkConfig {
  maxAgents?: number
  defaultTimeout?: number
  enablePersistence?: boolean
  enableParallelExecution?: boolean
  maxConcurrency?: number
  enableDynamicReplanning?: boolean
  maxReplanAttempts?: number
}

/**
 * 自主Agent框架
 *
 * 真正的自主决策系统：用户只需要描述任务，Agent自主完成一切
 * 不暴露内部规划和执行细节，就像与真人助手对话一样自然
 */
export class AgentFramework {
  private planner: IPlanner
  private engine: IExecutionEngine
  private callbackManager: CallbackManager

  constructor(
    _config: AgentFrameworkConfig = {},
    planner?: IPlanner,
    engine?: IExecutionEngine,
    callbackManager?: CallbackManager
  ) {
    this.planner = planner || new Planner()
    this.callbackManager = callbackManager || new CallbackManager()
    this.engine = engine || new ExecutionEngine(this.callbackManager)
  }

  /**
   * 主要API：自主执行任务
   *
   * 这是用户唯一需要的方法 - 就像对真人助手说话一样
   * Agent会自主完成：理解任务 -> 制定计划 -> 执行计划 -> 返回结果
   *
   * @example
   * ```typescript
   * const agent = new AgentFramework()
   *
   * // 就像对助手说话一样自然
   * await agent.execute("帮我看看当前目录有什么文件")
   * await agent.execute("创建一个新的React项目叫my-app")
   * await agent.execute("检查系统内存使用情况")
   * await agent.execute("把package.json里的版本号改成2.0.0")
   * ```
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

      const availableTools = getBuiltinTools()
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
   * 流式执行（用于需要实时反馈的UI）
   *
   * 适用于聊天界面或需要显示详细进度的场景
   */
  async executeWithStream(
    taskDescription: string,
    onMessage: (message: { type: string; content: string; timestamp: string; data?: unknown }) => Promise<void>,
    options?: { model?: string }
  ) {
    const sendMessage = (type: string, content: string, data?: unknown) =>
      onMessage({ type, content, timestamp: new Date().toISOString(), data })

    try {
      await sendMessage('start', '我来帮你完成这个任务...')

      const availableTools = getBuiltinTools()
      const planResult = await this.planner.planTask(taskDescription, {
        model: options?.model,
        includeThought: true,
        availableTools,
      })

      if (!planResult.success || !planResult.workflow) {
        await sendMessage('error', `抱歉，我无法理解这个任务: ${planResult.error || '请尝试更详细地描述'}`)
        return { success: false, error: planResult.error || '任务理解失败' }
      }

      await sendMessage('understood', `好的，我需要执行${planResult.workflow.agents.length}个步骤来完成`)

      const executionResult = await this.engine.execute(planResult.workflow, {}, async event => {
        // 只发送用户友好的消息
        if (event.type === 'agent_start') {
          await sendMessage('working', '正在处理中...')
        } else if (event.type === 'agent_completed') {
          await sendMessage('progress', '完成了一个步骤')
        }
      })

      const finalMessage = executionResult.success ? '任务完成！' : '任务执行遇到了问题'
      await sendMessage('complete', finalMessage, { result: executionResult.result })

      return {
        success: executionResult.success,
        result: executionResult.result,
        error: executionResult.success ? undefined : executionResult.result,
      }
    } catch (error) {
      const errorMessage = `抱歉，执行过程中出现了问题: ${error instanceof Error ? error.message : String(error)}`
      await sendMessage('error', errorMessage)
      return { success: false, error: errorMessage }
    }
  }

  // ===== 回调系统访问方法 =====

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

  // ===== 开发和调试辅助方法（可选）=====

  /**
   * 开发者调试：查看Agent的思考过程
   * 生产环境建议移除或限制访问
   */
  async debugThinking(taskDescription: string) {
    if (process.env.NODE_ENV === 'production') {
      return { success: false, error: 'Debug mode not available in production' }
    }
    const availableTools = getBuiltinTools()
    return await this.planner.planTask(taskDescription, { includeThought: true, availableTools })
  }
}

/**
 * 便捷函数：创建Agent实例
 */
export function createAgent(config?: AgentFrameworkConfig): AgentFramework {
  return new AgentFramework(config)
}

// 主要导出
export default AgentFramework
