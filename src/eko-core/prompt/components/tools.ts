/**
 * 工具相关的提示词组件
 */

import { ComponentConfig, ComponentContext, PromptComponent } from './types'
import { resolveTemplate } from '../template-engine'

// Human Interaction component removed: handled by per-tool UI confirmation, not a standalone tool

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

// Human Interaction prompt component removed
