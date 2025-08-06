/**
 * Agent相关类型定义
 */

export type AgentId = string

/**
 * Agent状态枚举
 */
export enum AgentStatus {
  IDLE = 'idle',
  THINKING = 'thinking',
  EXECUTING = 'executing',
  WAITING = 'waiting',
  ERROR = 'error',
  STOPPED = 'stopped',
}

/**
 * Agent配置接口
 */
export interface AgentConfig {
  id: AgentId
  name: string
  description?: string
  model?: string
  systemPrompt?: string
  maxIterations?: number
  timeout?: number
  enabledTools?: string[]
  customConfig?: Record<string, unknown>
}

/**
 * Agent状态接口
 */
export interface AgentState {
  id: AgentId
  status: AgentStatus
  currentTask?: string
  currentWorkflow?: string
  currentStep?: string
  context: Record<string, unknown>
  history: AgentHistoryEntry[]
  createdAt: Date
  updatedAt: Date
}

/**
 * Agent历史记录条目
 */
export interface AgentHistoryEntry {
  timestamp: Date
  type: 'input' | 'output' | 'tool_call' | 'error' | 'workflow_start' | 'workflow_end'
  content: string
  metadata?: Record<string, unknown>
}

/**
 * Agent事件类型
 */
export enum AgentEventType {
  STATUS_CHANGED = 'status_changed',
  TASK_STARTED = 'task_started',
  TASK_COMPLETED = 'task_completed',
  TOOL_CALLED = 'tool_called',
  WORKFLOW_STARTED = 'workflow_started',
  WORKFLOW_COMPLETED = 'workflow_completed',
  ERROR_OCCURRED = 'error_occurred',
  TASK_ANALYZING = 'task_analyzing',
  TASK_PLANNING = 'task_planning',
  COMMAND_EXECUTING = 'command_executing',
  COMMAND_COMPLETED = 'command_completed',
  COMMAND_FAILED = 'command_failed',
  TASK_RETRYING = 'task_retrying',
  RESULT_ANALYZING = 'result_analyzing',
  TASK_PROGRESS = 'task_progress',
}

/**
 * Agent事件
 */
export interface AgentEvent {
  type: AgentEventType
  agentId: AgentId
  timestamp: Date
  data: Record<string, unknown>
}

/**
 * 事件监听器
 */
export type AgentEventListener = (event: AgentEvent) => void | Promise<void>

/**
 * 具体的事件数据类型
 */
export interface TaskAnalyzingEventData {
  userInput: string
  intent?: string
}

export interface TaskPlanningEventData {
  plan: string
  steps?: string[]
}

export interface CommandExecutingEventData {
  command: string
  explanation?: string
  attemptCount?: number
}

export interface CommandCompletedEventData {
  command: string
  output: string
  exitCode: number
  duration: number
}

export interface CommandFailedEventData {
  command: string
  error: string
  exitCode?: number
  stderr?: string
}

export interface TaskRetryingEventData {
  reason: string
  nextCommand: string
  attemptCount: number
  maxAttempts?: number
}

export interface ResultAnalyzingEventData {
  command: string
  output: string
  analysis?: string
}

export interface TaskProgressEventData {
  progress: number
  currentStep: string
  totalSteps?: number
  message?: string
}
