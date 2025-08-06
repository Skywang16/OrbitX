/**
 * 工作流相关类型定义
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
 * 工作流Agent定义
 */
export interface WorkflowAgent {
  id: string
  name: string
  task: string
  type?: string
  dependsOn: string[] // 简化为字符串数组
  parallel?: boolean // 简单的并行标记
  status: 'init' | 'running' | 'done' | 'error'
  config?: Record<string, unknown> // 合并原来的steps配置
  toolCall?: {
    toolId: string
    parameters?: Record<string, unknown>
  }
}

/**
 * 工作流定义
 */
export interface WorkflowDefinition {
  taskId: string // 改为taskId
  name: string
  thought?: string
  agents: WorkflowAgent[]
  variables?: Record<string, unknown>
  taskPrompt?: string // 原始任务提示
}

/**
 * 简化的执行结果
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
 * Agent执行节点
 */
export interface AgentNode {
  type: 'normal' | 'parallel'
  agent?: WorkflowAgent // normal类型时使用
  agents?: WorkflowAgent[] // parallel类型时使用
  nextAgent?: AgentNode
  result?: string
}
