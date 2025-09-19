/**
 * 提示词配置管理
 * 定义结构化的提示词配置接口和默认配置
 */

import { PromptComponent, PromptType, PromptVariant } from '../prompt/components/types'

/**
 * 提示词配置接口
 */
export interface PromptConfig {
  // 默认组件顺序
  defaultComponentOrder: Record<PromptType, PromptComponent[]>

  // 模板覆盖
  templateOverrides: Record<string, Partial<Record<PromptComponent, string>>>

  // 启用的功能
  enabledFeatures: string[]

  // 组件配置
  componentConfig: Record<PromptComponent, ComponentConfigOptions>

  // 变体配置
  variants: Record<string, PromptVariant>
}

/**
 * 组件配置选项
 */
export interface ComponentConfigOptions {
  enabled?: boolean
  priority?: number
  dependencies?: PromptComponent[]
  conditionalRules?: ConditionalRule[]
}

/**
 * 条件规则
 */
export interface ConditionalRule {
  condition: string // 条件表达式
  action: 'include' | 'exclude' | 'modify'
  params?: Record<string, unknown>
}

/**
 * 默认提示词配置
 */
export const defaultPromptConfig: PromptConfig = {
  defaultComponentOrder: {
    [PromptType.AGENT]: [
      PromptComponent.AGENT_ROLE,
      PromptComponent.AGENT_CAPABILITIES,
      PromptComponent.AGENT_RULES,
      PromptComponent.WORK_METHODOLOGY,
      PromptComponent.SYSTEM_INFO,
      PromptComponent.TASK_CONTEXT,
      PromptComponent.TOOLS_DESCRIPTION,
      PromptComponent.TASK_NODES,
      PromptComponent.HUMAN_INTERACTION,
      PromptComponent.FOREACH_TOOLS,
      PromptComponent.WATCH_TOOLS,
      PromptComponent.CUSTOM_INSTRUCTIONS,
      PromptComponent.DATETIME,
    ],

    [PromptType.DIALOGUE]: [
      PromptComponent.AGENT_ROLE,
      PromptComponent.DIALOGUE_CAPABILITIES,
      PromptComponent.DIALOGUE_GUIDELINES,
      PromptComponent.CUSTOM_INSTRUCTIONS,
      PromptComponent.DATETIME,
    ],

    [PromptType.PLANNING]: [
      PromptComponent.AGENT_ROLE,
      PromptComponent.PLANNING_GUIDELINES,
      PromptComponent.AGENT_CAPABILITIES,
      PromptComponent.OUTPUT_FORMAT,
      PromptComponent.PLANNING_EXAMPLES,
      PromptComponent.DATETIME,
    ],
  },

  templateOverrides: {
    // 可以为特定场景定义模板覆盖
    terminal: {
      [PromptComponent.AGENT_ROLE]: 'You are {name}, a terminal-focused AI assistant.',
    } as Partial<Record<PromptComponent, string>>,
    development: {
      [PromptComponent.AGENT_ROLE]: 'You are {name}, a development-focused AI assistant.',
    } as Partial<Record<PromptComponent, string>>,
  },

  enabledFeatures: [
    'human_interaction',
    'foreach_processing',
    'watch_monitoring',
    'dynamic_tools',
    'context_awareness',
  ],

  componentConfig: {
    [PromptComponent.AGENT_ROLE]: {
      enabled: true,
      priority: 1,
    },
    [PromptComponent.AGENT_DESCRIPTION]: {
      enabled: true,
      priority: 2,
    },
    [PromptComponent.SYSTEM_INFO]: {
      enabled: true,
      priority: 3,
    },
    [PromptComponent.AGENT_CAPABILITIES]: {
      enabled: true,
      priority: 4,
      dependencies: [PromptComponent.TOOLS_DESCRIPTION],
    },
    [PromptComponent.HUMAN_INTERACTION]: {
      enabled: true,
      priority: 5,
      conditionalRules: [
        {
          condition: 'hasHumanTool',
          action: 'include',
        },
      ],
    },
    [PromptComponent.FOREACH_TOOLS]: {
      enabled: true,
      priority: 6,
      conditionalRules: [
        {
          condition: 'hasForEachTool',
          action: 'include',
        },
      ],
    },
    [PromptComponent.WATCH_TOOLS]: {
      enabled: true,
      priority: 7,
      conditionalRules: [
        {
          condition: 'hasWatchTool',
          action: 'include',
        },
      ],
    },
    [PromptComponent.TASK_CONTEXT]: {
      enabled: true,
      priority: 8,
    },
    [PromptComponent.TASK_NODES]: {
      enabled: true,
      priority: 9,
    },
    [PromptComponent.CUSTOM_INSTRUCTIONS]: {
      enabled: true,
      priority: 10,
    },
    [PromptComponent.DATETIME]: {
      enabled: true,
      priority: 11,
    },
    [PromptComponent.WORKSPACE_SNAPSHOT]: {
      enabled: true,
      priority: 5,
      dependencies: [],
    },
    [PromptComponent.TOOLS_DESCRIPTION]: {
      enabled: true,
      priority: 4,
    },
    [PromptComponent.DIALOGUE_CAPABILITIES]: {
      enabled: true,
      priority: 2,
    },
    [PromptComponent.DIALOGUE_GUIDELINES]: {
      enabled: true,
      priority: 3,
    },
    [PromptComponent.PLANNING_GUIDELINES]: {
      enabled: true,
      priority: 2,
    },
    [PromptComponent.PLANNING_EXAMPLES]: {
      enabled: true,
      priority: 4,
    },
    [PromptComponent.OUTPUT_FORMAT]: {
      enabled: true,
      priority: 3,
    },
    [PromptComponent.PLATFORM]: {
      enabled: false,
      priority: 3,
    },
    [PromptComponent.TASK_EXAMPLES]: {
      enabled: false,
      priority: 9,
    },
    [PromptComponent.ADDITIONAL_CONTEXT]: {
      enabled: false,
      priority: 12,
    },
    [PromptComponent.AGENT_RULES]: {
      enabled: true,
      priority: 3,
    },
    [PromptComponent.WORK_METHODOLOGY]: {
      enabled: true,
      priority: 4,
    },
  },

  variants: {
    'default-agent': {
      type: PromptType.AGENT,
      components: [PromptComponent.AGENT_ROLE, PromptComponent.AGENT_DESCRIPTION, PromptComponent.DATETIME],
      template: '{AGENT_ROLE}\n\n{AGENT_DESCRIPTION}\n\n{DATETIME}',
    },
    'default-dialogue': {
      type: PromptType.DIALOGUE,
      components: [PromptComponent.AGENT_ROLE, PromptComponent.DIALOGUE_CAPABILITIES, PromptComponent.DATETIME],
      template: '{AGENT_ROLE}\n\n{DIALOGUE_CAPABILITIES}\n\n{DATETIME}',
    },
  },
}

/**
 * 提示词配置管理器
 */
export class PromptConfigManager {
  private static instance: PromptConfigManager
  private config: PromptConfig

  constructor(config: PromptConfig = defaultPromptConfig) {
    this.config = { ...config }
  }

  static getInstance(): PromptConfigManager {
    if (!PromptConfigManager.instance) {
      PromptConfigManager.instance = new PromptConfigManager()
    }
    return PromptConfigManager.instance
  }

  /**
   * 获取配置
   */
  getConfig(): PromptConfig {
    return { ...this.config }
  }

  /**
   * 更新配置
   */
  updateConfig(updates: Partial<PromptConfig>): void {
    this.config = { ...this.config, ...updates }
  }

  /**
   * 获取组件顺序
   */
  getComponentOrder(type: PromptType): PromptComponent[] {
    const baseOrder = this.config.defaultComponentOrder[type] || []

    // 根据组件配置过滤和排序
    return baseOrder
      .filter(component => {
        const componentConfig = this.config.componentConfig[component]
        return componentConfig?.enabled !== false
      })
      .sort((a, b) => {
        const priorityA = this.config.componentConfig[a]?.priority || 0
        const priorityB = this.config.componentConfig[b]?.priority || 0
        return priorityA - priorityB
      })
  }

  /**
   * 获取模板覆盖
   */
  getTemplateOverrides(scenario?: string): Partial<Record<PromptComponent, string>> {
    if (scenario && this.config.templateOverrides[scenario]) {
      return this.config.templateOverrides[scenario]
    }
    return {}
  }

  /**
   * 检查功能是否启用
   */
  isFeatureEnabled(feature: string): boolean {
    return this.config.enabledFeatures.includes(feature)
  }

  /**
   * 获取变体配置
   */
  getVariant(name: string): PromptVariant | undefined {
    return this.config.variants[name]
  }
}

/**
 * 便捷函数：获取配置管理器
 */
export function getPromptConfig(): PromptConfigManager {
  return PromptConfigManager.getInstance()
}
