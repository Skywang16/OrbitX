/**
 * 错误类型定义
 */

/**
 * Agent错误类型
 */
export class AgentError extends Error {
  constructor(
    message: string,
    public code: string,
    public agentId?: string,
    public context?: Record<string, unknown>
  ) {
    super(message)
    this.name = 'AgentError'
  }
}

/**
 * 工具错误类型
 */
export class ToolError extends Error {
  constructor(
    message: string,
    public toolId: string,
    public code: string,
    public context?: Record<string, unknown>
  ) {
    super(message)
    this.name = 'ToolError'
  }
}

/**
 * 工作流错误类型
 */
export class WorkflowError extends Error {
  constructor(
    message: string,
    public workflowId: string,
    public code: string,
    public stepId?: string,
    public context?: Record<string, unknown>
  ) {
    super(message)
    this.name = 'WorkflowError'
  }
}

/**
 * 执行错误类型
 */
export class ExecutionError extends Error {
  constructor(
    message: string,
    public code: string,
    public executionId?: string,
    public context?: Record<string, unknown>
  ) {
    super(message)
    this.name = 'ExecutionError'
  }
}

/**
 * 配置错误类型
 */
export class ConfigError extends Error {
  constructor(
    message: string,
    public code: string,
    public configKey?: string,
    public context?: Record<string, unknown>
  ) {
    super(message)
    this.name = 'ConfigError'
  }
}
