import { JSONSchema7, NativeLLMToolCall } from '../types'
import { ToolWrapper } from './wrapper'
import { AgentContext } from '../core/context'
import HumanInteractTool from './human_interact'
import TaskNodeStatusTool from './task_node_status'
import ReactPlannerTool from './react_planner'
import NewTaskTool from './new_task'
import ReplanSubtreeTool from './replan_subtree'
import TaskTreeEditTool from './task_tree_edit'
import { Tool, ToolResult } from '../types/tools.types'

export class McpTool implements Tool {
  readonly name: string
  readonly description?: string
  readonly parameters: JSONSchema7
  private toolWrapper: ToolWrapper

  constructor(toolWrapper: ToolWrapper) {
    this.toolWrapper = toolWrapper
    this.name = toolWrapper.name
    this.description = toolWrapper.getTool().description
    this.parameters = toolWrapper.getTool().parameters
  }

  async execute(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: NativeLLMToolCall
  ): Promise<ToolResult> {
    return this.toolWrapper.callTool(args, agentContext, toolCall)
  }
}

export { HumanInteractTool, TaskNodeStatusTool, ReactPlannerTool, NewTaskTool, ReplanSubtreeTool, TaskTreeEditTool }
