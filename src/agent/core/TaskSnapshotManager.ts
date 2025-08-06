/**
 * @file TaskSnapshotManager.ts
 * @description 任务快照管理器
 * 提供任务状态的保存、恢复和压缩功能
 */

import { TaskContext } from '../context/TaskContext'
import { AgentContext, AgentSnapshot } from '../context/AgentContext'
import { WorkflowDefinition } from '../types/workflow'
import { Memory, ChatMessageRole } from '../types/memory'

/**
 * 任务快照数据结构
 */
export interface TaskSnapshot {
  taskId: string
  workflowId: string
  name: string
  timestamp: string
  version: string

  // 任务基本信息
  workflow: WorkflowDefinition
  variables: Record<string, unknown>

  // 记忆快照
  memory: Memory

  // Agent状态快照
  agentSnapshots: AgentSnapshot[]

  // 执行状态
  executionState: {
    currentAgent?: string
    completedAgents: string[]
    failedAgents: string[]
    startTime: string
    lastUpdateTime: string
  }

  // 元数据
  metadata: {
    snapshotReason: 'manual' | 'auto' | 'error' | 'compression'
    totalTokens: number
    messageCount: number
    agentCount: number
    errorCount: number
  }
}

/**
 * 快照配置
 */
export interface SnapshotConfig {
  enableAutoSnapshot: boolean
  autoSnapshotInterval: number // 自动快照间隔（毫秒）
  maxSnapshots: number // 最大保存快照数量
  compressionEnabled: boolean
  compressionThreshold: number // 触发压缩的快照数量
}

const DEFAULT_SNAPSHOT_CONFIG: SnapshotConfig = {
  enableAutoSnapshot: true,
  autoSnapshotInterval: 300000, // 5分钟
  maxSnapshots: 10,
  compressionEnabled: true,
  compressionThreshold: 5,
}

/**
 * 任务快照管理器
 */
export class TaskSnapshotManager {
  private snapshots: Map<string, TaskSnapshot[]> = new Map() // taskId -> snapshots
  private config: SnapshotConfig
  private autoSnapshotTimers: Map<string, NodeJS.Timeout> = new Map()

  constructor(config: Partial<SnapshotConfig> = {}) {
    this.config = { ...DEFAULT_SNAPSHOT_CONFIG, ...config }
  }

  /**
   * 创建任务快照
   */
  public async createSnapshot(
    taskContext: TaskContext,
    agentContexts: AgentContext[] = [],
    reason: TaskSnapshot['metadata']['snapshotReason'] = 'manual'
  ): Promise<TaskSnapshot> {
    const now = new Date()
    const memory = taskContext.memory.getMemory()

    // 计算统计信息
    const totalTokens = this.calculateTotalTokens(memory)
    const errorCount = agentContexts.reduce((count, ctx) => count + ctx.consecutiveErrorCount, 0)

    const snapshot: TaskSnapshot = {
      taskId: taskContext.taskId,
      workflowId: taskContext.workflow?.taskId || taskContext.taskId,
      name: taskContext.workflow?.name || 'Unknown Task',
      timestamp: now.toISOString(),
      version: '1.0.0',

      workflow: taskContext.workflow!,
      variables: Object.fromEntries(taskContext.variables.entries()),
      memory,

      agentSnapshots: agentContexts.map(ctx => ctx.createSnapshot()),

      executionState: {
        currentAgent: this.getCurrentAgent(agentContexts),
        completedAgents: this.getCompletedAgents(agentContexts),
        failedAgents: this.getFailedAgents(agentContexts),
        startTime: taskContext.workflow?.taskPrompt ? now.toISOString() : now.toISOString(),
        lastUpdateTime: now.toISOString(),
      },

      metadata: {
        snapshotReason: reason,
        totalTokens,
        messageCount: memory.chatHistory.length,
        agentCount: agentContexts.length,
        errorCount,
      },
    }

    // 保存快照
    await this.saveSnapshot(snapshot)

    return snapshot
  }

  /**
   * 保存快照
   */
  private async saveSnapshot(snapshot: TaskSnapshot): Promise<void> {
    const taskSnapshots = this.snapshots.get(snapshot.taskId) || []
    taskSnapshots.push(snapshot)

    // 限制快照数量
    if (taskSnapshots.length > this.config.maxSnapshots) {
      taskSnapshots.splice(0, taskSnapshots.length - this.config.maxSnapshots)
    }

    this.snapshots.set(snapshot.taskId, taskSnapshots)

    // 检查是否需要压缩
    if (this.config.compressionEnabled && taskSnapshots.length >= this.config.compressionThreshold) {
      await this.compressSnapshots(snapshot.taskId)
    }
  }

  /**
   * 从快照恢复任务
   */
  public async restoreFromSnapshot(
    snapshotId: string,
    taskId: string
  ): Promise<{ taskContext: TaskContext; agentContexts: AgentContext[] } | null> {
    const taskSnapshots = this.snapshots.get(taskId)
    if (!taskSnapshots) return null

    const snapshot = taskSnapshots.find(s => s.timestamp === snapshotId)
    if (!snapshot) return null

    // 恢复TaskContext
    const taskContext = new TaskContext(
      snapshot.taskId,
      {}, // 空配置
      snapshot.workflow,
      snapshot.variables
    )

    // 恢复记忆
    await this.restoreMemory(taskContext, snapshot.memory)

    // 恢复AgentContexts
    const agentContexts: AgentContext[] = []
    for (const agentSnapshot of snapshot.agentSnapshots) {
      const agent = snapshot.workflow.agents.find(a => a.id === agentSnapshot.agentId)
      if (agent) {
        const agentContext = new AgentContext(agent, taskContext)
        agentContext.restoreFromSnapshot(agentSnapshot)
        agentContexts.push(agentContext)
      }
    }

    return { taskContext, agentContexts }
  }

  /**
   * 获取任务的所有快照
   */
  public getSnapshots(taskId: string): TaskSnapshot[] {
    return this.snapshots.get(taskId) || []
  }

  /**
   * 获取最新快照
   */
  public getLatestSnapshot(taskId: string): TaskSnapshot | null {
    const taskSnapshots = this.snapshots.get(taskId)
    return taskSnapshots && taskSnapshots.length > 0 ? taskSnapshots[taskSnapshots.length - 1] : null
  }

  /**
   * 删除快照
   */
  public deleteSnapshot(taskId: string, snapshotId: string): boolean {
    const taskSnapshots = this.snapshots.get(taskId)
    if (!taskSnapshots) return false

    const index = taskSnapshots.findIndex(s => s.timestamp === snapshotId)
    if (index === -1) return false

    taskSnapshots.splice(index, 1)
    return true
  }

  /**
   * 清理任务的所有快照
   */
  public clearTaskSnapshots(taskId: string): void {
    this.snapshots.delete(taskId)

    // 清理自动快照定时器
    const timer = this.autoSnapshotTimers.get(taskId)
    if (timer) {
      clearInterval(timer)
      this.autoSnapshotTimers.delete(taskId)
    }
  }

  /**
   * 启动自动快照
   */
  public startAutoSnapshot(taskContext: TaskContext, agentContexts: AgentContext[]): void {
    if (!this.config.enableAutoSnapshot) return

    const timer = setInterval(async () => {
      try {
        await this.createSnapshot(taskContext, agentContexts, 'auto')
      } catch (error) {
        // 自动快照失败时静默处理
      }
    }, this.config.autoSnapshotInterval)

    this.autoSnapshotTimers.set(taskContext.taskId, timer)
  }

  /**
   * 停止自动快照
   */
  public stopAutoSnapshot(taskId: string): void {
    const timer = this.autoSnapshotTimers.get(taskId)
    if (timer) {
      clearInterval(timer)
      this.autoSnapshotTimers.delete(taskId)
    }
  }

  /**
   * 压缩快照
   */
  private async compressSnapshots(taskId: string): Promise<void> {
    const taskSnapshots = this.snapshots.get(taskId)
    if (!taskSnapshots || taskSnapshots.length < this.config.compressionThreshold) return

    // 保留最新的几个快照，压缩较旧的快照
    const keepCount = Math.floor(this.config.compressionThreshold / 2)
    const toCompress = taskSnapshots.slice(0, -keepCount)
    const toKeep = taskSnapshots.slice(-keepCount)

    // 创建压缩快照
    if (toCompress.length > 0) {
      const compressedSnapshot = await this.createCompressedSnapshot(toCompress)
      this.snapshots.set(taskId, [compressedSnapshot, ...toKeep])
    }
  }

  /**
   * 创建压缩快照
   */
  private async createCompressedSnapshot(snapshots: TaskSnapshot[]): Promise<TaskSnapshot> {
    const latest = snapshots[snapshots.length - 1]
    const earliest = snapshots[0]

    return {
      ...latest,
      timestamp: `${earliest.timestamp}_to_${latest.timestamp}`,
      metadata: {
        ...latest.metadata,
        snapshotReason: 'compression',
        messageCount: snapshots.reduce((sum, s) => sum + s.metadata.messageCount, 0),
        totalTokens: snapshots.reduce((sum, s) => sum + s.metadata.totalTokens, 0),
      },
      // 压缩记忆 - 只保留关键信息
      memory: {
        chatHistory: [
          ...earliest.memory.chatHistory.slice(0, 2), // 保留开始的消息
          {
            role: ChatMessageRole.SYSTEM,
            content: `[压缩快照] 包含${snapshots.length}个快照的历史记录，时间范围：${earliest.timestamp} 到 ${latest.timestamp}`,
          },
          ...latest.memory.chatHistory.slice(-2), // 保留最新的消息
        ],
        workingMemory: latest.memory.workingMemory,
      },
    }
  }

  // 辅助方法
  private calculateTotalTokens(memory: Memory): number {
    return memory.chatHistory.reduce((total, message) => {
      const chineseCharCount = (message.content.match(/[\u4e00-\u9fff]/g) || []).length
      const otherCharCount = message.content.length - chineseCharCount
      return total + chineseCharCount + Math.ceil(otherCharCount / 4)
    }, 0)
  }

  private getCurrentAgent(agentContexts: AgentContext[]): string | undefined {
    return agentContexts.find(ctx => ctx.status === 'executing')?.agent.id
  }

  private getCompletedAgents(agentContexts: AgentContext[]): string[] {
    return agentContexts.filter(ctx => ctx.status === 'completed').map(ctx => ctx.agent.id)
  }

  private getFailedAgents(agentContexts: AgentContext[]): string[] {
    return agentContexts.filter(ctx => ctx.status === 'error').map(ctx => ctx.agent.id)
  }

  private async restoreMemory(taskContext: TaskContext, memory: Memory): Promise<void> {
    // 恢复聊天历史
    for (const message of memory.chatHistory) {
      await taskContext.memory.addChatMessage(message)
    }

    // 恢复工作记忆
    for (const [key, value] of Object.entries(memory.workingMemory)) {
      taskContext.memory.setWorkingMemory(key, value)
    }
  }
}
