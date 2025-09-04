/**
 * 规划相关的提示词组件
 */

import { ComponentConfig, PromptComponent } from './types'

/**
 * 规划指导原则组件
 */
export const planningGuidelinesComponent: ComponentConfig = {
  id: PromptComponent.PLANNING_GUIDELINES,
  name: 'Planning Guidelines',
  description: 'Planning guidance principles',
  required: false,
  template: `# Planning Guidelines
- Adaptive planning: Single node for simple tasks, multiple nodes for complex tasks only when necessary
- Sequential execution: Break down tasks into logical sequential steps
- Tool utilization: Make use of available tools and capabilities
- Efficient planning: Focus on the most direct path to complete the user's task`,
  fn: async () => {
    return planningGuidelinesComponent.template!
  },
}

/**
 * 规划示例组件
 */
export const planningExamplesComponent: ComponentConfig = {
  id: PromptComponent.PLANNING_EXAMPLES,
  name: 'Planning Examples',
  description: 'Planning examples',
  required: false,
  template: `# Planning Examples
{examples}`,
  fn: async () => {
    // 规划示例将在后续实现
    return undefined
  },
}

/**
 * 输出格式组件
 */
export const outputFormatComponent: ComponentConfig = {
  id: PromptComponent.OUTPUT_FORMAT,
  name: 'Output Format',
  description: 'Output format description',
  required: false,
  template: `# Output Format
{format}`,
  fn: async () => {
    // 输出格式将在后续实现
    return undefined
  },
}
