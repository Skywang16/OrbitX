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
    // wire weak references for child-task operations
    context.spawnChildTask = this.spawnChildTask.bind(this)
    context.completeChildTask = this.completeChildTask.bind(this)
    context.executeTask = async (id: string) => await this.execute(id)
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
    // wire weak references for child-task operations
    context.spawnChildTask = this.spawnChildTask.bind(this)
    context.completeChildTask = this.completeChildTask.bind(this)
    context.executeTask = async (id: string) => await this.execute(id)
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
      // emit task_status running
      this.config.callback &&
        (await this.config.callback.onMessage({
          taskId: context.taskId,
          agentName: this.agent.Name,
          nodeId: startNodeId,
          type: 'task_status',
          status: task.status,
        }))
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
      // emit task_status done
      this.config.callback &&
        (await this.config.callback.onMessage({
          taskId: context.taskId,
          agentName: this.agent.Name,
          nodeId: context.currentNodeId || startNodeId,
          type: 'task_status',
          status: task.status,
        }))
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
      // emit task_status abort->init or error
      this.config.callback &&
        (await this.config.callback.onMessage({
          taskId: context.taskId,
          agentName: this.agent.Name,
          nodeId: context.currentNodeId || generateNodeId(context.taskId, 'execution'),
          type: 'task_status',
          status: task.status,
        }))
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
      // emit pause/resume events to UI
      if (this.config.callback) {
        const eventBase = {
          taskId: context.taskId,
          agentName: this.agent.Name,
          nodeId: context.currentNodeId || generateNodeId(context.taskId, 'execution'),
        }
        if (pause) {
          this.config.callback.onMessage({ ...eventBase, type: 'task_pause', reason: reason || 'manual' })
        } else {
          this.config.callback.onMessage({ ...eventBase, type: 'task_resume', reason: reason || 'manual' })
        }
      }
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
      // task tree fields default
      rootTaskId: undefined,
      parentTaskId: undefined,
      childTaskIds: [],
    }
  }

  // Create a child task context, link to parent, and pause parent
  public async spawnChildTask(parentTaskId: string, message: string): Promise<string> {
    const parent = this.getTask(parentTaskId)
    if (!parent) throw new Error('Parent task not found')
    const childId = uuidv4()
    const chain = new Chain(message)
    const ctx = new Context(childId, this.config, this.agent, chain)
    // wire weak refs
    ctx.spawnChildTask = this.spawnChildTask.bind(this)
    ctx.completeChildTask = this.completeChildTask.bind(this)
    ctx.executeTask = async (id: string) => await this.execute(id)
    const baseTask = this.createInitialTask(childId, message)
    ctx.task = {
      ...baseTask,
      parentTaskId: parent.taskId,
      rootTaskId: parent.task?.rootTaskId || parent.taskId,
    }
    ctx.attachParent(parent.taskId, ctx.task.rootTaskId)
    this.taskMap.set(childId, ctx)
    parent.addChild(childId)
    parent.setPause(true)
    await this.config.callback?.onMessage({
      type: 'task_spawn',
      taskId: childId,
      agentName: this.agent.Name,
      nodeId: generateNodeId(childId, 'start'),
      parentTaskId: parent.taskId,
      rootTaskId: ctx.task.rootTaskId!,
      task: ctx.task,
    })
    await this.config.callback?.onMessage({
      type: 'task_pause',
      taskId: parent.taskId,
      agentName: this.agent.Name,
      nodeId: parent.currentNodeId || generateNodeId(parent.taskId, 'execution'),
      reason: 'child_spawn',
    })
    await this.onTaskStatus(parent, 'pause', 'child_spawn')
    return childId
  }

  // Complete the child task, emit result back to parent, and resume parent
  public async completeChildTask(childTaskId: string, summary: string, payload?: unknown): Promise<void> {
    const child = this.getTask(childTaskId)
    if (!child) return
    const parentId = child.parentTaskId || child.task?.parentTaskId
    const parent = parentId ? this.getTask(parentId) : undefined
    if (child.task) {
      child.task.status = 'done'
      await this.config.callback?.onMessage({
        type: 'task_status',
        taskId: child.task.taskId,
        agentName: this.agent.Name,
        nodeId: child.currentNodeId || generateNodeId(child.task.taskId, 'execution'),
        status: child.task.status,
      })
    }
    if (parent) {
      await this.config.callback?.onMessage({
        type: 'task_child_result',
        taskId: childTaskId,
        agentName: this.agent.Name,
        nodeId: parent.currentNodeId || generateNodeId(parent.taskId, 'execution'),
        parentTaskId: parent.taskId,
        summary,
        payload,
      })
      parent.setPause(false)
      await this.config.callback?.onMessage({
        type: 'task_resume',
        taskId: parent.taskId,
        agentName: this.agent.Name,
        nodeId: parent.currentNodeId || generateNodeId(parent.taskId, 'execution'),
        reason: 'child_done',
      })
      await this.onTaskStatus(parent, 'resume-pause', 'child_done')
    }
  }
}
