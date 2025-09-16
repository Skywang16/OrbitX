import Context, { generateNodeId } from './context'
import { Agent } from '../agent'
import { Planner } from './plan'
import Log from '../common/log'
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
    // Context parameters no longer supported - use conversation history instead
    try {
      this.taskMap.set(taskId, context)
      const planner = new Planner(context)
      context.task = await planner.plan(taskPrompt)
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
    // A2A client removed - single agent mode
    const planner = new Planner(context)
    context.task = await planner.replan(modifyTaskPrompt)
    return context.task
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
    try {
      return await this.doRunTask(context)
    } catch (e: unknown) {
      Log.error('execute error', e instanceof Error ? e : String(e))
      const errName = (e as { name?: string }).name || 'Error'
      const errMsg = (e as { message?: string }).message || ''
      return {
        taskId,
        success: false,
        stopReason: (e as { name?: string })?.name == 'AbortError' ? 'abort' : 'error',
        result: e ? `${errName}: ${errMsg}` : 'Error',
        error: e,
      }
    }
  }

  public async run(
    taskPrompt: string,
    taskId: string = uuidv4(),
    _contextParams?: Record<string, unknown>
  ): Promise<EkoResult> {
    const chain: Chain = new Chain(taskPrompt)
    const context = new Context(taskId, this.config, this.agent, chain)

    // 创建一个简单的task对象，但不通过Planner生成
    context.task = {
      taskId,
      name: 'Direct Agent Task',
      thought: 'Processing user request directly without pre-planning',
      taskPrompt,
      description: `Process user request: ${taskPrompt}`,
      nodes: [], // Agent会根据需要动态创建节点
      status: 'init',
      xml: `<task>${taskPrompt}</task>`, // 简单的XML包装
    }

    this.taskMap.set(taskId, context)

    try {
      return await this.doRunTask(context)
    } catch (e) {
      this.deleteTask(taskId)
      throw e
    }
  }

  public async initContext(task: Task, _contextParams?: Record<string, unknown>): Promise<Context> {
    const chain: Chain = new Chain(task.taskPrompt || task.name)
    const context = new Context(task.taskId, this.config, this.agent, chain)
    // Context parameters no longer supported - use conversation history instead
    context.task = task
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
          },
          this.agent.AgentContext
        ))

      await this.onTaskStatus(context, 'done')
      return {
        taskId: context.taskId,
        success: true,
        stopReason: 'done',
        result: result,
      }
    } catch (e) {
      // Notify task error
      this.config.callback &&
        (await this.config.callback.onMessage(
          {
            taskId: context.taskId,
            agentName: this.agent.Name,
            type: 'agent_result',
            task: task,
            error: e,
          },
          this.agent.AgentContext
        ))

      throw e
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
}
