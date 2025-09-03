/**
 * 任务相关的提示词组件
 */

import { ComponentConfig, ComponentContext, PromptComponent } from './types'
import { resolveTemplate } from '../template-engine'

// 原有的节点提示词常量
const FOR_EACH_NODE = `
## ForEach Node Processing
When processing forEach nodes:
- Each iteration processes one item from the specified list
- Use the current item context in your operations
- Maintain state between iterations when needed
- Report progress for each iteration
`

const WATCH_NODE = `
## Watch Node Processing
When processing watch nodes:
- Set up monitoring for the specified conditions
- Define clear trigger criteria
- Implement appropriate response actions
- Handle both one-time and continuous monitoring scenarios
`

/**
 * 任务上下文组件
 */
export const taskContextComponent: ComponentConfig = {
  id: PromptComponent.TASK_CONTEXT,
  name: 'Task Context',
  description: '任务上下文信息',
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
  description: '任务节点处理说明',
  required: false,
  template: `# Task Node Processing
{nodePrompt}`,
  fn: async (context: ComponentContext) => {
    const { task, tools = [] } = context

    if (!task?.xml) return undefined

    let nodePrompt = ''
    const taskXml = task.xml
    const hasForEachNode = taskXml.indexOf('</forEach>') > -1
    const hasWatchNode = taskXml.indexOf('</watch>') > -1

    if (hasForEachNode && tools.some(tool => tool.name === 'foreach_task')) {
      nodePrompt += FOR_EACH_NODE
    }

    if (hasWatchNode && tools.some(tool => tool.name === 'watch_trigger')) {
      nodePrompt += WATCH_NODE
    }

    if (!nodePrompt.trim()) return undefined

    const template = taskNodesComponent.template!
    return resolveTemplate(template, { nodePrompt })
  },
}

/**
 * 任务示例组件
 */
export const taskExamplesComponent: ComponentConfig = {
  id: PromptComponent.TASK_EXAMPLES,
  name: 'Task Examples',
  description: '任务处理示例',
  required: false,
  template: `# Task Processing Examples
{examples}`,
  fn: async () => {
    // 这里可以根据任务类型提供相应的示例
    // 暂时返回undefined，后续可以扩展
    return undefined
  },
}
