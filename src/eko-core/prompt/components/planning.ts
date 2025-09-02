/**
 * 规划相关的提示词组件
 */

import { ComponentConfig, ComponentContext, PromptComponent } from './types'

/**
 * 规划指导原则组件
 */
export const planningGuidelinesComponent: ComponentConfig = {
  id: PromptComponent.PLANNING_GUIDELINES,
  name: 'Planning Guidelines',
  description: '规划指导原则',
  required: false,
  template: `# Planning Guidelines
- Sequential execution: Break down the task into logical sequential steps
- Tool utilization: Make use of available tools and capabilities
- Context preservation: Each step can reference results from previous steps
- Efficient planning: Focus on the most direct path to complete the user's task`,
  fn: async (context: ComponentContext) => {
    return planningGuidelinesComponent.template!
  }
}

/**
 * 规划示例组件
 */
export const planningExamplesComponent: ComponentConfig = {
  id: PromptComponent.PLANNING_EXAMPLES,
  name: 'Planning Examples',
  description: '规划示例',
  required: false,
  template: `# Planning Examples
{examples}`,
  fn: async (context: ComponentContext) => {
    // 规划示例将在后续实现
    return undefined
  }
}

/**
 * 输出格式组件
 */
export const outputFormatComponent: ComponentConfig = {
  id: PromptComponent.OUTPUT_FORMAT,
  name: 'Output Format',
  description: '输出格式说明',
  required: false,
  template: `# Output Format
{format}`,
  fn: async (context: ComponentContext) => {
    // 输出格式将在后续实现
    return undefined
  }
}
