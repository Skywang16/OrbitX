import Context, { generateNodeId } from './context'
import { Agent } from '../agent'
import Chain from './chain'
import { uuidv4 } from '../common/utils'
import { EkoConfig, EkoResult, Task } from '../types/core.types'

export class Eko {
  protected config: EkoConfig
  protected taskMap: Map<string, Context>
  protected agent: Agent

  constructor(config: EkoConfig) {
    this.config = config
    this.taskMap = new Map()

    if (!config.agent) {
      throw new Error('Agent is required in config')
    }
    this.agent = config.agent
  }

  public async generate(
    taskPrompt: string,
    taskId: string = uuidv4(),
    _contextParams?: Record<string, unknown>
  ): Promise<Task> {
    const chain: Chain = new Chain(taskPrompt)
    const context = new Context(taskId, this.config, this.agent, chain)
    try {
      this.taskMap.set(taskId, context)
      context.task = this.createInitialTask(taskId, taskPrompt)
      return context.task
    } catch (e) {
      this.deleteTask(taskId)
      throw e
    }
  }

  public async modify(taskId: string, modifyTaskPrompt: string): Promise<Task> {
    const context = this.taskMap.get(taskId)
    if (!context) {
      return await this.generate(modifyTaskPrompt, taskId)
    }
    // Reset existing context to initial task state with new prompt
    this.deleteTask(taskId)
    return await this.generate(modifyTaskPrompt, taskId)
  }

  public async execute(taskId: string): Promise<EkoResult> {
    const context = this.getTask(taskId)
    if (!context) {
      throw new Error('The task does not exist')
    }
    if (context.pause) {
      context.setPause(false)
    }
    context.conversation = []
    if (context.controller.signal.aborted) {
      context.controller = new AbortController()
    }
    return await this.doRunTask(context)
  }

  public async run(
    taskPrompt: string,
    taskId: string = uuidv4(),
    _contextParams?: Record<string, unknown>
  ): Promise<EkoResult> {
    await this.generate(taskPrompt, taskId)
    return await this.execute(taskId)
  }

  public async initContext(task: Task, _contextParams?: Record<string, unknown>): Promise<Context> {
    const chain: Chain = new Chain(task.taskPrompt || task.name)
    const context = new Context(task.taskId, this.config, this.agent, chain)
    // Context parameters no longer supported - use conversation history instead
    const baseTask = this.createInitialTask(task.taskId, chain.taskPrompt)
    context.task = {
      ...baseTask,
      ...task,
      taskPrompt: (task.taskPrompt || baseTask.taskPrompt).trim(),
      description: (task.description || baseTask.description).trim(),
    }
    this.taskMap.set(task.taskId, context)
    return context
  }

  private async doRunTask(context: Context): Promise<EkoResult> {
    const task = context.task
    if (!task) {
      throw new Error('No task found')
    }

    await context.checkAborted()

    try {
      // Notify task start
      const startNodeId = generateNodeId(context.taskId, 'start')
      context.currentNodeId = startNodeId
      // Mark task as running at the beginning of execution
      task.status = 'running'
      this.config.callback &&
        (await this.config.callback.onMessage({
          taskId: context.taskId,
          agentName: this.agent.Name,
          nodeId: startNodeId,
          type: 'agent_start',
          task: task,
        }))

      // Execute the single agent
      const result = await this.agent.run(context)

      // Notify task completion
      this.config.callback &&
        (await this.config.callback.onMessage(
          {
            taskId: context.taskId,
            agentName: this.agent.Name,
            type: 'agent_result',
            task: task,
            result: result,
            stopReason: 'done',
          },
          this.agent.AgentContext
        ))

      // Mark task done on success
      task.status = 'done'
      await this.onTaskStatus(context, 'done')
      return {
        taskId: context.taskId,
        success: true,
        stopReason: 'done',
        result: result,
      }
    } catch (e) {
      const isAbort = (e as { name?: string })?.name === 'AbortError'
      context.reactRuntime.setStopReason(isAbort ? 'abort' : 'error')
      // Notify task end with stopReason
      this.config.callback &&
        (await this.config.callback.onMessage(
          {
            taskId: context.taskId,
            agentName: this.agent.Name,
            type: 'agent_result',
            task: task,
            ...(isAbort ? { stopReason: 'abort' as const } : { stopReason: 'error' as const, error: e }),
          },
          this.agent.AgentContext
        ))

      // Mark task status for abort/error
      task.status = isAbort ? 'init' : 'error'
      await this.onTaskStatus(context, isAbort ? 'abort' : 'error')
      return {
        taskId: context.taskId,
        success: false,
        stopReason: isAbort ? 'abort' : 'error',
        result: '',
        error: e,
      }
    }
  }

  public getTask(taskId: string): Context | undefined {
    return this.taskMap.get(taskId)
  }

  public getAllTaskId(): string[] {
    return Array.from(this.taskMap.keys())
  }

  public deleteTask(taskId: string): boolean {
    this.abortTask(taskId)
    return this.taskMap.delete(taskId)
  }

  public abortTask(taskId: string, reason?: string): boolean {
    let context = this.taskMap.get(taskId)
    if (context) {
      context.setPause(false)
      this.onTaskStatus(context, 'abort', reason)
      context.controller.abort(reason)
      return true
    } else {
      return false
    }
  }

  public pauseTask(taskId: string, pause: boolean, abortCurrentStep?: boolean, reason?: string): boolean {
    const context = this.taskMap.get(taskId)
    if (context) {
      this.onTaskStatus(context, pause ? 'pause' : 'resume-pause', reason)
      context.setPause(pause, abortCurrentStep)
      return true
    } else {
      return false
    }
  }

  public chatTask(taskId: string, userPrompt: string): string[] | undefined {
    const context = this.taskMap.get(taskId)
    if (context) {
      context.conversation.push(userPrompt)
      return context.conversation
    }
  }

  private async onTaskStatus(_context: Context, status: string, reason?: string) {
    type AgentWithOnTaskStatus = Agent & { onTaskStatus?: (status: string, reason?: string) => Promise<void> | void }
    const agent = this.agent as unknown as AgentWithOnTaskStatus
    if (agent.onTaskStatus) {
      await agent.onTaskStatus(status, reason)
    }
  }

  private createInitialTask(taskId: string, taskPrompt: string): Task {
    const normalizedPrompt = (taskPrompt || '').trim()
    const fallbackName = normalizedPrompt.split('\n')[0]?.trim() || `Task-${taskId}`
    return {
      taskId,
      name: fallbackName.slice(0, 80),
      thought: '',
      description: normalizedPrompt,
      nodes: [],
      status: 'init',
      xml: '<task></task>',
      taskPrompt: normalizedPrompt,
    }
  }
}
