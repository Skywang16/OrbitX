/**
 * Memory管理器
 *
 * 参考Eko的Memory机制，实现上下文压缩和工具使用历史提取
 * 用于优化动态重规划的性能
 */

import type { WorkflowStep } from '../types'
import type { StepExecutionResult } from '../types/execution'

/**
 * 执行历史记录
 */
export interface ExecutionHistory {
  stepId: string
  toolId: string
  parameters: Record<string, unknown>
  result: StepExecutionResult
  timestamp: number
  duration: number
  success: boolean
}

/**
 * 压缩后的上下文
 */
export interface CompressedContext {
  essentialSteps: ExecutionHistory[]
  usedTools: string[]
  keyResults: Record<string, unknown>
  terminalState: {
    currentDirectory?: string
    lastOutput?: string
    activeProcesses?: string[]
  }
  summary: string
}

/**
 * Memory管理器
 */
export class MemoryManager {
  private executionHistory: ExecutionHistory[] = []
  private maxHistorySize = 100
  private compressionThreshold = 50

  /**
   * 记录步骤执行
   */
  recordExecution(step: WorkflowStep, result: StepExecutionResult, duration: number): void {
    const record: ExecutionHistory = {
      stepId: step.id,
      toolId: (step.config.toolId as string) || 'unknown',
      parameters: step.config.parameters || {},
      result,
      timestamp: Date.now(),
      duration,
      success: result.success,
    }

    this.executionHistory.push(record)

    // 限制历史记录大小
    if (this.executionHistory.length > this.maxHistorySize) {
      this.executionHistory = this.executionHistory.slice(-this.maxHistorySize)
    }
  }

  /**
   * 提取实际使用的工具
   */
  extractUsedTools(): string[] {
    return [...new Set(this.executionHistory.filter(r => r.success).map(r => r.toolId))]
  }

  /**
   * 提取关键执行结果
   */
  extractKeyResults(): Record<string, unknown> {
    const keyResults: Record<string, unknown> = {}
    const recentSuccessful = this.executionHistory.filter(r => r.success).slice(-10)

    for (const record of recentSuccessful) {
      if (record.result.result) {
        keyResults[`${record.toolId}_${record.stepId}`] = {
          data: record.result.result,
          timestamp: record.timestamp,
        }
      }
    }

    return keyResults
  }

  /**
   * 压缩执行上下文
   */
  compressContext(): CompressedContext {
    const essentialSteps = this.executionHistory.filter(r => r.success).slice(-20)
    return {
      essentialSteps,
      usedTools: this.extractUsedTools(),
      keyResults: this.extractKeyResults(),
      terminalState: this.extractTerminalState(),
      summary: this.generateExecutionSummary(essentialSteps),
    }
  }

  /**
   * 提取终端状态
   */
  private extractTerminalState(): CompressedContext['terminalState'] {
    const terminalRecords = this.executionHistory
      .filter(record => record.toolId === 'terminal_execute' && record.success)
      .slice(-5) // 最近5个终端命令

    let currentDirectory: string | undefined
    let lastOutput: string | undefined
    const activeProcesses: string[] = []

    for (const record of terminalRecords) {
      // 提取工作目录变化
      if (record.parameters.command?.includes('cd ')) {
        const cdMatch = record.parameters.command.match(/cd\s+(.+)/)
        if (cdMatch) {
          currentDirectory = cdMatch[1].trim()
        }
      }

      // 提取最后的输出
      if (record.result.output) {
        lastOutput = record.result.output
      }

      // 检测后台进程
      if (record.parameters.command?.includes('&') || record.parameters.command?.includes('nohup')) {
        activeProcesses.push(record.parameters.command)
      }
    }

    return {
      currentDirectory,
      lastOutput,
      activeProcesses: activeProcesses.length > 0 ? activeProcesses : undefined,
    }
  }

  /**
   * 生成执行摘要
   */
  private generateExecutionSummary(steps: ExecutionHistory[]): string {
    if (steps.length === 0) {
      return '暂无执行历史'
    }

    const toolCounts = new Map<string, number>()
    let totalDuration = 0
    let successCount = 0

    for (const step of steps) {
      toolCounts.set(step.toolId, (toolCounts.get(step.toolId) || 0) + 1)
      totalDuration += step.duration
      if (step.success) successCount++
    }

    const mostUsedTool = Array.from(toolCounts.entries()).sort((a, b) => b[1] - a[1])[0]

    return (
      `执行了${steps.length}个步骤，成功率${Math.round((successCount / steps.length) * 100)}%，` +
      `总耗时${Math.round(totalDuration)}ms，主要使用工具：${mostUsedTool?.[0] || '无'}`
    )
  }

  /**
   * 检查是否需要压缩
   */
  shouldCompress(): boolean {
    return this.executionHistory.length >= this.compressionThreshold
  }

  /**
   * 获取重规划上下文
   */
  getReplanningContext() {
    const recentHistory = this.executionHistory.slice(-10)
    return {
      recentHistory,
      usedTools: this.extractUsedTools(),
      terminalState: this.extractTerminalState(),
      summary: this.generateExecutionSummary(recentHistory),
    }
  }

  /**
   * 清理历史记录
   */
  clearHistory(): void {
    this.executionHistory = []
  }

  /**
   * 检查是否需要压缩
   */
  shouldCompress(): boolean {
    return this.executionHistory.length >= this.compressionThreshold
  }

  /**
   * 获取执行统计
   */
  getExecutionStats(): {
    totalSteps: number
    successfulSteps: number
    failedSteps: number
    successRate: number
    averageDuration: number
    mostUsedTools: Array<{ toolId: string; count: number }>
  } {
    const totalSteps = this.executionHistory.length
    const successfulSteps = this.executionHistory.filter(r => r.success).length
    const failedSteps = totalSteps - successfulSteps
    const successRate = totalSteps > 0 ? successfulSteps / totalSteps : 0

    const totalDuration = this.executionHistory.reduce((sum, r) => sum + r.duration, 0)
    const averageDuration = totalSteps > 0 ? totalDuration / totalSteps : 0

    // 统计工具使用频率
    const toolCounts = new Map<string, number>()
    for (const record of this.executionHistory) {
      toolCounts.set(record.toolId, (toolCounts.get(record.toolId) || 0) + 1)
    }

    const mostUsedTools = Array.from(toolCounts.entries())
      .map(([toolId, count]) => ({ toolId, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 5)

    return {
      totalSteps,
      successfulSteps,
      failedSteps,
      successRate,
      averageDuration,
      mostUsedTools,
    }
  }

  /**
   * 导出/导入历史记录
   */
  exportHistory(): ExecutionHistory[] {
    return [...this.executionHistory]
  }

  importHistory(history: ExecutionHistory[]): void {
    this.executionHistory = history.slice(-this.maxHistorySize)
  }
}
