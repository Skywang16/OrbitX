/**
 * 工具系统类型定义
 */

export type ToolId = string

/**
 * 工具参数定义
 */
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

/**
 * 工具定义接口
 */
export interface ToolDefinition {
  id: ToolId
  name: string
  description: string
  parameters: ToolParameter[]
  category?: string
  version?: string
  author?: string
  tags?: string[]
}

/**
 * 工具执行结果 - 统一的Result模式
 */
export interface ToolResult {
  success: boolean
  data?: unknown
  error?: string
  metadata?: {
    executionTime?: number
    toolVersion?: string
    [key: string]: unknown
  }
}

/**
 * 工具执行上下文
 */
export interface ToolContext {
  agentId: string
  workflowId?: string
  stepId?: string
  parameters: Record<string, unknown>
  agentContext: Record<string, unknown>
}

/**
 * 工具接口
 */
export interface Tool {
  definition: ToolDefinition
  execute(context: ToolContext): Promise<ToolResult>
}

/**
 * 工具执行统计
 */
export interface ToolExecutionStats {
  toolId: ToolId
  executionCount: number
  totalExecutionTime: number
  averageExecutionTime: number
  successCount: number
  errorCount: number
  lastExecuted?: Date
  lastError?: string
}

/**
 * 工具权限配置
 */
export interface ToolPermission {
  toolId: ToolId
  allowedAgents?: string[]
  deniedAgents?: string[]
  maxExecutionsPerMinute?: number
  requiresConfirmation?: boolean
}
