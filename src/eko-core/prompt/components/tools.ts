/**
 * 工具相关的提示词组件
 */

import { ComponentConfig, ComponentContext, PromptComponent } from './types'
import { resolveTemplate } from '../template-engine'
import { TOOL_NAME as foreach_task } from '../../tools/foreach_task'
import { TOOL_NAME as watch_trigger } from '../../tools/watch_trigger'
import { TOOL_NAME as human_interact } from '../../tools/human_interact'

// 原有的提示词常量
const HUMAN_PROMPT = `
# Human Interaction
When you need to interact with humans, you can use the human interaction tools to:
- Ask for confirmation before executing critical operations
- Request additional information or clarification
- Get user input for decision making
- Provide status updates and progress reports

Always be clear and specific in your human interactions.
`

const FOR_EACH_PROMPT = `
# ForEach Task Processing
You can process multiple items using forEach tasks:
- Use forEach when you need to repeat similar operations on multiple items
- Each iteration can access the current item and index
- Results from previous iterations are available in context
- Break down complex batch operations into manageable forEach tasks
`

const WATCH_PROMPT = `
# Watch and Monitoring
You can set up monitoring and watch triggers:
- Monitor file changes, system events, or external conditions
- Set up automated responses to specific triggers
- Use watch for continuous monitoring scenarios
- Configure appropriate trigger conditions and response actions
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

/**
 * ForEach工具组件
 */
export const foreachToolsComponent: ComponentConfig = {
  id: PromptComponent.FOREACH_TOOLS,
  name: 'ForEach Tools',
  description: 'ForEach task processing tools description',
  required: false,
  dependencies: [PromptComponent.TOOLS_DESCRIPTION],
  template: FOR_EACH_PROMPT,
  fn: async (context: ComponentContext) => {
    const { tools = [] } = context
    const hasForEachTool = tools.some(tool => tool.name === foreach_task)

    return hasForEachTool ? foreachToolsComponent.template! : undefined
  },
}

/**
 * Watch工具组件
 */
export const watchToolsComponent: ComponentConfig = {
  id: PromptComponent.WATCH_TOOLS,
  name: 'Watch Tools',
  description: 'Watch and trigger tools description',
  required: false,
  dependencies: [PromptComponent.TOOLS_DESCRIPTION],
  template: WATCH_PROMPT,
  fn: async (context: ComponentContext) => {
    const { tools = [] } = context
    const hasWatchTool = tools.some(tool => tool.name === watch_trigger)

    return hasWatchTool ? watchToolsComponent.template! : undefined
  },
}
