/**
 * 简化的执行引擎
 *
 * 参考eko的设计理念，使用简单的树形执行模型而不是复杂的依赖图
 */

import type { WorkflowDefinition, WorkflowAgent, AgentNode, WorkflowExecution, ExecutionEvent } from '../types/workflow'
import type { AgentExecutionContext, ExecutionResult, IExecutionEngine } from '../types/execution'
import type { ToolManager } from '../tools/ToolManager'
import { agentRegistry } from '../agents/AgentRegistry'

/**
 * 执行事件回调
 */
export type ExecutionCallback = (event: ExecutionEvent) => Promise<void>

/**
 * 简化的执行引擎类
 */
export class SimpleExecutionEngine implements IExecutionEngine {
  private taskMap: Map<string, WorkflowExecution> = new Map()
  private toolManager: ToolManager

  constructor(toolManager: ToolManager) {
    this.toolManager = toolManager
  }

  /**
   * 执行工作流 - 主入口
   */
  async execute(
    workflow: WorkflowDefinition,
    contextParams?: Record<string, unknown>,
    callback?: ExecutionCallback
  ): Promise<ExecutionResult> {
    const execution = this.initializeExecution(workflow, contextParams)
    this.taskMap.set(workflow.taskId, execution)

    try {
      const agentTree = this.buildAgentTree(workflow.agents)
      const result = await this.executeAgentTree(agentTree, execution, callback)

      execution.status = 'completed'
      execution.endTime = new Date()

      return {
        taskId: workflow.taskId,
        success: true,
        stopReason: 'done',
        result: result || '任务执行完成',
      }
    } catch (error) {
      execution.status = 'failed'
      execution.endTime = new Date()
      execution.error = error instanceof Error ? error.message : String(error)

      if (callback) {
        await callback({
          type: 'workflow_failed',
          timestamp: new Date(),
          error: execution.error,
        })
      }

      return {
        taskId: workflow.taskId,
        success: false,
        stopReason: 'error',
        result: execution.error,
        error,
      }
    } finally {
      this.taskMap.delete(workflow.taskId)
    }
  }

  /**
   * 执行单个Agent的核心逻辑
   */
  private async runSingleAgent(
    agent: WorkflowAgent,
    execution: WorkflowExecution,
    callback?: ExecutionCallback
  ): Promise<string> {
    try {
      agent.status = 'running'
      execution.currentAgent = agent.id

      await callback?.({
        type: 'agent_start',
        agentId: agent.id,
        timestamp: new Date(),
        data: agent,
      })

      // 从注册表获取Agent实例
      const agentInstance = agentRegistry.getAgent(agent.type)
      if (!agentInstance) {
        throw new Error(`未找到类型为 "${agent.type}" 的Agent实现。`)
      }

      const context: AgentExecutionContext = {
        agentId: agent.id,
        workflowId: execution.workflowId,
        variables: execution.variables,
        stepResults: execution.agentResults,
      }

      // 将执行委托给Agent实例
      const agentResult = await agentInstance.execute(agent, context, this.toolManager)

      if (!agentResult.success) {
        throw new Error(agentResult.result)
      }

      agent.status = 'done'
      execution.agentResults[agent.id] = agentResult.result

      await callback?.({
        type: 'agent_completed',
        agentId: agent.id,
        timestamp: new Date(),
        data: { result: agentResult.result },
      })

      return agentResult.result
    } catch (error) {
      agent.status = 'error'
      const errorMessage = error instanceof Error ? error.message : String(error)

      await callback?.({
        type: 'agent_failed',
        agentId: agent.id,
        timestamp: new Date(),
        error: errorMessage,
      })

      throw error
    }
  }

  /**
   * 并行执行多个Agent
   */
  private async runParallelAgents(
    agents: WorkflowAgent[],
    execution: WorkflowExecution,
    callback?: ExecutionCallback
  ): Promise<string[]> {
    const executeAgent = (agent: WorkflowAgent, index: number) =>
      this.runSingleAgent(agent, execution, callback).then(result => ({ result, index }))

    const enableParallel = execution.variables.agentParallel !== false

    if (enableParallel) {
      const parallelResults = await Promise.all(agents.map(executeAgent))
      parallelResults.sort((a, b) => a.index - b.index)
      return parallelResults.map(({ result }) => result)
    } else {
      const results: string[] = []
      for (let i = 0; i < agents.length; i++) {
        const { result } = await executeAgent(agents[i], i)
        results.push(result)
      }
      return results
    }
  }

  /**
   * 执行Agent树
   */
  private async executeAgentTree(
    agentTree: AgentNode,
    execution: WorkflowExecution,
    callback?: ExecutionCallback
  ): Promise<string> {
    let currentTree: AgentNode | undefined = agentTree
    let lastResult = ''

    while (currentTree) {
      if (currentTree.type === 'normal' && currentTree.agent) {
        lastResult = await this.runSingleAgent(currentTree.agent, execution, callback)
        currentTree.result = lastResult
      } else if (currentTree.type === 'parallel' && currentTree.agents) {
        const parallelResults = await this.runParallelAgents(currentTree.agents, execution, callback)
        lastResult = parallelResults.join('\n\n')
        currentTree.result = lastResult
      }
      currentTree = currentTree.nextAgent
    }
    return lastResult
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

  /**
   * 初始化执行状态
   */
  private initializeExecution(
    workflow: WorkflowDefinition,
    contextParams?: Record<string, unknown>
  ): WorkflowExecution {
    return {
      taskId: workflow.taskId,
      workflowId: workflow.taskId,
      status: 'running',
      agentResults: {},
      variables: { ...workflow.variables, ...contextParams },
      startTime: new Date(),
    }
  }
}
