import type { Agent } from '../agent'
import { sleep } from '../common/utils'
import Chain from './chain'
import { EkoConfig, NativeLLMMessage, Task } from '../types'
import config from '../config'
import ReactRuntime from '../react/runtime'
import { ReactRuntimeConfig } from '../react/types'

/**
 * 生成节点ID
 * @param taskId 任务ID
 * @param phase 执行阶段
 * @param nodeIndex 节点索引（可选）
 * @returns 生成的节点ID
 */
export function generateNodeId(
  taskId: string,
  phase: 'planning' | 'execution' | 'thinking' | 'start',
  nodeIndex?: number
): string {
  if (nodeIndex !== undefined) {
    return `${taskId}_node_${nodeIndex}`
  }
  return `${taskId}_${phase}`
}

export default class Context {
  taskId: string
  config: EkoConfig
  chain: Chain
  agent: Agent
  controller: AbortController
  task?: Task
  conversation: string[] = []
  currentNodeId?: string // 当前执行的节点ID
  private pauseStatus: 0 | 1 | 2 = 0
  readonly currentStepControllers: Set<AbortController> = new Set()
  readonly reactRuntime: ReactRuntime
  // 任务树字段
  rootTaskId?: string
  parentTaskId?: string
  children: string[] = []
  // 与 Eko 的弱引用能力（避免循环依赖）
  spawnChildTask?: (parentTaskId: string, message: string) => Promise<string>
  completeChildTask?: (childTaskId: string, summary: string, payload?: unknown) => Promise<void>
  executeTask?: (taskId: string) => Promise<{
    taskId: string
    success: boolean
    stopReason: 'abort' | 'error' | 'done'
    result: string
    error?: unknown
  }>

  constructor(taskId: string, config: EkoConfig, agent: Agent, chain: Chain) {
    this.taskId = taskId
    this.config = config
    this.agent = agent
    this.chain = chain
    this.controller = new AbortController()
    this.reactRuntime = new ReactRuntime(this.createReactRuntimeConfig())
  }

  // 绑定父/根任务ID
  attachParent(parentTaskId: string, rootTaskId?: string) {
    this.parentTaskId = parentTaskId
    this.rootTaskId = rootTaskId || parentTaskId
    if (this.task) {
      this.task.parentTaskId = parentTaskId
      this.task.rootTaskId = this.rootTaskId
    }
  }

  // 添加子任务ID
  addChild(childTaskId: string) {
    if (!this.children.includes(childTaskId)) {
      this.children.push(childTaskId)
    }
    if (this.task) {
      const existing = this.task.childTaskIds || []
      if (!existing.includes(childTaskId)) {
        this.task.childTaskIds = [...existing, childTaskId]
      }
    }
  }

  async checkAborted(noCheckPause?: boolean): Promise<void> {
    if (this.controller.signal.aborted) {
      const error = new Error('Operation was interrupted')
      error.name = 'AbortError'
      throw error
    }
    while (this.pauseStatus > 0 && !noCheckPause) {
      await sleep(500)
      if (this.pauseStatus == 2) {
        this.currentStepControllers.forEach(c => {
          c.abort('Pause')
        })
        this.currentStepControllers.clear()
      }
      if (this.controller.signal.aborted) {
        const error = new Error('Operation was interrupted')
        error.name = 'AbortError'
        throw error
      }
    }
  }

  currentAgent(): [Agent, AgentContext] | null {
    const agentContext = this.agent.AgentContext as AgentContext
    if (!agentContext) {
      return null
    }
    return [this.agent, agentContext]
  }

  get pause() {
    return this.pauseStatus > 0
  }

  setPause(pause: boolean, abortCurrentStep?: boolean) {
    this.pauseStatus = pause ? (abortCurrentStep ? 2 : 1) : 0
    if (this.pauseStatus == 2) {
      this.currentStepControllers.forEach(c => {
        c.abort('Pause')
      })
      this.currentStepControllers.clear()
    }
  }

  private createReactRuntimeConfig(): ReactRuntimeConfig {
    return {
      maxIterations: config.maxReactNum,
      maxIdleRounds: config.maxReactIdleRounds,
      maxConsecutiveErrors: config.maxReactErrorStreak,
    }
  }
}

export class AgentContext {
  agent: Agent
  context: Context
  consecutiveErrorNum: number
  messages?: NativeLLMMessage[]
  platform?: string

  constructor(context: Context, agent: Agent, platform?: string) {
    this.context = context
    this.agent = agent
    this.consecutiveErrorNum = 0
    this.platform = platform || 'web'
  }
}
