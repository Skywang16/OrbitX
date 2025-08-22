/**
 * 可修改工具基础类
 */

import type { AgentContext } from '@eko-ai/eko'
import type { Tool, ToolResult } from '@eko-ai/eko/types'

import { ToolError, formatToolError } from './tool-error'

export interface ToolParameters {
  type: 'object'
  properties: Record<string, unknown>
  required?: string[]
}

export interface ToolExecutionContext {
  agentContext: AgentContext
  parameters: Record<string, unknown>
  toolName: string
}

/**
 * 可修改工具抽象基类
 */
export abstract class ModifiableTool implements Tool {
  public readonly name: string
  public readonly description: string
  public readonly parameters: ToolParameters

  constructor(name: string, description: string, parameters: ToolParameters) {
    this.name = name
    this.description = description
    this.parameters = parameters
  }

  /**
   * 验证参数
   */
  protected validateParameters(params: Record<string, unknown>): void {
    if (!this.parameters.required) return

    for (const required of this.parameters.required) {
      if (!(required in params) || params[required] === undefined || params[required] === null) {
        throw new ToolError(`缺少必需参数: ${required}`, 'MISSING_PARAMETER')
      }
    }
  }

  /**
   * 执行前的钩子
   */
  protected async beforeExecute(_context: ToolExecutionContext): Promise<void> {
    // 默认空实现，子类可覆盖
  }

  /**
   * 执行后的钩子
   */
  protected async afterExecute(_context: ToolExecutionContext, _result: ToolResult): Promise<void> {
    // 默认空实现，子类可覆盖
  }

  /**
   * 错误处理钩子
   */
  protected async onError(_context: ToolExecutionContext, error: unknown): Promise<ToolResult> {
    // 对于ToolError类型的错误，重新抛出以便测试可以捕获
    if (error instanceof ToolError) {
      throw error
    }

    return {
      content: [
        {
          type: 'text',
          text: formatToolError(error),
        },
      ],
    }
  }

  /**
   * 具体的执行逻辑，由子类实现
   */
  protected abstract executeImpl(context: ToolExecutionContext): Promise<ToolResult>

  /**
   * 工具执行入口
   */
  public async execute(params: unknown, agentContext: AgentContext): Promise<ToolResult> {
    try {
      const parameters = params as Record<string, unknown>

      // 验证参数
      this.validateParameters(parameters)

      const context: ToolExecutionContext = {
        agentContext,
        parameters,
        toolName: this.name,
      }

      // 执行前钩子
      await this.beforeExecute(context)

      // 执行具体逻辑
      const result = await this.executeImpl(context)

      // 执行后钩子
      await this.afterExecute(context, result)

      return result
    } catch (error) {
      const context: ToolExecutionContext = {
        agentContext,
        parameters: params as Record<string, unknown>,
        toolName: this.name,
      }

      return this.onError(context, error)
    }
  }
}

/**
 * 简单工具辅助函数
 */
export function createSimpleTool(
  name: string,
  description: string,
  parameters: ToolParameters,
  executeFunc: (context: ToolExecutionContext) => Promise<ToolResult>
): Tool {
  return new (class extends ModifiableTool {
    protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
      return executeFunc(context)
    }
  })(name, description, parameters)
}
