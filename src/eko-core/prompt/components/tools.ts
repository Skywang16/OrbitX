/**
 * 工具相关的提示词组件
 */

import { ComponentConfig, ComponentContext, PromptComponent } from './types'
import { resolveTemplate } from '../template-engine'
import { TOOL_NAME as human_interact } from '../../tools/human_interact'

const HUMAN_PROMPT = `
# Human Interaction
When you need to interact with humans, you can use the human interaction tools to:
- Ask for confirmation before executing critical operations
- Request additional information or clarification
- Get user input for decision making
- Provide status updates and progress reports

Always be clear and specific in your human interactions.
`

/**
 * 工具描述组件
 */
export const toolsDescriptionComponent: ComponentConfig = {
  id: PromptComponent.TOOLS_DESCRIPTION,
  name: 'Tools Description',
  description: 'Basic description of tools',
  required: false,
  template: `# Available Tools
{toolsList}`,
  fn: async (context: ComponentContext) => {
    const { tools = [] } = context

    if (tools.length === 0) return undefined

    const toolsList = tools.map(tool => `- ${tool.name}: ${tool.description || ''}`).join('\n')

    const template = toolsDescriptionComponent.template!
    return resolveTemplate(template, { toolsList })
  },
}

/**
 * 人机交互工具组件
 */
export const humanInteractionComponent: ComponentConfig = {
  id: PromptComponent.HUMAN_INTERACTION,
  name: 'Human Interaction',
  description: 'Human interaction tools description',
  required: false,
  template: HUMAN_PROMPT,
  fn: async (context: ComponentContext) => {
    const { tools = [] } = context
    const hasHumanTool = tools.some(tool => tool.name === human_interact)

    return hasHumanTool ? humanInteractionComponent.template! : undefined
  },
}
