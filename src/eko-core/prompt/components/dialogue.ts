/**
 * 对话相关的提示词组件
 */

import { ComponentConfig, ComponentContext, PromptComponent } from './types'

/**
 * 对话能力组件
 */
export const dialogueCapabilitiesComponent: ComponentConfig = {
  id: PromptComponent.DIALOGUE_CAPABILITIES,
  name: 'Dialogue Capabilities',
  description: '对话能力描述',
  required: false,
  template: `# Terminal Environment Capabilities
You excel at helping users with:
- File operations and text editing
- Shell command execution and scripting
- Code development and project management
- System administration and automation
- Terminal-based workflows and productivity`,
  fn: async (context: ComponentContext) => {
    return dialogueCapabilitiesComponent.template!
  }
}

/**
 * 对话指导原则组件
 */
export const dialogueGuidelinesComponent: ComponentConfig = {
  id: PromptComponent.DIALOGUE_GUIDELINES,
  name: 'Dialogue Guidelines',
  description: '对话指导原则',
  required: false,
  template: `# Dialogue Guidelines
- Provide clear and helpful responses
- Ask clarifying questions when needed
- Offer practical solutions and examples
- Maintain context throughout the conversation`,
  fn: async (context: ComponentContext) => {
    return dialogueGuidelinesComponent.template!
  }
}
