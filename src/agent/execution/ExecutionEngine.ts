/**
 * 简化的执行引擎
 *
 * 使用简单的树形执行模型而不是复杂的依赖图
 * 增强版本：集成TaskContext、AgentContext和TaskSnapshot机制
 */

import type { WorkflowDefinition, WorkflowAgent, AgentNode } from '../types/workflow'
import type { ExecutionResult, IExecutionEngine } from '../types/execution'
import type { ExecutionCallback } from '../types/callbacks'
import { agentRegistry } from '../agents/AgentRegistry'
import { CallbackManager } from '../core/CallbackManager'
import { TaskContext } from '../context/TaskContext'
import { AgentContext } from '../context/AgentContext'
import { TaskSnapshotManager } from '../core/TaskSnapshotManager'
import { AgentFrameworkConfig } from '../index'
import { BaseAgent } from '../agents/BaseAgent'

/**
 * 简化的执行引擎类
 * 增强版本：集成TaskContext、AgentContext和快照机制
 */
export class ExecutionEngine implements IExecutionEngine {
  private taskContextMap: Map<string, TaskContext> = new Map()
  private agentContextMap: Map<string, AgentContext[]> = new Map()
  private callbackManager: CallbackManager
  private snapshotManager: TaskSnapshotManager
  private config: AgentFrameworkConfig

  constructor(config: AgentFrameworkConfig = {}, callbackManager?: CallbackManager) {
    this.config = config
    this.callbackManager = callbackManager || new CallbackManager()
    this.snapshotManager = new TaskSnapshotManager()
  }

  /**
   * 执行工作流 - 主入口（增强版本）
   */
  async execute(
    workflow: WorkflowDefinition,
    contextParams?: Record<string, unknown>,
    callback?: ExecutionCallback
  ): Promise<ExecutionResult> {
    // 如果提供了callback，注册到CallbackManager
    if (callback) {
      this.callbackManager.onExecution(callback)
    }

    // 创建TaskContext
    const taskContext = new TaskContext(workflow.taskId, this.config, workflow, contextParams)
    this.taskContextMap.set(workflow.taskId, taskContext)

    // 创建AgentContexts
    const agentContexts = workflow.agents.map(agent => new AgentContext(agent, taskContext))
    this.agentContextMap.set(workflow.taskId, agentContexts)

    // 启动自动快照
    this.snapshotManager.startAutoSnapshot(taskContext, agentContexts)

    // 触发工作流开始事件
    await this.callbackManager.triggerExecutionEvent({
      type: 'workflow_start',
      timestamp: new Date(),
      workflowId: workflow.taskId,
      data: { workflow, contextParams },
      metadata: {
        workflowName: workflow.name,
        agentCount: workflow.agents.length,
        executionStartTime: new Date().toISOString(),
      },
    })

    try {
      const result = await this.executeWithContext(taskContext, agentContexts)

      // 创建完成快照
      await this.snapshotManager.createSnapshot(taskContext, agentContexts, 'manual')

      // 触发工作流完成事件
      await this.callbackManager.triggerExecutionEvent({
        type: 'workflow_completed',
        timestamp: new Date(),
        workflowId: workflow.taskId,
        data: { result },
        metadata: {
          workflowName: workflow.name,
          executionTime: Date.now() - Date.now(), // 简化处理，实际应该记录开始时间
          executionEndTime: new Date().toISOString(),
        },
      })

      return {
        taskId: workflow.taskId,
        success: true,
        stopReason: 'done',
        result: result || '任务执行完成',
      }
    } catch (error) {
      // 创建错误快照
      await this.snapshotManager.createSnapshot(taskContext, agentContexts, 'error')

      // 触发工作流失败事件
      await this.callbackManager.triggerExecutionEvent({
        type: 'workflow_failed',
        timestamp: new Date(),
        workflowId: workflow.taskId,
        error: error instanceof Error ? error.message : String(error),
        metadata: {
          workflowName: workflow.name,
          errorType: error instanceof Error ? error.constructor.name : 'UnknownError',
          executionEndTime: new Date().toISOString(),
        },
      })

      return {
        taskId: workflow.taskId,
        success: false,
        stopReason: 'error',
        result: error instanceof Error ? error.message : String(error),
        error,
      }
    } finally {
      // 停止自动快照
      this.snapshotManager.stopAutoSnapshot(workflow.taskId)

      // 清理上下文
      this.taskContextMap.delete(workflow.taskId)
      this.agentContextMap.delete(workflow.taskId)
    }
  }

  /**
   * 使用上下文执行工作流
   */
  private async executeWithContext(taskContext: TaskContext, agentContexts: AgentContext[]): Promise<string> {
    const agentTree = this.buildAgentTree(taskContext.workflow!.agents)
    return this.executeAgentTreeWithContext(agentTree, agentContexts)
  }

  /**
   * 使用AgentContext执行Agent树
   */
  private async executeAgentTreeWithContext(agentTree: AgentNode, agentContexts: AgentContext[]): Promise<string> {
    let currentTree: AgentNode | undefined = agentTree
    let lastResult = ''

    while (currentTree) {
      if (currentTree.type === 'normal' && currentTree.agent) {
        const agentContext = agentContexts.find(ctx => ctx.agent.id === currentTree.agent!.id)
        if (agentContext) {
          lastResult = await this.runSingleAgentWithContext(agentContext)
          currentTree.result = lastResult
        }
      } else if (currentTree.type === 'parallel' && currentTree.agents) {
        const parallelResults = await this.runParallelAgentsWithContext(currentTree.agents, agentContexts)
        lastResult = parallelResults.join('\n\n')
        currentTree.result = lastResult
      }
      currentTree = currentTree.nextAgent
    }
    return lastResult
  }

  /**
   * 使用AgentContext运行单个Agent
   */
  private async runSingleAgentWithContext(agentContext: AgentContext): Promise<string> {
    const agent = agentContext.agent
    const startTime = Date.now()

    try {
      // 触发Agent开始事件
      await this.callbackManager.triggerExecutionEvent({
        type: 'agent_start',
        timestamp: new Date(),
        workflowId: agentContext.taskContext.taskId,
        data: { agent },
        metadata: {
          agentId: agent.id,
          agentName: agent.name,
          agentTask: agent.task,
        },
      })

      // 获取Agent实例
      const agentInstance = agentRegistry.getAgent(agent.type || 'tool')
      if (!agentInstance) {
        throw new Error(`Agent type '${agent.type || 'tool'}' not found`)
      }

      // 使用新的executeWithContext方法
      const agentResult = await (agentInstance as BaseAgent).executeWithRetry(agentContext)

      if (!agentResult.success) {
        throw new Error(agentResult.error || 'Agent执行失败')
      }

      agent.status = 'done'
      const resultString = typeof agentResult.data === 'string' ? agentResult.data : JSON.stringify(agentResult.data)

      // 触发Agent完成事件
      await this.callbackManager.triggerExecutionEvent({
        type: 'agent_completed',
        timestamp: new Date(),
        workflowId: agentContext.taskContext.taskId,
        data: { agent, result: agentResult },
        metadata: {
          agentId: agent.id,
          agentName: agent.name,
          executionTime: Date.now() - startTime,
          success: true,
        },
      })

      return resultString
    } catch (error) {
      agent.status = 'error'
      const errorMessage = error instanceof Error ? error.message : String(error)

      // 触发Agent失败事件
      await this.callbackManager.triggerExecutionEvent({
        type: 'agent_failed',
        timestamp: new Date(),
        workflowId: agentContext.taskContext.taskId,
        error: errorMessage,
        metadata: {
          agentId: agent.id,
          agentName: agent.name,
          executionTime: Date.now() - startTime,
          success: false,
        },
      })

      throw error
    }
  }

  /**
   * 使用AgentContext运行并行Agents
   */
  private async runParallelAgentsWithContext(
    agents: WorkflowAgent[],
    agentContexts: AgentContext[]
  ): Promise<string[]> {
    const promises = agents.map(agent => {
      const agentContext = agentContexts.find(ctx => ctx.agent.id === agent.id)
      return agentContext ? this.runSingleAgentWithContext(agentContext) : Promise.resolve('')
    })

    return Promise.all(promises)
  }

  /**
   * 构建Agent执行树
   */
  private buildAgentTree(agents: WorkflowAgent[]): AgentNode {
    if (agents.length === 0) {
      throw new Error('没有可执行的Agent')
    }

    const safeAgents = this.detectAndBreakCycles(agents)
    const agentMap = new Map<string, WorkflowAgent>()
    const dependents = new Map<string, WorkflowAgent[]>()

    for (const agent of safeAgents) {
      agentMap.set(agent.id, agent)
      dependents.set(agent.id, [])
    }

    for (const agent of safeAgents) {
      for (const depId of agent.dependsOn) {
        if (dependents.has(depId)) {
          dependents.get(depId)!.push(agent)
        }
      }
    }

    const entryAgents = safeAgents.filter(agent => agent.dependsOn.length === 0)
    if (entryAgents.length === 0) {
      throw new Error('没有找到入口Agent，所有Agent都有依赖')
    }

    const processedAgents = new Set<string>()
    const buildNodeRecursive = (currentAgents: WorkflowAgent[]): AgentNode | undefined => {
      if (currentAgents.length === 0) return undefined

      currentAgents.forEach(agent => processedAgents.add(agent.id))

      const nextLevelAgents = currentAgents.flatMap(agent =>
        (dependents.get(agent.id) || []).filter(dependent =>
          dependent.dependsOn.every(depId => processedAgents.has(depId))
        )
      )

      const uniqueNextLevelAgents = [...new Map(nextLevelAgents.map(item => [item.id, item])).values()]

      const nextNode = buildNodeRecursive(uniqueNextLevelAgents)

      if (currentAgents.length === 1) {
        return { type: 'normal', agent: currentAgents[0], nextAgent: nextNode }
      } else {
        return { type: 'parallel', agents: currentAgents, nextAgent: nextNode }
      }
    }

    const rootNode = buildNodeRecursive(entryAgents)
    if (!rootNode) {
      throw new Error('无法构建执行树')
    }

    return rootNode
  }

  /**
   * 检测并处理循环依赖
   */
  private detectAndBreakCycles(agents: WorkflowAgent[]): WorkflowAgent[] {
    const agentMap = new Map<string, WorkflowAgent>()
    const inDegree = new Map<string, number>()
    const adjList = new Map<string, string[]>()

    for (const agent of agents) {
      agentMap.set(agent.id, agent)
      inDegree.set(agent.id, 0)
      adjList.set(agent.id, [])
    }

    for (const agent of agents) {
      for (const depId of agent.dependsOn) {
        if (agentMap.has(depId)) {
          adjList.get(depId)!.push(agent.id)
          inDegree.set(agent.id, (inDegree.get(agent.id) || 0) + 1)
        }
      }
    }

    const queue: string[] = []
    inDegree.forEach((degree, agentId) => {
      if (degree === 0) {
        queue.push(agentId)
      }
    })

    let processedNodes = 0
    while (queue.length > 0) {
      const currentId = queue.shift()!
      processedNodes++
      for (const neighborId of adjList.get(currentId) || []) {
        const newInDegree = (inDegree.get(neighborId) || 0) - 1
        inDegree.set(neighborId, newInDegree)
        if (newInDegree === 0) {
          queue.push(neighborId)
        }
      }
    }

    if (processedNodes < agents.length) {
      const cyclicNodes = new Set<string>()
      inDegree.forEach((degree, agentId) => {
        if (degree > 0) {
          cyclicNodes.add(agentId)
        }
      })
      return agents.map(agent => (cyclicNodes.has(agent.id) ? { ...agent, dependsOn: [] } : agent))
    }

    return agents
  }
}
