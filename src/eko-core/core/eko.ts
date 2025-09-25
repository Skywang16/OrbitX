import Context, { generateNodeId } from './context'
import { Agent } from '../agent'
import Chain from './chain'
import { uuidv4 } from '../common/utils'
import { EkoConfig, EkoResult, Task, PlannedTask } from '../types/core.types'
import { buildAgentXmlFromPlanned, parseTask } from '../common/xml'
import { EventEmitter } from '../events/emitter'
import { StateManager, type TaskState } from '../state/manager'
import globalConfig from '../config'
import { ToolRegistry } from '../tools/registry'

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

  private initInfra(context: Context): void {
    const emitter = new EventEmitter()
    const initial: TaskState = {
      taskId: context.taskId,
      taskStatus: 'init',
      paused: false,
      consecutiveErrors: 0,
      iterations: 0,
      idleRounds: 0,
      maxConsecutiveErrors: globalConfig.maxReactErrorStreak,
      maxIterations: globalConfig.maxReactNum,
      maxIdleRounds: globalConfig.maxReactIdleRounds,
      lastStatusChange: Date.now(),
    }
    context.eventEmitter = emitter
    context.stateManager = new StateManager(initial, emitter)

    // 初始化工具注册表并注入（统一来源：静态 + 动态 + MCP）
    const registry = new ToolRegistry()
    // 注册 Agent 静态工具（若有）
    if (this.agent && Array.isArray((this.agent as unknown as { Tools?: unknown }).Tools)) {
      try {
        // @ts-ignore - Agent.Tools getter
        registry.registerStaticTools(this.agent.Tools || [])
      } catch {
        // ignore
      }
    }
    // 注册默认 MCP 客户端（若有）
    if (this.config.defaultMcpClient) {
      registry.registerMcpClient('default', this.config.defaultMcpClient)
    }
    context.toolRegistry = registry
  }

  public async spawnPlannedTree(
    parentTaskId: string,
    planned: PlannedTask,
    options?: { silent?: boolean }
  ): Promise<{ rootId: string; allTaskIds: string[]; leafTaskIds: string[] }> {
    const parent = this.getTask(parentTaskId)
    if (!parent) throw new Error('Parent task not found')

    const allIds: string[] = []
    const leafIds: string[] = []

    // Update parent's name/description only (do not override parent's xml or nodes)
    if (parent.task) {
      if (planned.name && planned.name.trim()) {
        parent.task.name = planned.name.trim().slice(0, 80)
      } else if (planned.description && planned.description.trim()) {
        parent.task.name = planned.description.trim().slice(0, 80)
      }
      if (planned.description && planned.description.trim()) {
        parent.task.description = planned.description.trim()
      }
      if (this.config.callback) {
        await this.config.callback.onMessage({
          taskId: parent.taskId,
          agentName: this.agent.Name,
          type: 'task',
          streamDone: true,
          task: parent.task,
        })
      }
    }

    const silent = !!options?.silent

    // Flatten any intermediate grouping: spawn only a single level of children under parent
    const groups = planned.subtasks || []
    const flattened: PlannedTask[] = []
    for (const g of groups) {
      if (g.subtasks && g.subtasks.length > 0) {
        flattened.push(...g.subtasks)
      } else {
        flattened.push(g)
      }
    }

    for (const sub of flattened) {
      const subMsg = sub.description || sub.name || 'Subtask'
      const subId = await this.spawnChildTask(parentTaskId, subMsg, { pauseParent: false, silent })
      allIds.push(subId)
      leafIds.push(subId)

      const subCtx = this.getTask(subId)
      if (subCtx?.task) {
        if (sub.name && sub.name.trim()) {
          subCtx.task.name = sub.name.trim().slice(0, 80)
        } else if (sub.description && sub.description.trim()) {
          subCtx.task.name = sub.description.trim().slice(0, 80)
        }
        if (sub.description && sub.description.trim()) subCtx.task.description = sub.description.trim()
        subCtx.task.xml = buildAgentXmlFromPlanned(sub)
        if (this.config.callback) {
          const parsedSub = parseTask(subCtx.task.taskId, subCtx.task.xml, false)
          if (parsedSub) {
            parsedSub.rootTaskId = subCtx.task.rootTaskId
            parsedSub.parentTaskId = subCtx.task.parentTaskId
            parsedSub.childTaskIds = subCtx.task.childTaskIds
            parsedSub.name = subCtx.task.name || parsedSub.name
            parsedSub.description = subCtx.task.description || parsedSub.description
            await this.config.callback.onMessage({
              taskId: subCtx.taskId,
              agentName: this.agent.Name,
              type: 'task',
              streamDone: true,
              task: parsedSub,
            })
          }
        }
      }
    }

    // Emit one tree update for the parent with its direct children
    if (this.config.callback && parent.task) {
      await this.config.callback.onMessage({
        taskId: parent.taskId,
        agentName: this.agent.Name,
        type: 'task_tree_update',
        parentTaskId: parent.taskId,
        childTaskIds: parent.task.childTaskIds || [],
      })
    }

    return { rootId: parentTaskId, allTaskIds: allIds, leafTaskIds: leafIds }
  }

  public async generate(
    taskPrompt: string,
    taskId: string = uuidv4(),
    _contextParams?: Record<string, unknown>
  ): Promise<Task> {
    const chain: Chain = new Chain(taskPrompt)
    const context = new Context(taskId, this.config, this.agent, chain)
    this.initInfra(context)
    context.spawnChildTask = this.spawnChildTask.bind(this)
    context.spawnPlannedTree = this.spawnPlannedTree.bind(this)
    context.completeChildTask = this.completeChildTask.bind(this)
    context.executeTask = async (id: string) => await this.execute(id)
    context.getTaskContext = this.getTask.bind(this)
    context.deleteTask = this.deleteTask.bind(this)
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
    this.initInfra(context)
    context.spawnChildTask = this.spawnChildTask.bind(this)
    context.spawnPlannedTree = this.spawnPlannedTree.bind(this)
    context.completeChildTask = this.completeChildTask.bind(this)
    context.executeTask = async (id: string) => await this.execute(id)
    context.getTaskContext = this.getTask.bind(this)
    context.deleteTask = this.deleteTask.bind(this)
    const baseTask = this.createInitialTask(task.taskId, chain.taskPrompt || task.name)
    context.task = {
      ...baseTask,
      ...task,
      taskPrompt: (task.taskPrompt || baseTask.taskPrompt || '').trim(),
      description: (task.description || baseTask.description || '').trim(),
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
      const startNodeId = generateNodeId(context.taskId, 'start')
      context.currentNodeId = startNodeId
      task.status = 'running'
      context.stateManager.updateTaskStatus('running')
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

      const result = await this.agent.run(context)

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

      task.status = 'done'
      context.stateManager?.updateTaskStatus('done')
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
      context.stateManager.updateTaskStatus(isAbort ? 'aborted' : 'error')
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

      task.status = isAbort ? 'init' : 'error'
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
      context.stateManager.updateTaskStatus('aborted', reason)
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
      context.stateManager.setPauseStatus(pause, reason)
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
      rootTaskId: undefined,
      parentTaskId: undefined,
      childTaskIds: [],
    }
  }

  public async spawnChildTask(
    parentTaskId: string,
    message: string,
    options?: { silent?: boolean; pauseParent?: boolean }
  ): Promise<string> {
    const parent = this.getTask(parentTaskId)
    if (!parent) throw new Error('Parent task not found')
    const childId = uuidv4()
    const chain = new Chain(message)
    const ctx = new Context(childId, this.config, this.agent, chain)
    this.initInfra(ctx)
    ctx.spawnChildTask = this.spawnChildTask.bind(this)
    ctx.spawnPlannedTree = this.spawnPlannedTree.bind(this)
    ctx.completeChildTask = this.completeChildTask.bind(this)
    ctx.executeTask = async (id: string) => await this.execute(id)
    ctx.getTaskContext = this.getTask.bind(this)
    ctx.deleteTask = this.deleteTask.bind(this)
    const baseTask = this.createInitialTask(childId, message)
    ctx.task = {
      ...baseTask,
      parentTaskId: parent.taskId,
      rootTaskId: parent.task?.rootTaskId || parent.taskId,
    }
    ctx.attachParent(parent.taskId, ctx.task.rootTaskId)
    this.taskMap.set(childId, ctx)
    parent.addChild(childId)
    const pauseParent = options?.pauseParent !== undefined ? options.pauseParent : true
    if (pauseParent) {
      parent.setPause(true)
      parent.stateManager.setPauseStatus(true, 'child_spawn')
    }
    if (!options?.silent) {
      await this.config.callback?.onMessage({
        type: 'task_spawn',
        taskId: childId,
        agentName: this.agent.Name,
        nodeId: generateNodeId(childId, 'start'),
        parentTaskId: parent.taskId,
        rootTaskId: ctx.task.rootTaskId!,
        task: ctx.task,
      })
      if (pauseParent) {
        await this.config.callback?.onMessage({
          type: 'task_pause',
          taskId: parent.taskId,
          agentName: this.agent.Name,
          nodeId: parent.currentNodeId || generateNodeId(parent.taskId, 'execution'),
          reason: 'child_spawn',
        })
        await this.onTaskStatus(parent, 'pause', 'child_spawn')
      }
    }
    return childId
  }

  public async completeChildTask(childTaskId: string, summary: string, payload?: unknown): Promise<void> {
    const child = this.getTask(childTaskId)
    if (!child) return
    const parentId = child.parentTaskId || child.task?.parentTaskId
    const parent = parentId ? this.getTask(parentId) : undefined
    if (child.task) {
      child.task.status = 'done'
      child.stateManager.updateTaskStatus('done')
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
      parent.stateManager.setPauseStatus(false, 'child_done')
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
