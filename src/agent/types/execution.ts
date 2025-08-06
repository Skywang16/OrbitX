import type { WorkflowDefinition, WorkflowAgent } from './workflow'

/**
 * 简化的执行结果 - 基于上下文的纯净设计
 */
export interface ExecutionResult {
  taskId: string
  success: boolean
  stopReason: 'abort' | 'error' | 'done'
  result: string
  error?: unknown
}

/**
 * 执行事件
 */
export interface ExecutionEvent {
  type: 'workflow_start' | 'workflow_completed' | 'workflow_failed' | 'agent_start' | 'agent_completed' | 'agent_failed'
  timestamp: Date
  agentId?: string
  workflowId?: string
  data?: unknown
  error?: string
  metadata?: Record<string, unknown>
}

/**
 * Agent执行树节点
 */
export interface AgentNode {
  type: 'normal' | 'parallel'
  agent?: WorkflowAgent
  agents?: WorkflowAgent[]
  nextAgent?: AgentNode
  result?: string
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
