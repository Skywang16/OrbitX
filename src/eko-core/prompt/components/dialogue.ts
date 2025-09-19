/**
 * 对话相关的提示词组件
 */

import { ComponentConfig, PromptComponent } from './types'

/**
 * 对话能力组件
 */
export const dialogueCapabilitiesComponent: ComponentConfig = {
  id: PromptComponent.DIALOGUE_CAPABILITIES,
  name: 'Dialogue Capabilities',
  description: 'Dialogue capabilities description',
  required: false,
  template: `# Terminal Environment Capabilities
You excel at helping users with:
- File operations and text editing
- Shell command execution and scripting
- Code development and project management
- System administration and automation
- Terminal-based workflows and productivity`,
  fn: async () => {
    return dialogueCapabilitiesComponent.template!
  },
}

/**
 * 对话指导原则组件
 */
export const dialogueGuidelinesComponent: ComponentConfig = {
  id: PromptComponent.DIALOGUE_GUIDELINES,
  name: 'Dialogue Guidelines',
  description: 'Dialogue guidance principles',
  required: false,
  template: `# Dialogue Guidelines
- Provide clear and helpful responses
- Ask clarifying questions when needed
- Offer practical solutions and examples
- Maintain context throughout the conversation

## Safety & Scope Confirmation
- If the user's current working directory or requested operation scope appears overly broad or system-managed (e.g., '/', '/Users', '/Users/<name>', '/home', '/var', '/etc', '/Library', OS media/library folders like 'Music Library.musiclibrary'), do not proceed with broad file operations.
- First, ask for confirmation or a narrower subpath. For example, request a specific project folder rather than scanning the entire home directory.
- Downstream in the agent execution stage, this translates to calling a human confirmation interaction (e.g., a confirm/select step) before any wide directory listing or potentially sensitive actions.`,
  fn: async () => {
    return dialogueGuidelinesComponent.template!
  },
}
