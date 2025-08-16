/**
 * Eko框架原生工具基类 - 完全符合官方规范
 */

import type { AgentContext } from '@eko-ai/eko'
import type { Tool, ToolResult, LanguageModelV2ToolCallPart } from '@eko-ai/eko/types'
import type { JSONSchema7 } from 'json-schema'

/**
 * JSON Schema构建器 - 用于构建工具参数
 */
export class ParameterSchema {
  private schema: JSONSchema7 = {
    type: 'object',
    properties: {},
    required: [],
  }

  string(name: string, description: string, required = false, defaultValue?: string): this {
    if (!this.schema.properties) this.schema.properties = {}
    this.schema.properties[name] = {
      type: 'string',
      description,
      ...(defaultValue !== undefined && { default: defaultValue }),
    }
    if (required && this.schema.required) this.schema.required.push(name)
    return this
  }

  number(name: string, description: string, required = false, min?: number, max?: number, defaultValue?: number): this {
    if (!this.schema.properties) this.schema.properties = {}
    this.schema.properties[name] = {
      type: 'number',
      description,
      ...(min !== undefined && { minimum: min }),
      ...(max !== undefined && { maximum: max }),
      ...(defaultValue !== undefined && { default: defaultValue }),
    }
    if (required && this.schema.required) this.schema.required.push(name)
    return this
  }

  boolean(name: string, description: string, required = false, defaultValue?: boolean): this {
    if (!this.schema.properties) this.schema.properties = {}
    this.schema.properties[name] = {
      type: 'boolean',
      description,
      ...(defaultValue !== undefined && { default: defaultValue }),
    }
    if (required && this.schema.required) this.schema.required.push(name)
    return this
  }

  array(
    name: string,
    description: string,
    itemType: 'string' | 'number' | 'boolean' | 'object',
    required = false
  ): this {
    if (!this.schema.properties) this.schema.properties = {}
    this.schema.properties[name] = {
      type: 'array',
      description,
      items: { type: itemType },
    }
    if (required && this.schema.required) this.schema.required.push(name)
    return this
  }

  object(name: string, description: string, properties: Record<string, JSONSchema7>, required = false): this {
    if (!this.schema.properties) this.schema.properties = {}
    this.schema.properties[name] = {
      type: 'object',
      description,
      properties,
    }
    if (required && this.schema.required) this.schema.required.push(name)
    return this
  }

  build(): JSONSchema7 {
    return { ...this.schema }
  }
}

/**
 * Eko原生工具基类
 */
export abstract class EkoTool implements Tool {
  public readonly name: string
  public readonly description: string
  public readonly parameters: JSONSchema7

  constructor(config: { name: string; description: string; parameters: JSONSchema7 }) {
    this.name = config.name
    this.description = config.description
    this.parameters = config.parameters
  }

  /**
   * 执行工具 - 符合Eko规范
   */
  async execute(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: LanguageModelV2ToolCallPart
  ): Promise<ToolResult> {
    try {
      // 验证参数
      const validationResult = this.validateParameters(args)
      if (!validationResult.valid) {
        return this.error(`参数验证失败: ${validationResult.errors.join(', ')}`)
      }

      // 执行具体逻辑
      const result = await this.executeImpl(validationResult.validated, agentContext, toolCall)

      return result
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return this.error(errorMessage)
    }
  }

  /**
   * 具体执行逻辑，由子类实现
   */
  protected abstract executeImpl(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: LanguageModelV2ToolCallPart
  ): Promise<ToolResult>

  /**
   * 验证参数
   */
  private validateParameters(
    args: Record<string, unknown>
  ): { valid: true; validated: Record<string, unknown> } | { valid: false; errors: string[] } {
    const errors: string[] = []
    const validated: Record<string, unknown> = {}

    // 检查必需参数
    if (this.parameters.required) {
      for (const requiredParam of this.parameters.required) {
        if (!(requiredParam in args) || args[requiredParam] === undefined || args[requiredParam] === null) {
          errors.push(`缺少必需参数: ${requiredParam}`)
        }
      }
    }

    // 验证参数类型和设置默认值
    if (this.parameters.properties) {
      for (const [paramName, paramSchema] of Object.entries(
        this.parameters.properties as Record<string, JSONSchema7>
      )) {
        const value = args[paramName]

        if (value === undefined) {
          if (paramSchema.default !== undefined) {
            validated[paramName] = paramSchema.default
          }
          continue
        }

        // 类型验证
        const schemaType = Array.isArray(paramSchema.type) ? paramSchema.type[0] : paramSchema.type
        if (schemaType && !this.validateParameterType(value, schemaType)) {
          errors.push(`参数 ${paramName} 类型错误，期望 ${schemaType}`)
          continue
        }

        // 数值范围验证
        if (schemaType === 'number' && typeof value === 'number') {
          if (paramSchema.minimum !== undefined && value < paramSchema.minimum) {
            errors.push(`参数 ${paramName} 不能小于 ${paramSchema.minimum}`)
            continue
          }
          if (paramSchema.maximum !== undefined && value > paramSchema.maximum) {
            errors.push(`参数 ${paramName} 不能大于 ${paramSchema.maximum}`)
            continue
          }
        }

        validated[paramName] = value
      }
    }

    if (errors.length > 0) {
      return { valid: false, errors }
    }

    return { valid: true, validated }
  }

  /**
   * 验证参数类型
   */
  private validateParameterType(value: unknown, expectedType: string): boolean {
    switch (expectedType) {
      case 'string':
        return typeof value === 'string'
      case 'number':
        return typeof value === 'number' && !isNaN(value)
      case 'integer':
        return typeof value === 'number' && Number.isInteger(value)
      case 'boolean':
        return typeof value === 'boolean'
      case 'array':
        return Array.isArray(value)
      case 'object':
        return typeof value === 'object' && value !== null && !Array.isArray(value)
      default:
        return true
    }
  }

  /**
   * 发送tool_running回调
   */
  protected async sendCallback(
    agentContext: AgentContext,
    toolCall: LanguageModelV2ToolCallPart,
    message: string,
    streamDone: boolean
  ): Promise<void> {
    try {
      const callback = agentContext.context?.config?.callback
      if (!callback?.onMessage) return

      await callback.onMessage({
        taskId: agentContext.context.taskId,
        agentName: agentContext.agent.Name,
        nodeId: agentContext.agentChain?.agent?.id || 'unknown',
        type: 'tool_running',
        toolName: toolCall.toolName,
        toolId: toolCall.toolCallId,
        text: message,
        streamId: toolCall.toolCallId,
        streamDone,
      })
    } catch (error) {
      // 静默处理回调错误，避免影响工具执行
    }
  }

  /**
   * 创建成功结果
   */
  protected success(text: string): ToolResult {
    return {
      content: [
        {
          type: 'text',
          text: `✅ ${text}`,
        },
      ],
    }
  }

  /**
   * 创建错误结果
   */
  protected error(message: string): ToolResult {
    return {
      content: [
        {
          type: 'text',
          text: `❌ ${message}`,
        },
      ],
    }
  }

  /**
   * 创建文本结果
   */
  protected text(content: string): ToolResult {
    return {
      content: [
        {
          type: 'text',
          text: content,
        },
      ],
    }
  }

  /**
   * 访问代理变量
   */
  protected getVariable(agentContext: AgentContext, key: string): unknown {
    return agentContext.variables.get(key)
  }

  /**
   * 设置代理变量
   */
  protected setVariable(agentContext: AgentContext, key: string, value: unknown): void {
    agentContext.variables.set(key, value)
  }

  /**
   * 访问全局变量
   */
  protected getGlobalVariable(agentContext: AgentContext, key: string): unknown {
    return agentContext.context?.variables?.get(key)
  }

  /**
   * 设置全局变量
   */
  protected setGlobalVariable(agentContext: AgentContext, key: string, value: unknown): void {
    agentContext.context?.variables?.set(key, value)
  }
}

/**
 * 创建参数Schema的辅助函数
 */
export function createParameterSchema(): ParameterSchema {
  return new ParameterSchema()
}

/**
 * 创建简单工具的辅助函数
 */
export function createTool(
  config: {
    name: string
    description: string
    parameters: JSONSchema7
  },
  executeFunc: (
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: LanguageModelV2ToolCallPart
  ) => Promise<ToolResult>
): EkoTool {
  return new (class extends EkoTool {
    protected async executeImpl(
      args: Record<string, unknown>,
      agentContext: AgentContext,
      toolCall: LanguageModelV2ToolCallPart
    ): Promise<ToolResult> {
      return executeFunc(args, agentContext, toolCall)
    }
  })(config)
}
