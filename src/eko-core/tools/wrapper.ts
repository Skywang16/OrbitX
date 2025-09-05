import { ToolResult, ToolExecuter, ToolSchema, NativeLLMTool, NativeLLMToolCall } from '../types'
import { convertToolSchema } from '../common/utils'
import { AgentContext } from '../core/context'

export class ToolWrapper {
  private tool: NativeLLMTool
  private execute: ToolExecuter

  constructor(toolSchema: ToolSchema, execute: ToolExecuter) {
    this.tool = convertToolSchema(toolSchema)
    this.execute = execute
  }

  get name(): string {
    return this.tool.name
  }

  getTool(): NativeLLMTool {
    return this.tool
  }

  async callTool(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: NativeLLMToolCall
  ): Promise<ToolResult> {
    return await this.execute.execute(args, agentContext, toolCall)
  }
}
