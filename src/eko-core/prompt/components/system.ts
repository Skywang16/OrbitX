/**
 * 系统信息相关的提示词组件
 */

import config from '../../config'
import { ComponentConfig, ComponentContext, PromptComponent } from './types'
import { resolveTemplate } from '../template-engine'

/**
 * 系统信息组件
 */
export const systemInfoComponent: ComponentConfig = {
  id: PromptComponent.SYSTEM_INFO,
  name: 'System Info',
  description: 'Basic system information',
  required: false,
  template: `# System Information
Platform: {platform}
Agent: {agent}`,
  fn: async (context: ComponentContext) => {
    const { agent } = context
    const template = systemInfoComponent.template!
    return resolveTemplate(template, {
      platform: config.platform,
      agent: agent.Name,
    })
  },
}

/**
 * 日期时间组件
 */
export const datetimeComponent: ComponentConfig = {
  id: PromptComponent.DATETIME,
  name: 'DateTime',
  description: 'Current date and time',
  required: true,
  template: `Current datetime: {datetime}`,
  fn: async () => {
    const template = datetimeComponent.template!
    return resolveTemplate(template, {
      datetime: new Date().toLocaleString(),
    })
  },
}

/**
 * 平台信息组件
 */
export const platformComponent: ComponentConfig = {
  id: PromptComponent.PLATFORM,
  name: 'Platform',
  description: 'Platform information',
  required: false,
  template: `Platform: {platform}`,
  fn: async () => {
    const template = platformComponent.template!
    return resolveTemplate(template, {
      platform: config.platform,
    })
  },
}
