/**
 * Agent提示词构建器
 * 专门用于构建Agent系统提示词
 */

import { Agent } from '../../agent'
import Context from '../../core/context'
import { Task, Tool } from '../../types'
import { ComponentContext, PromptType } from '../components/types'
import { PromptBuilder } from './prompt-builder'
import { buildAgentRootXml } from '../../common/xml'
import { TOOL_NAME as task_node_status } from '../../tools/task_node_status'
import { getPromptConfig } from '../../config/prompt.config'

/**
 * Agent提示词构建器
 */
export class AgentPromptBuilder extends PromptBuilder {
  /**
   * 构建Agent系统提示词
   */
  async buildAgentSystemPrompt(
    agent: Agent,
    task?: Task,
    context?: Context,
    tools?: Tool[],
    extSysPrompt?: string
  ): Promise<string> {
    // 准备组件上下文
    const componentContext: ComponentContext = {
      agent,
      task,
      context,
      tools: tools || agent.Tools,
      extSysPrompt,
    }

    // 定义Agent系统提示词的组件顺序
    // 从配置管理器获取组件顺序
    const configManager = getPromptConfig()
    const components = configManager.getComponentOrder(PromptType.AGENT)
    const templateOverrides = configManager.getTemplateOverrides()

    return this.build(componentContext, { components, templateOverrides })
  }

  /**
   * 构建Agent用户提示词
   */
  buildAgentUserPrompt(agent: Agent, task?: Task, context?: Context, tools?: Tool[]): string {
    const hasTaskNodeStatusTool = (tools || agent.Tools).some(tool => tool.name === task_node_status)

    return buildAgentRootXml(task?.xml || '', context?.chain.taskPrompt || '', (_nodeId, node) => {
      if (hasTaskNodeStatusTool) {
        node.setAttribute('status', 'todo')
      }
    })
  }
}

/**
 * 便捷函数：构建Agent系统提示词
 */
export async function buildAgentSystemPrompt(
  agent: Agent,
  task?: Task,
  context?: Context,
  tools?: Tool[],
  extSysPrompt?: string
): Promise<string> {
  const builder = new AgentPromptBuilder()
  return builder.buildAgentSystemPrompt(agent, task, context, tools, extSysPrompt)
}

/**
 * 便捷函数：构建Agent用户提示词
 */
export function buildAgentUserPrompt(agent: Agent, task?: Task, context?: Context, tools?: Tool[]): string {
  const builder = new AgentPromptBuilder()
  return builder.buildAgentUserPrompt(agent, task, context, tools)
}
