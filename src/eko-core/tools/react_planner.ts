import { JSONSchema7 } from '../types'
import { Planner } from '../core/plan'
import { AgentContext } from '../core/context'
import { Tool, ToolResult } from '../types/tools.types'

export const TOOL_NAME = 'react_planner'

export default class ReactPlannerTool implements Tool {
  readonly name = TOOL_NAME
  readonly description =
    'Generate or update a structured execution plan. Use this tool when you need task nodes before continuing execution.'
  readonly parameters: JSONSchema7 = {
    type: 'object',
    properties: {
      prompt: {
        type: 'string',
        description: 'Optional custom planning prompt. Defaults to the original task prompt when omitted.',
      },
      mode: {
        type: 'string',
        enum: ['plan', 'replan'],
        description: 'Select "replan" when refining an existing plan with new context.',
        default: 'plan',
      },
    },
  }
  readonly planDescription =
    'Calls the planning LLM to create or refresh task structure and nodes; invoke when a structured plan would help the next actions.'
  readonly noPlan = true

  async execute(args: Record<string, unknown>, agentContext: AgentContext): Promise<ToolResult> {
    const context = agentContext.context
    const planner = new Planner(context)

    const rawPrompt =
      typeof args.prompt === 'string' && args.prompt.trim().length > 0 ? args.prompt : context.chain.taskPrompt
    const planningPrompt = rawPrompt || context.task?.description || context.task?.taskPrompt || ''
    const mode = typeof args.mode === 'string' ? args.mode : 'plan'

    const task =
      mode === 'replan' && context.chain.planRequest && context.chain.planResult
        ? await planner.replan(planningPrompt)
        : await planner.plan(planningPrompt)

    context.chain.taskPrompt = planningPrompt
    context.task = task
    context.task.status = 'running'

    return {
      content: [
        {
          type: 'text',
          text: `Plan ${mode === 'replan' ? 'updated' : 'created'} with ${task.nodes.length} step(s).`,
        },
      ],
    }
  }
}

export { ReactPlannerTool }
