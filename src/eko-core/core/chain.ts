import { ToolResult } from '../types/tools.types'
import { LLMRequest, NativeLLMToolCall } from '../types/llm.types'

type ChainEvent = {
  type: 'update'
  target: ToolChain
}

interface Callback {
  (chain: Chain, event: ChainEvent): void
}

export class ToolChain {
  readonly toolName: string
  readonly toolCallId: string
  readonly request: LLMRequest
  params?: Record<string, unknown>
  toolResult?: ToolResult
  onUpdate?: () => void

  constructor(toolCall: NativeLLMToolCall, request: LLMRequest) {
    this.toolName = toolCall.name
    this.toolCallId = toolCall.id
    this.request = JSON.parse(JSON.stringify(request))
  }

  updateParams(params: Record<string, unknown>): void {
    this.params = params
    this.onUpdate && this.onUpdate()
  }

  updateToolResult(toolResult: ToolResult): void {
    this.toolResult = toolResult
    this.onUpdate && this.onUpdate()
  }
}

export default class Chain {
  taskPrompt: string
  planRequest?: LLMRequest
  planResult?: string
  tools: ToolChain[] = []
  private listeners: Callback[] = []

  constructor(taskPrompt: string) {
    this.taskPrompt = taskPrompt
  }

  push(tool: ToolChain): void {
    tool.onUpdate = () => {
      this.pub({
        type: 'update',
        target: tool,
      })
    }
    this.tools.push(tool)
    this.pub({
      type: 'update',
      target: tool,
    })
  }

  private pub(event: ChainEvent): void {
    this.listeners.forEach(listener => listener(this, event))
  }

  public addListener(callback: Callback): void {
    this.listeners.push(callback)
  }

  public removeListener(callback: Callback): void {
    this.listeners = this.listeners.filter(listener => listener !== callback)
  }
}
