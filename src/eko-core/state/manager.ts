import { EventEmitter } from '../events/emitter'

export type TaskStatus = 'init' | 'running' | 'paused' | 'done' | 'error' | 'aborted'

export interface TaskState {
  taskId: string
  taskStatus: TaskStatus
  paused: boolean
  pauseReason?: string
  consecutiveErrors: number
  iterations: number
  idleRounds: number
  maxConsecutiveErrors: number
  maxIterations: number
  maxIdleRounds: number
  lastStatusChange: number
  statusChangeReason?: string
}

/**
 * 统一状态管理器（与 UI 回调解耦）
 * 注意：与 Task.status 并行存在，不直接覆盖 Task.status，避免破坏兼容
 */
export class StateManager {
  private state: TaskState
  private eventEmitter: EventEmitter

  constructor(initialState: TaskState, eventEmitter: EventEmitter) {
    this.state = { ...initialState }
    this.eventEmitter = eventEmitter
  }

  getState(): Readonly<TaskState> {
    return { ...this.state }
  }

  updateTaskStatus(status: TaskStatus, reason?: string): void {
    const oldStatus = this.state.taskStatus
    this.state.taskStatus = status
    this.state.lastStatusChange = Date.now()
    if (reason) this.state.statusChangeReason = reason
    this.emitStateChange('task_status', { oldStatus, newStatus: status, reason })
  }

  setPauseStatus(paused: boolean, reason?: string): void {
    const oldPaused = this.state.paused
    this.state.paused = paused
    this.state.pauseReason = reason
    this.emitStateChange('pause_status', { oldPaused, newPaused: paused, reason })
  }

  incrementErrorCount(): void {
    this.state.consecutiveErrors++
    this.emitStateChange('error_count', { count: this.state.consecutiveErrors })
  }

  resetErrorCount(): void {
    this.state.consecutiveErrors = 0
    this.emitStateChange('error_count', { count: 0 })
  }

  incrementIteration(): void {
    this.state.iterations++
    this.emitStateChange('iteration', { iterations: this.state.iterations })
  }

  markIdleRound(): void {
    this.state.idleRounds++
    this.emitStateChange('idle_round', { idleRounds: this.state.idleRounds })
  }

  resetIdleRounds(): void {
    this.state.idleRounds = 0
    this.emitStateChange('idle_round', { idleRounds: 0 })
  }

  shouldHalt(): boolean {
    return (
      this.state.consecutiveErrors >= this.state.maxConsecutiveErrors ||
      this.state.iterations >= this.state.maxIterations ||
      this.state.idleRounds >= this.state.maxIdleRounds
    )
  }

  private emitStateChange(type: string, payload: Record<string, unknown>): void {
    this.eventEmitter.emit({
      type: `state_${type}_changed`,
      payload: { taskId: this.state.taskId, ...payload, timestamp: Date.now() },
    })
  }
}
