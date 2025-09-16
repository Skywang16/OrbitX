/**
 * Dialogue提示词构建器
 * 专门用于构建对话系统提示词
 */

import config from '../../config'
import { PromptComponent, ComponentContext } from '../components/types'
import type { Agent } from '../../agent'
import { PromptBuilder } from './prompt-builder'

/**
 * Dialogue提示词构建器
 */
export class DialoguePromptBuilder extends PromptBuilder {
  /**
   * 构建对话系统提示词
   */
  async buildDialogueSystemPrompt(extSysPrompt?: string): Promise<string> {
    // 准备组件上下文
    const componentContext: ComponentContext = {
      agent: {
        Name: config.name,
        Description: 'AI assistant specialized for terminal emulator applications',
      } as unknown as Agent,
      extSysPrompt,
    }

    // 定义对话系统提示词的组件顺序
    const components: PromptComponent[] = [
      PromptComponent.AGENT_ROLE,
      PromptComponent.DIALOGUE_CAPABILITIES,
      PromptComponent.DIALOGUE_GUIDELINES,
      PromptComponent.CUSTOM_INSTRUCTIONS,
      PromptComponent.DATETIME,
    ]

    // 使用自定义模板覆盖
    const templateOverrides = {
      [PromptComponent.AGENT_ROLE]: `You are {name}, a helpful AI assistant specialized for terminal emulator applications.`,
    }

    return this.build(componentContext, {
      components,
      templateOverrides,
      additionalContext: {
        name: config.name,
      },
    })
  }
}

/**
 * 便捷函数：构建对话系统提示词
 */
export async function buildDialogueSystemPrompt(extSysPrompt?: string): Promise<string> {
  const builder = new DialoguePromptBuilder()
  return builder.buildDialogueSystemPrompt(extSysPrompt)
}

/**
 * 同步版本：用于构造函数等需要同步调用的场景
 */
export function getDialogueSystemPrompt(extSysPrompt?: string): string {
  let prompt = ''
  if (extSysPrompt && extSysPrompt.trim()) {
    prompt += '\n' + extSysPrompt.trim() + '\n'
  }
  prompt += '\nCurrent datetime: ' + new Date().toLocaleString()

  const template = `You are OrbitX, a helpful AI assistant specialized for terminal emulator applications.

# Terminal Environment Capabilities
You excel at helping users with:
- File operations and text editing
- Shell command execution and scripting
- Code development and project management
- System administration and automation
- Terminal-based workflows and productivity

${prompt}`

  return template.trim()
}
