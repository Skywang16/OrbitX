import { JSONSchema7, NativeLLMToolCall } from '../types'
import { AgentContext } from '../core/context'
import { Tool, ToolResult } from '../types/tools.types'
import { extractAgentXmlNode } from '../common/xml'

export const TOOL_NAME = 'foreach_task'

export default class ForeachTaskTool implements Tool {
  readonly name: string = TOOL_NAME
  readonly description: string
  readonly parameters: JSONSchema7

  constructor() {
    this.description = `When executing the \`forEach\` node, please use the current tool for counting to ensure tasks are executed sequentially, the tool needs to be called with each loop iteration.`
    this.parameters = {
      type: 'object',
      properties: {
        nodeId: {
          type: 'number',
          description: 'forEach node ID.',
        },
        progress: {
          type: 'string',
          description: 'Current execution progress.',
        },
        next_step: {
          type: 'string',
          description: 'Next task description.',
        },
      },
      required: ['nodeId', 'progress', 'next_step'],
    }
  }

  async execute(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    _toolCall?: NativeLLMToolCall
  ): Promise<ToolResult> {
    let nodeId = args.nodeId as string
    let agentXml = agentContext.context.task?.xml || ''
    let node = extractAgentXmlNode(agentXml, `[id="${nodeId}"]`)
    if (node == null) {
      throw new Error('Node ID does not exist: ' + nodeId)
    }
    if (node.tagName !== 'forEach') {
      throw new Error('Node ID is not a forEach node: ' + nodeId)
    }
    let items = node.getAttribute('items')
    let resultText = 'Recorded forEach task execution'
    if (items && items != 'list') {
      resultText = `Executing forEach loop with items: ${items.trim()}`
    }
    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    }
  }
}

export { ForeachTaskTool }
