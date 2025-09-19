import config from '../config'
import Log from '../common/log'
import { extractUsedTool, handleLargeContextMessages } from '../memory'
import { RetryLanguageModel } from '../llm'
import { ToolWrapper } from '../tools/wrapper'
import { ToolChain } from '../core/chain'
import Context, { AgentContext, generateNodeId } from '../core/context'
import { ForeachTaskTool, McpTool, WatchTriggerTool, ReactPlannerTool } from '../tools'
import { mergeTools } from '../common/utils'
import {
  Task,
  IMcpClient,
  LLMRequest,
  Tool,
  ToolExecuter,
  ToolResult,
  ToolSchema,
  StreamCallback,
  HumanCallback,
  NativeLLMMessage,
  NativeLLMMessagePart,
  NativeLLMToolCall,
} from '../types'
import {
  callAgentLLM,
  convertTools,
  getTool,
  convertToolResult,
  removeDuplicateToolUse,
  AgentLLMCallResult,
} from './llm'
import { doTaskResultCheck } from '../tools/task_result_check'
import { TOOL_NAME as task_node_status } from '../tools/task_node_status'
import { getAgentSystemPrompt, getAgentUserPrompt } from '../prompt'
import { doTodoListManager } from '../tools/todo_list_manager'
import { ReactIteration } from '../react/types'

export type AgentParams = {
  name: string
  description: string
  tools: Tool[]
  llms?: string[]
  mcpClient?: IMcpClient
  planDescription?: string
  requestHandler?: (request: LLMRequest) => void
}

type ReActIterationOutcome = {
  finalText?: string
  toolExecuted: boolean
  hadError: boolean
}

export class Agent {
  protected name: string
  protected description: string
  protected tools: Tool[] = []
  protected llms?: string[]
  protected mcpClient?: IMcpClient
  protected planDescription?: string
  protected requestHandler?: (request: LLMRequest) => void
  protected callback?: StreamCallback & HumanCallback
  protected agentContext?: AgentContext

  constructor(params: AgentParams) {
    this.name = params.name
    this.description = params.description
    this.tools = params.tools
    this.llms = params.llms
    this.mcpClient = params.mcpClient
    this.planDescription = params.planDescription
    this.requestHandler = params.requestHandler
  }

  public async run(context: Context): Promise<string> {
    let mcpClient = this.mcpClient || context.config.defaultMcpClient
    let agentContext = new AgentContext(context, this)
    try {
      this.agentContext = agentContext
      mcpClient && !mcpClient.isConnected() && (await mcpClient.connect(context.controller.signal))

      return this.runWithContext(agentContext, mcpClient, config.maxReactNum, [])
    } finally {
      mcpClient && (await mcpClient.close())
    }
  }

  public async runWithContext(
    agentContext: AgentContext,
    mcpClient?: IMcpClient,
    maxReactNum: number = 100,
    historyMessages: NativeLLMMessage[] = []
  ): Promise<string> {
    let loopNum = 0
    let checkNum = 0
    this.agentContext = agentContext
    const context = agentContext.context
    const task = context.task
    const runtime = context.reactRuntime

    context.currentNodeId = generateNodeId(context.taskId, 'execution')
    const staticTools = [...this.tools]
    const baseTools = mergeTools(staticTools, this.system_auto_tools(task))
    const systemPrompt = await this.buildSystemPrompt(agentContext, baseTools)
    const userPrompt = await this.buildUserPrompt(agentContext, baseTools)
    const messages: NativeLLMMessage[] = [
      {
        role: 'system',
        content: systemPrompt,
      },
      ...historyMessages,
      {
        role: 'user',
        content: userPrompt,
      },
    ]
    agentContext.messages = messages
    const rlm = new RetryLanguageModel(context.config.llms, this.llms)
    let agentTools = baseTools

    while (loopNum < maxReactNum) {
      await context.checkAborted()

      if (runtime.shouldHalt()) {
        throw new Error('ReAct loop halted by runtime guard')
      }

      const latestBaseTools = mergeTools(staticTools, this.system_auto_tools(context.task))
      agentTools = latestBaseTools

      if (mcpClient) {
        const controlMcp = await this.controlMcpTools(agentContext, messages, loopNum)
        if (controlMcp.mcpTools) {
          const mcpTools = await this.listTools(context, mcpClient, context.task, controlMcp.mcpParams)
          const usedTools: Tool[] = extractUsedTool(messages, agentTools)
          const mergedTools = mergeTools(latestBaseTools, mergeTools(usedTools, mcpTools))
          agentTools = mergedTools
        }
      }

      await this.handleMessages(agentContext, messages, latestBaseTools)
      const llmTools = convertTools(agentTools)
      const iteration = runtime.startIteration()

      const llmOutput = await callAgentLLM(
        agentContext,
        rlm,
        messages,
        llmTools,
        false,
        undefined,
        0,
        this.callback,
        this.requestHandler
      )

      if (llmOutput.rawText) {
        const normalizedThought = llmOutput.thinkingText || llmOutput.rawText
        runtime.recordThought(iteration, llmOutput.rawText, normalizedThought)
      }

      const outcome = await this.handleCallResult(agentContext, messages, agentTools, llmOutput, iteration)
      loopNum++

      if (outcome.finalText) {
        const finalText = outcome.finalText.trim()
        if (!finalText) {
          runtime.markIdleRound()
          continue
        }

        if (config.expertMode && checkNum === 0) {
          checkNum++
          const { completionStatus } = await doTaskResultCheck(agentContext, rlm, messages, llmTools)
          if (completionStatus === 'incomplete') {
            runtime.markIdleRound()
            continue
          }
        }

        runtime.completeIteration(iteration, finalText, llmOutput.finishReason)
        if (!llmOutput.finishReason) {
          runtime.setStopReason('stop')
        }
        return finalText
      }

      if (outcome.toolExecuted) {
        if (config.expertMode && loopNum % config.expertModeTodoLoopNum === 0) {
          await doTodoListManager(agentContext, rlm, messages, llmTools)
        }

        if (outcome.hadError && runtime.shouldHalt()) {
          throw new Error('ReAct loop halted after repeated tool failures')
        }

        continue
      }

      runtime.markIdleRound()
    }

    runtime.setStopReason('length')
    return runtime.getSnapshot().finalResponse || 'Unfinished'
  }

  protected async handleCallResult(
    agentContext: AgentContext,
    messages: NativeLLMMessage[],
    agentTools: Tool[],
    llmOutput: AgentLLMCallResult,
    iteration: ReactIteration
  ): Promise<ReActIterationOutcome> {
    const context = agentContext.context
    const runtime = context.reactRuntime
    const assistantContent: NativeLLMMessagePart[] = []
    const toolResults: NativeLLMMessagePart[] = []
    const toolCalls = removeDuplicateToolUse(llmOutput.toolCalls)

    if (llmOutput.rawText) {
      assistantContent.push({ type: 'text', text: llmOutput.rawText })
    }

    toolCalls.forEach(toolCall => {
      assistantContent.push({
        type: 'tool-call',
        toolCallId: toolCall.id,
        toolName: toolCall.name,
        args: toolCall.arguments,
      })
    })

    if (assistantContent.length > 0) {
      messages.push({
        role: 'assistant',
        content: assistantContent,
      })
    }

    if (toolCalls.length === 0) {
      const finalText = llmOutput.responseText || llmOutput.rawText
      return {
        finalText: finalText || undefined,
        toolExecuted: false,
        hadError: false,
      }
    }

    if (toolCalls.length > 1) {
      Log.warn('Multiple tool calls detected in a single ReAct iteration; executing sequentially.')
    }

    let hadError = false

    for (const toolCall of toolCalls) {
      const nativeToolCall: NativeLLMToolCall = {
        id: toolCall.id,
        name: toolCall.name,
        arguments: toolCall.arguments,
      }
      const toolChain = new ToolChain(nativeToolCall, agentContext.context.chain.planRequest as LLMRequest)
      agentContext.context.chain.push(toolChain)
      const args = toolCall.arguments || {}
      toolChain.params = args
      runtime.recordAction(iteration, toolCall.name, args)

      let toolResult: ToolResult
      try {
        const tool = getTool(agentTools, toolCall.name)
        if (!tool) {
          throw new Error(`${toolCall.name} tool does not exist`)
        }

        Log.warn(`[工具执行] ${toolCall.name}`)
        try {
          Log.warn('输入参数: ' + JSON.stringify(args, null, 2))
        } catch (_e) {
          Log.warn('输入参数: [unserializable]')
        }
        toolResult = await tool.execute(args, agentContext, nativeToolCall)
        // Persist task node status updates back to context
        if (toolCall.name === task_node_status) {
          try {
            const firstPart = toolResult.content[0] as
              | { type: 'text'; text: string }
              | { type: 'image'; data: string; mimeType?: string }
              | undefined
            if (firstPart && firstPart.type === 'text' && agentContext.context.task) {
              agentContext.context.task.xml = firstPart.text
            }
            // Update current node pointer to the next todo id if provided
            const todoIdsRaw = (args as Record<string, unknown>)['todoIds']
            const todoIds = Array.isArray(todoIdsRaw) ? todoIdsRaw : undefined
            const nextId = todoIds && typeof todoIds[0] === 'string' ? (todoIds[0] as string) : undefined
            if (typeof nextId === 'string' && nextId.startsWith('node-')) {
              const idxStr = nextId.substring('node-'.length)
              const idx = parseInt(idxStr, 10)
              if (!Number.isNaN(idx)) {
                agentContext.context.currentNodeId = generateNodeId(agentContext.context.taskId, 'execution', idx)
              }
            }
          } catch (_e) {
            // non-fatal; keep going
          }
        }
        toolChain.updateToolResult(toolResult)

        runtime.recordObservation(iteration, toolCall.name, toolResult)

        try {
          Log.warn(`[工具输出] ${toolCall.name}`)
          Log.warn('输出结果: ' + JSON.stringify(toolResult, null, 2))
        } catch (_e) {
          Log.warn('输出结果: [unserializable]')
        }

        if (toolResult.isError) {
          const errorText = toolResult.content[0]?.type === 'text' ? toolResult.content[0].text : 'Unknown error'
          agentContext.consecutiveErrorNum++
          hadError = true
          runtime.failIteration(iteration, `Tool ${toolCall.name} failed: ${errorText}`)
          if (agentContext.consecutiveErrorNum >= 5) {
            throw new Error(`Tool ${toolCall.name} failed repeatedly: ${errorText}`)
          }
        } else {
          agentContext.consecutiveErrorNum = 0
          runtime.resetErrorCounter()
        }
      } catch (e) {
        Log.error(
          'tool call system error: ',
          toolCall.name,
          toolCall.arguments as Record<string, unknown>,
          e instanceof Error ? e : String(e)
        )
        const errorMessage = e instanceof Error ? e.message : String(e)
        toolResult = {
          content: [
            {
              type: 'text',
              text: errorMessage,
            },
          ],
          isError: true,
        }
        toolChain.updateToolResult(toolResult)
        runtime.recordObservation(iteration, toolCall.name, toolResult)
        agentContext.consecutiveErrorNum++
        hadError = true
        runtime.failIteration(iteration, errorMessage)
        if (agentContext.consecutiveErrorNum >= 5) {
          throw e
        }
      }

      const callback = this.callback || context.config.callback
      if (callback) {
        await callback.onMessage(
          {
            taskId: context.taskId,
            agentName: agentContext.agent.Name,
            nodeId: context.currentNodeId || generateNodeId(context.taskId, 'execution'),
            type: 'tool_result',
            toolId: toolCall.id,
            toolName: toolCall.name,
            params: toolCall.arguments,
            toolResult,
          },
          agentContext
        )
      }

      const llmToolResult = convertToolResult(toolCall, toolResult)
      toolResults.push(llmToolResult)
    }

    if (toolResults.length > 0) {
      messages.push({
        role: 'tool',
        content: toolResults,
      })
    }

    return {
      toolExecuted: true,
      hadError,
    }
  }

  protected system_auto_tools(task?: Task): Tool[] {
    const autoTools: Tool[] = [new ReactPlannerTool()]
    let taskXml = task?.xml || ''

    let hasForeach = taskXml.indexOf('</forEach>') > -1
    if (hasForeach) {
      autoTools.push(new ForeachTaskTool())
    }
    let hasWatch = taskXml.indexOf('</watch>') > -1
    if (hasWatch) {
      autoTools.push(new WatchTriggerTool())
    }
    let existingNames = this.tools.map(tool => tool.name)
    return autoTools.filter(tool => existingNames.indexOf(tool.name) === -1)
  }

  protected async buildSystemPrompt(agentContext: AgentContext, tools: Tool[]): Promise<string> {
    return await getAgentSystemPrompt(
      this,
      agentContext.context.task,
      agentContext.context,
      tools,
      await this.extSysPrompt(agentContext, tools)
    )
  }

  protected async buildUserPrompt(agentContext: AgentContext, tools: Tool[]): Promise<string> {
    return getAgentUserPrompt(this, agentContext.context.task, agentContext.context, tools)
  }

  protected async extSysPrompt(_agentContext: AgentContext, _tools: Tool[]): Promise<string> {
    return ''
  }

  private async listTools(
    context: Context,
    mcpClient: IMcpClient,
    task?: Task,
    mcpParams?: Record<string, unknown>
  ): Promise<Tool[]> {
    try {
      if (!mcpClient.isConnected()) {
        await mcpClient.connect(context.controller.signal)
      }
      let list = await mcpClient.listTools(
        {
          taskId: context.taskId,
          nodeId: task?.taskId,
          environment: config.platform,
          agent_name: this.name,
          params: {},
          prompt: task?.description || context.chain.taskPrompt,
          ...(mcpParams || {}),
        },
        context.controller.signal
      )
      let mcpTools: Tool[] = []
      for (let i = 0; i < list.length; i++) {
        let toolSchema: ToolSchema = list[i]
        let execute = this.toolExecuter(mcpClient, toolSchema.name)
        let toolWrapper = new ToolWrapper(toolSchema, execute)
        mcpTools.push(new McpTool(toolWrapper))
      }
      return mcpTools
    } catch (e) {
      Log.error('Mcp listTools error', e instanceof Error ? e : String(e))
      return []
    }
  }

  protected async controlMcpTools(
    _agentContext: AgentContext,
    _messages: NativeLLMMessage[],
    _loopNum: number
  ): Promise<{
    mcpTools: boolean
    mcpParams?: Record<string, unknown>
  }> {
    return {
      mcpTools: _loopNum == 0,
    }
  }

  protected toolExecuter(mcpClient: IMcpClient, name: string): ToolExecuter {
    return {
      execute: async function (args, agentContext): Promise<ToolResult> {
        return await mcpClient.callTool(
          {
            name: name,
            arguments: args,
            extInfo: {
              taskId: agentContext.context.taskId,
              nodeId: agentContext.context.taskId,
              environment: config.platform,
              agent_name: agentContext.agent.Name,
            },
          },
          agentContext.context.controller.signal
        )
      },
    }
  }

  protected async handleMessages(
    _agentContext: AgentContext,
    _messages: NativeLLMMessage[],
    _tools: Tool[]
  ): Promise<void> {
    // Only keep the last image / file, large tool-text-result
    handleLargeContextMessages(_messages)
  }

  protected async callInnerTool(fun: () => Promise<unknown>): Promise<ToolResult> {
    let result = await fun()
    return {
      content: [
        {
          type: 'text',
          text: result ? (typeof result == 'string' ? result : JSON.stringify(result)) : 'Successful',
        },
      ],
    }
  }

  public async loadTools(context: Context): Promise<Tool[]> {
    if (this.mcpClient) {
      let mcpTools = await this.listTools(context, this.mcpClient, context.task)
      if (mcpTools && mcpTools.length > 0) {
        return mergeTools(this.tools, mcpTools)
      }
    }
    return this.tools
  }

  public addTool(tool: Tool) {
    this.tools.push(tool)
  }

  protected async onTaskStatus(_status: 'pause' | 'abort' | 'resume-pause', _reason?: string) {
    // Task status handling - variables removed
  }

  get Llms(): string[] | undefined {
    return this.llms
  }

  get Name(): string {
    return this.name
  }

  get Description(): string {
    return this.description
  }

  get Tools(): Tool[] {
    return this.tools
  }

  get PlanDescription() {
    return this.planDescription
  }

  get McpClient() {
    return this.mcpClient
  }

  get AgentContext(): AgentContext | undefined {
    return this.agentContext
  }
}
