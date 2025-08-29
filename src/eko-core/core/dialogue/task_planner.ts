import { JSONSchema7 } from 'json-schema'
import { Eko } from '../eko'
import { EkoDialogue } from '../dialogue'
import { DialogueParams, DialogueTool, ToolResult } from '../../types'

export const TOOL_NAME = 'taskPlanner'

export default class TaskPlannerTool implements DialogueTool {
  readonly name: string = TOOL_NAME
  readonly description: string
  readonly parameters: JSONSchema7
  private ekoDialogue: EkoDialogue
  private params: DialogueParams

  constructor(ekoDialogue: EkoDialogue, params: DialogueParams) {
    const agent = ekoDialogue.getConfig().agent
    const agentNames = agent ? agent.Name : 'Unknown'
    this.description = `Used for task planning, this tool is only responsible for generating task plans, not executing them, the following agents are available: ${agentNames}...`
    this.parameters = {
      type: 'object',
      properties: {
        taskDescription: {
          type: 'string',
          description:
            "Task description, Do not omit any information from the user's question, maintain the task as close to the user's input as possible, and use the same language as the user's question.",
        },
        oldTaskId: {
          type: 'string',
          description: 'Previous task ID, modifications based on the previously planned task.',
        },
      },
      required: ['taskDescription'],
    }
    this.params = params
    this.ekoDialogue = ekoDialogue
  }

  async execute(args: Record<string, unknown>): Promise<ToolResult> {
    const taskDescription = args.taskDescription as string
    const oldTaskId = args.oldTaskId as string
    if (oldTaskId) {
      const eko = this.ekoDialogue.getEko(oldTaskId)
      if (eko) {
        // modify the old action plan
        const task = await eko.modify(oldTaskId, taskDescription)
        const taskPlan = task.xml
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify({
                taskId: oldTaskId,
                taskPlan: taskPlan,
              }),
            },
          ],
        }
      }
    }
    // generate a new action plan
    const taskId = this.params.messageId as string
    const eko = new Eko({
      ...this.ekoDialogue.getConfig(),
      callback: this.params.callback?.taskCallback,
    })
    const task = await eko.generate(taskDescription, taskId, this.ekoDialogue.getGlobalContext())
    this.ekoDialogue.addEko(taskId, eko)
    const taskPlan = task.xml
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify({
            taskId: taskId,
            taskPlan: taskPlan,
          }),
        },
      ],
    }
  }
}

export { TaskPlannerTool as ActionPlannerTool }
