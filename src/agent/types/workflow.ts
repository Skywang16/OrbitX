/**
 * 工作流相关类型定义 - 简化版，参考eko设计但保持JSON格式
 */

export type WorkflowId = string
export type StepId = string

/**
 * 工作流步骤类型
 */
export enum StepType {
  LLM_CALL = 'llm_call',
  TOOL_CALL = 'tool_call',
  CONDITION = 'condition',
  LOOP = 'loop',
  FOR_EACH = 'for_each',
  PARALLEL = 'parallel',
  WAIT = 'wait',
  INPUT = 'input',
  OUTPUT = 'output',
  WATCH = 'watch',
}

/**
 * 工作流Agent定义 - 简化版，参考eko但使用JSON
 */
export interface WorkflowAgent {
  id: string
  name: string
  task: string
  type?: string
  dependsOn: string[] // 简化为字符串数组，像eko一样
  parallel?: boolean // 简单的并行标记
  status: 'init' | 'running' | 'done' | 'error'
  config?: Record<string, unknown> // 合并原来的steps配置
  toolCall?: {
    toolId: string
    parameters?: Record<string, unknown>
  }
}

/**
 * 工作流定义 - 简化版，参考eko设计
 */
export interface WorkflowDefinition {
  taskId: string // 改为taskId，与eko保持一致
  name: string
  thought?: string
  agents: WorkflowAgent[]
  variables?: Record<string, unknown>
  taskPrompt?: string // 原始任务提示
}

/**
 * 简化的执行结果 - 参考eko的EkoResult
 */
export interface ExecutionResult {
  taskId: string
  success: boolean
  stopReason: 'abort' | 'error' | 'done'
  result: string
  error?: unknown
}

/**
 * 简化的工作流执行状态
 */
export interface WorkflowExecution {
  taskId: string
  workflowId: string
  status: 'running' | 'completed' | 'failed' | 'paused'
  currentAgent?: string
  agentResults: Record<string, string> // agent_id -> result
  variables: Record<string, unknown>
  startTime: Date
  endTime?: Date
  error?: string
}

/**
 * Agent执行节点 - 参考eko的AgentNode设计
 */
export interface AgentNode {
  type: 'normal' | 'parallel'
  agent?: WorkflowAgent // normal类型时使用
  agents?: WorkflowAgent[] // parallel类型时使用
  nextAgent?: AgentNode
  result?: string
}

/**
 * 并行执行事件 - 简化版
 */
export interface ExecutionEvent {
  type: 'agent_start' | 'agent_completed' | 'agent_failed' | 'workflow_completed' | 'workflow_failed'
  agentId?: string
  timestamp: Date
  data?: unknown
  error?: string
}

// ===== 向后兼容的类型定义 =====

/**
 * 工作流步骤定义 - 保留用于向后兼容
 * @deprecated 建议直接使用WorkflowAgent的config字段
 */
export interface WorkflowStep {
  id: string
  type: StepType
  name: string
  description?: string
  dependsOn: string[] // 简化为字符串数组
  config: Record<string, unknown>
}

/**
 * 步骤执行状态 - 保留用于向后兼容
 * @deprecated 新的简化架构不需要复杂的步骤状态
 */
export enum StepExecutionStatus {
  PENDING = 'pending',
  READY = 'ready',
  RUNNING = 'running',
  COMPLETED = 'completed',
  FAILED = 'failed',
  SKIPPED = 'skipped',
  CANCELLED = 'cancelled',
}
