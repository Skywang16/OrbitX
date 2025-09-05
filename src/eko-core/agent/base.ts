import config from '../config'
import Log from '../common/log'
import { extractUsedTool, handleLargeContextMessages } from '../memory'
import { RetryLanguageModel } from '../llm'
import { ToolWrapper } from '../tools/wrapper'
import { ToolChain } from '../core/chain'
import Context, { AgentContext, generateNodeId } from '../core/context'
import { ForeachTaskTool, McpTool, WatchTriggerTool } from '../tools'
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
import { callAgentLLM, convertTools, getTool, convertToolResult, removeDuplicateToolUse } from './llm'
import { doTaskResultCheck } from '../tools/task_result_check'
import { getAgentSystemPrompt, getAgentUserPrompt } from '../prompt'
import { doTodoListManager } from '../tools/todo_list_manager'

export type AgentParams = {
  name: string
  description: string
  tools: Tool[]
  llms?: string[]
  mcpClient?: IMcpClient
  planDescription?: string
  requestHandler?: (request: LLMRequest) => void
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

    // 设置执行阶段的nodeId
    context.currentNodeId = generateNodeId(context.taskId, 'execution')
    const tools = [...this.tools, ...this.system_auto_tools(task)]
    const systemPrompt = await this.buildSystemPrompt(agentContext, tools)
    const userPrompt = await this.buildUserPrompt(agentContext, tools)
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
    let agentTools = tools
    while (loopNum < maxReactNum) {
      await context.checkAborted()
      if (mcpClient) {
        const controlMcp = await this.controlMcpTools(agentContext, messages, loopNum)
        if (controlMcp.mcpTools) {
          const mcpTools = await this.listTools(context, mcpClient, task, controlMcp.mcpParams)
          const usedTools: Tool[] = extractUsedTool(messages, agentTools)
          const _agentTools = mergeTools(tools, usedTools)
          agentTools = mergeTools(_agentTools, mcpTools)
        }
      }
      await this.handleMessages(agentContext, messages, tools)
      const llm_tools = convertTools(agentTools)
      const results = await callAgentLLM(
        agentContext,
        rlm,
        messages,
        llm_tools,
        false,
        undefined,
        0,
        this.callback,
        this.requestHandler
      )
      // Force stop functionality removed with variable storage
      const finalResult = await this.handleCallResult(agentContext, messages, agentTools, results)
      loopNum++
      if (!finalResult) {
        if (config.expertMode && loopNum % config.expertModeTodoLoopNum == 0) {
          await doTodoListManager(agentContext, rlm, messages, llm_tools)
        }
        continue
      }
      if (config.expertMode && checkNum == 0) {
        checkNum++
        const { completionStatus } = await doTaskResultCheck(agentContext, rlm, messages, llm_tools)
        if (completionStatus === 'incomplete') {
          continue
        }
      }
      return finalResult
    }
    return 'Unfinished'
  }

  protected async handleCallResult(
    agentContext: AgentContext,
    messages: NativeLLMMessage[],
    agentTools: Tool[],
    results: Array<{ type: 'text'; text: string } | NativeLLMToolCall>
  ): Promise<string | null> {
    let text: string | null = null
    let context = agentContext.context
    let toolResults: NativeLLMMessagePart[] = []
    results = removeDuplicateToolUse(results)
    if (results.length == 0) {
      return null
    }

    // Separate text and tool calls
    const textResults = results.filter(r => 'type' in r && r.type === 'text') as Array<{ type: 'text'; text: string }>
    const toolCallResults = results.filter(r => 'id' in r) as NativeLLMToolCall[]

    // Handle text results
    if (textResults.length > 0) {
      text = textResults.map(r => r.text).join('')
    }

    // Handle tool calls
    for (let i = 0; i < toolCallResults.length; i++) {
      let result = toolCallResults[i]
      let toolResult: ToolResult
      // Create compatibility adapter for ToolChain
      const toolCallAdapter = {
        type: 'tool-call' as const,
        toolCallId: result.id,
        toolName: result.name,
        input: result.arguments,
      }
      let toolChain = new ToolChain(toolCallAdapter as any, agentContext.context.chain.planRequest as LLMRequest)
      agentContext.context.chain.push(toolChain)
      try {
        let args = result.arguments || {}
        toolChain.params = args
        let tool = getTool(agentTools, result.name)
        if (!tool) {
          throw new Error(result.name + ' tool does not exist')
        }
        toolResult = await tool.execute(args, agentContext, result)
        toolChain.updateToolResult(toolResult)
        agentContext.consecutiveErrorNum = 0
      } catch (e) {
        Log.error('tool call error: ', result.name, result.arguments, e)
        toolResult = {
          content: [
            {
              type: 'text',
              text: e + '',
            },
          ],
          isError: true,
        }
        toolChain.updateToolResult(toolResult)
        if (++agentContext.consecutiveErrorNum >= 10) {
          throw e
        }
      }
      const callback = this.callback || context.config.callback
      if (callback) {
        await callback.onMessage(
          {
            taskId: context.taskId,
            agentName: agentContext.agent.Name,
            nodeId: agentContext.context.taskId,
            type: 'tool_result',
            toolId: result.id,
            toolName: result.name,
            params: result.arguments,
            toolResult: toolResult,
          },
          agentContext
        )
      }
      const llmToolResult = convertToolResult(result, toolResult)
      toolResults.push(llmToolResult)
    }

    // Add assistant message with results
    if (textResults.length > 0 || toolCallResults.length > 0) {
      const assistantContent: NativeLLMMessagePart[] = []

      if (text) {
        assistantContent.push({ type: 'text', text })
      }

      toolCallResults.forEach(toolCall => {
        assistantContent.push({
          type: 'tool-call',
          toolCallId: toolCall.id,
          toolName: toolCall.name,
          args: toolCall.arguments,
        })
      })

      messages.push({
        role: 'assistant',
        content: assistantContent,
      })
    }

    if (toolResults.length > 0) {
      messages.push({
        role: 'tool',
        content: toolResults,
      })
      return null
    } else {
      return text
    }
  }

  protected system_auto_tools(task?: Task): Tool[] {
    let tools: Tool[] = []
    if (!task) return tools

    let taskXml = task.xml

    let hasForeach = taskXml.indexOf('</forEach>') > -1
    if (hasForeach) {
      tools.push(new ForeachTaskTool())
    }
    let hasWatch = taskXml.indexOf('</watch>') > -1
    if (hasWatch) {
      tools.push(new WatchTriggerTool())
    }
    let toolNames = this.tools.map(tool => tool.name)
    return tools.filter(tool => toolNames.indexOf(tool.name) == -1)
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
      Log.error('Mcp listTools error', e)
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

  protected async callInnerTool(fun: () => Promise<any>): Promise<ToolResult> {
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
