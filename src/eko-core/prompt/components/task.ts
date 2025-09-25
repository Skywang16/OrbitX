import { ComponentConfig, ComponentContext, PromptComponent } from './types'
import { resolveTemplate } from '../template-engine'

/**
 * 任务上下文组件
 */
export const taskContextComponent: ComponentConfig = {
  id: PromptComponent.TASK_CONTEXT,
  name: 'Task Context',
  description: 'Task context information',
  required: false,
  template: `# Task Context
{taskContext}`,
  fn: async (context: ComponentContext) => {
    const { task, context: agentContext } = context

    if (!task && !agentContext?.chain.taskPrompt) return undefined

    const taskContext = task?.xml || agentContext?.chain.taskPrompt || ''
    if (!taskContext.trim()) return undefined

    const template = taskContextComponent.template!
    return resolveTemplate(template, { taskContext })
  },
}

/**
 * 任务节点组件
 */
export const taskNodesComponent: ComponentConfig = {
  id: PromptComponent.TASK_NODES,
  name: 'Task Nodes',
  description: 'Task node processing description',
  required: false,
  template: `# Task Node Processing
{nodePrompt}`,
  fn: async (context: ComponentContext) => {
    const { task } = context

    if (!task?.xml) return undefined

    return undefined
  },
}

/**
 * 任务示例组件
 */
export const taskExamplesComponent: ComponentConfig = {
  id: PromptComponent.TASK_EXAMPLES,
  name: 'Task Examples',
  description: 'Task processing examples',
  required: false,
  template: `# Task Processing Examples
{examples}`,
  fn: async () => {
    // 这里可以根据任务类型提供相应的示例
    // 暂时返回undefined，后续可以扩展
    return undefined
  },
}
