import type { WorkflowAgent, WorkflowDefinition } from './workflow'

/**
 * Agent执行上下文
 *
 * 在Agent执行期间传递的上下文信息
 */
export interface AgentExecutionContext {
  agentId: string
  workflowId: string
  sessionId?: string
  variables: Record<string, unknown>
  stepResults: Record<string, unknown>
  metadata?: Record<string, unknown>
}

/**
 * Agent执行结果
 */
export interface AgentResult {
  success: boolean
  data?: unknown
  error?: string
  agentId: string
  executionTime: number
  metadata?: Record<string, unknown>
}

/**
 * 工作流执行状态
 */
export interface WorkflowExecution {
  taskId: string
  workflowId: string
  status: 'running' | 'completed' | 'failed'
  agentResults: Record<string, unknown>
  variables: Record<string, unknown>
  startTime: Date
  endTime?: Date
  currentAgent?: string
  error?: string
}

/**
 * 最终执行结果
 */
export interface ExecutionResult {
  taskId: string
  success: boolean
  stopReason: 'done' | 'error' | 'max_iterations'
  result?: unknown
  error?: unknown
}

/**
 * 执行事件
 */
export interface ExecutionEvent {
  type: 'workflow_start' | 'workflow_completed' | 'workflow_failed' | 'agent_start' | 'agent_completed' | 'agent_failed'
  timestamp: Date
  agentId?: string
  data?: unknown
  error?: string
}

/**
 * Agent执行树节点
 */
export interface AgentNode {
  type: 'normal' | 'parallel'
  agent?: WorkflowAgent
  agents?: WorkflowAgent[]
  nextAgent?: AgentNode
  result?: any
}

/**
 * 执行引擎接口
 */
export interface IExecutionEngine {
  execute(
    workflow: WorkflowDefinition,
    contextParams?: Record<string, unknown>,
    callback?: (event: ExecutionEvent) => Promise<void>
  ): Promise<ExecutionResult>
}
