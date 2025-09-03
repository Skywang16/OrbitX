import config from '../config'
import Log from '../common/log'
import { extractUsedTool, removeDuplicateToolUse, handleLargeContextMessages } from '../memory'
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
} from '../types'
import {
  LanguageModelV2FilePart,
  LanguageModelV2Prompt,
  LanguageModelV2TextPart,
  LanguageModelV2ToolCallPart,
  LanguageModelV2ToolResultPart,
} from '@ai-sdk/provider'
import { callAgentLLM, convertTools, getTool, convertToolResult, defaultMessageProviderOptions } from './llm'
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
    historyMessages: LanguageModelV2Prompt = []
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
    const messages: LanguageModelV2Prompt = [
      {
        role: 'system',
        content: systemPrompt,
        providerOptions: defaultMessageProviderOptions(),
      },
      ...historyMessages,
      {
        role: 'user',
        content: userPrompt,
        providerOptions: defaultMessageProviderOptions(),
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
          const usedTools = extractUsedTool(messages, agentTools)
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
        if (completionStatus == 'incomplete') {
          continue
        }
      }
      return finalResult
    }
    return 'Unfinished'
  }

  protected async handleCallResult(
    agentContext: AgentContext,
    messages: LanguageModelV2Prompt,
    agentTools: Tool[],
    results: Array<LanguageModelV2TextPart | LanguageModelV2ToolCallPart>
  ): Promise<string | null> {
    let text: string | null = null
    let context = agentContext.context
    let user_messages: LanguageModelV2Prompt = []
    let toolResults: LanguageModelV2ToolResultPart[] = []
    results = removeDuplicateToolUse(results)
    if (results.length == 0) {
      return null
    }
    for (let i = 0; i < results.length; i++) {
      let result = results[i]
      if (result.type == 'text') {
        text = result.text
        continue
      }
      let toolResult: ToolResult
      let toolChain = new ToolChain(result, agentContext.context.chain.planRequest as LLMRequest)
      agentContext.context.chain.push(toolChain)
      try {
        let args = typeof result.input == 'string' ? JSON.parse(result.input || '{}') : result.input || {}
        toolChain.params = args
        let tool = getTool(agentTools, result.toolName)
        if (!tool) {
          throw new Error(result.toolName + ' tool does not exist')
        }
        toolResult = await tool.execute(args, agentContext, result)
        toolChain.updateToolResult(toolResult)
        agentContext.consecutiveErrorNum = 0
      } catch (e) {
        Log.error('tool call error: ', result.toolName, result.input, e)
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
            toolId: result.toolCallId,
            toolName: result.toolName,
            params: (result.input as Record<string, unknown>) || {},
            toolResult: toolResult,
          },
          agentContext
        )
      }
      const llmToolResult = convertToolResult(result, toolResult, user_messages)
      toolResults.push(llmToolResult)
    }
    messages.push({
      role: 'assistant',
      content: results,
    })
    if (toolResults.length > 0) {
      messages.push({
        role: 'tool',
        content: toolResults,
      })
      user_messages.forEach(message => messages.push(message))
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

  protected async buildUserPrompt(
    agentContext: AgentContext,
    tools: Tool[]
  ): Promise<Array<LanguageModelV2TextPart | LanguageModelV2FilePart>> {
    return [
      {
        type: 'text',
        text: getAgentUserPrompt(this, agentContext.context.task, agentContext.context, tools),
      },
    ]
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
    _messages: LanguageModelV2Prompt,
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
    messages: LanguageModelV2Prompt,
    _tools: Tool[]
  ): Promise<void> {
    // Only keep the last image / file, large tool-text-result
    handleLargeContextMessages(messages)
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
