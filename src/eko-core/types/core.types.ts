import { Agent } from '../agent'
import { LLMs, FinishReason } from './llm.types'
import { IMcpClient } from './mcp.types'
import { ToolResult } from './tools.types'
import { AgentContext } from '../core/context'

export type EkoConfig = {
  llms: LLMs
  agent: Agent
  planLlms?: string[]
  callback?: StreamCallback & HumanCallback
  defaultMcpClient?: IMcpClient
}

export type StreamCallbackMessage = {
  taskId: string
  agentName: string
  nodeId?: string | null
} & (
  | {
      type: 'task'
      streamDone: boolean
      task: Task
    }
  | {
      type: 'agent_start'
      task: Task
    }
  | {
      type: 'text' | 'thinking'
      streamId: string
      streamDone: boolean
      text: string
    }
  | {
      type: 'file'
      mimeType: string
      data: string
    }
  | {
      type: 'tool_streaming'
      toolName: string
      toolId: string
      paramsText: string
    }
  | {
      type: 'tool_use'
      toolName: string
      toolId: string
      params: Record<string, unknown>
    }
  | {
      type: 'tool_result'
      toolName: string
      toolId: string
      params: Record<string, unknown>
      toolResult: ToolResult
    }
  | {
      type: 'agent_result'
      task: Task
      error?: unknown
      result?: string
    }
  | {
      type: 'error'
      error: unknown
    }
  | {
      type: 'finish'
      finishReason: FinishReason
      usage: {
        promptTokens: number
        completionTokens: number
        totalTokens: number
      }
    }
)

export interface StreamCallback {
  onMessage: (message: StreamCallbackMessage, agentContext?: AgentContext) => Promise<void>
}

// Workflow node types removed - using Task node types only

export type TaskTextNode = {
  type: 'normal'
  text: string
}

export type TaskForEachNode = {
  type: 'forEach'
  items: string // list or variable name
  nodes: TaskNode[]
}

export type TaskWatchNode = {
  type: 'watch'
  event: 'dom' | 'gui' | 'file'
  loop: boolean
  description: string
  triggerNodes: (TaskTextNode | TaskForEachNode)[]
}

export type TaskNode = TaskTextNode | TaskForEachNode | TaskWatchNode

// WorkflowAgent and Workflow types removed - using Task architecture only

export type Task = {
  taskId: string
  name: string
  thought: string
  description: string
  nodes: TaskNode[]
  status: 'init' | 'running' | 'done' | 'error'
  xml: string
  modified?: boolean
  taskPrompt?: string
}

export interface HumanCallback {
  onHumanConfirm?: (agentContext: AgentContext, prompt: string, extInfo?: any) => Promise<boolean>
  onHumanInput?: (agentContext: AgentContext, prompt: string, extInfo?: any) => Promise<string>
  onHumanSelect?: (
    agentContext: AgentContext,
    prompt: string,
    options: string[],
    multiple?: boolean,
    extInfo?: any
  ) => Promise<string[]>
  onHumanHelp?: (
    agentContext: AgentContext,
    helpType: 'request_login' | 'request_assistance',
    prompt: string,
    extInfo?: any
  ) => Promise<boolean>
}

export type EkoResult = {
  taskId: string
  success: boolean
  stopReason: 'abort' | 'error' | 'done'
  result: string
  error?: unknown
}
