/**
 * @file AgentContext.ts
 * @description Agent级别的上下文管理
 */

import { TaskContext } from './TaskContext'
import { WorkflowAgent } from '../types/workflow'
import { ChatMessage, ChatMessageRole } from '../types/memory'

/**
 * Agent执行状态
 */
export enum AgentExecutionStatus {
  IDLE = 'idle',
  THINKING = 'thinking',
  EXECUTING = 'executing',
  WAITING = 'waiting',
  ERROR = 'error',
  COMPLETED = 'completed',
}

/**
 * Agent级别的上下文
 * 提供Agent级别的变量管理、状态跟踪和错误处理
 */
export class AgentContext {
  public readonly agent: WorkflowAgent
  public readonly taskContext: TaskContext
  public readonly variables: Map<string, unknown> = new Map()
  public consecutiveErrorCount: number = 0
  public status: AgentExecutionStatus = AgentExecutionStatus.IDLE
  public startTime?: Date
  public endTime?: Date
  public lastError?: Error
  public executionHistory: AgentExecutionRecord[] = []

  constructor(agent: WorkflowAgent, taskContext: TaskContext) {
    this.agent = agent
    this.taskContext = taskContext
  }

  /**
   * 设置Agent级别的变量
   */
  public setVariable(key: string, value: unknown): void {
    this.variables.set(key, value)
  }

  /**
   * 获取变量（优先级：Agent级别 > Task级别）
   */
  public getVariable(key: string): unknown {
    return this.variables.has(key) ? this.variables.get(key) : this.taskContext.getVariable(key)
  }

  /**
   * 获取所有可用变量
   */
  public getAllVariables(): Record<string, unknown> {
    const taskVars = Object.fromEntries(this.taskContext.variables.entries())
    const agentVars = Object.fromEntries(this.variables.entries())

    // Agent变量覆盖Task变量
    return { ...taskVars, ...agentVars }
  }

  /**
   * 更新Agent状态
   */
  public updateStatus(status: AgentExecutionStatus): void {
    const previousStatus = this.status
    this.status = status

    // 记录状态变化
    if (status === AgentExecutionStatus.EXECUTING && !this.startTime) {
      this.startTime = new Date()
    }

    if (status === AgentExecutionStatus.COMPLETED || status === AgentExecutionStatus.ERROR) {
      this.endTime = new Date()
    }

    // 添加执行记录
    this.executionHistory.push({
      timestamp: new Date(),
      previousStatus,
      newStatus: status,
      duration: this.startTime ? Date.now() - this.startTime.getTime() : 0,
    })
  }

  /**
   * 记录错误
   */
  public recordError(error: Error): void {
    this.lastError = error
    this.consecutiveErrorCount++
    this.updateStatus(AgentExecutionStatus.ERROR)
  }

  /**
   * 重置错误计数
   */
  public resetErrorCount(): void {
    this.consecutiveErrorCount = 0
    this.lastError = undefined
  }

  /**
   * 检查是否应该停止执行（基于连续错误次数）
   */
  public shouldStopExecution(maxErrors: number = 3): boolean {
    return this.consecutiveErrorCount >= maxErrors
  }

  /**
   * 添加Agent消息到任务上下文的记忆中
   */
  public async addMessage(role: ChatMessageRole, content: string): Promise<void> {
    const message: ChatMessage = {
      role,
      content: `[${this.agent.name}] ${content}`,
    }

    await this.taskContext.memory.addChatMessage(message)
  }

  /**
   * 获取Agent的执行统计信息
   */
  public getExecutionStats(): AgentExecutionStats {
    const duration =
      this.startTime && this.endTime
        ? this.endTime.getTime() - this.startTime.getTime()
        : this.startTime
          ? Date.now() - this.startTime.getTime()
          : 0

    return {
      agentId: this.agent.id,
      agentName: this.agent.name,
      status: this.status,
      duration,
      consecutiveErrorCount: this.consecutiveErrorCount,
      totalExecutions: this.executionHistory.length,
      lastError: this.lastError?.message,
      variableCount: this.variables.size,
    }
  }

  /**
   * 创建Agent的快照
   */
  public createSnapshot(): AgentSnapshot {
    return {
      agentId: this.agent.id,
      agentName: this.agent.name,
      status: this.status,
      variables: Object.fromEntries(this.variables.entries()),
      consecutiveErrorCount: this.consecutiveErrorCount,
      executionHistory: [...this.executionHistory],
      lastError: this.lastError?.message,
      timestamp: new Date().toISOString(),
    }
  }

  /**
   * 从快照恢复Agent状态
   */
  public restoreFromSnapshot(snapshot: AgentSnapshot): void {
    this.status = snapshot.status
    this.consecutiveErrorCount = snapshot.consecutiveErrorCount
    this.executionHistory = [...snapshot.executionHistory]

    // 恢复变量
    this.variables.clear()
    for (const [key, value] of Object.entries(snapshot.variables)) {
      this.variables.set(key, value)
    }

    // 恢复错误信息
    if (snapshot.lastError) {
      this.lastError = new Error(snapshot.lastError)
    }
  }
}

/**
 * Agent执行记录
 */
export interface AgentExecutionRecord {
  timestamp: Date
  previousStatus: AgentExecutionStatus
  newStatus: AgentExecutionStatus
  duration: number
}

/**
 * Agent执行统计信息
 */
export interface AgentExecutionStats {
  agentId: string
  agentName: string
  status: AgentExecutionStatus
  duration: number
  consecutiveErrorCount: number
  totalExecutions: number
  lastError?: string
  variableCount: number
}

/**
 * Agent快照
 */
export interface AgentSnapshot {
  agentId: string
  agentName: string
  status: AgentExecutionStatus
  variables: Record<string, unknown>
  consecutiveErrorCount: number
  executionHistory: AgentExecutionRecord[]
  lastError?: string
  timestamp: string
}
