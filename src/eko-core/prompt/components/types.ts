/**
 * 提示词组件类型定义
 */

import { Agent } from '../../agent'
import Context from '../../core/context'
import { Task, Tool } from '../../types'
import { TemplateContext } from '../template-engine'

/**
 * 提示词组件枚举
 * 定义了OrbitX中所有可用的提示词组件
 */
export enum PromptComponent {
  // Agent相关组件
  AGENT_ROLE = 'AGENT_ROLE',
  AGENT_DESCRIPTION = 'AGENT_DESCRIPTION',
  AGENT_CAPABILITIES = 'AGENT_CAPABILITIES',

  // 系统信息组件
  SYSTEM_INFO = 'SYSTEM_INFO',
  DATETIME = 'DATETIME',
  PLATFORM = 'PLATFORM',

  // 工具相关组件
  TOOLS_DESCRIPTION = 'TOOLS_DESCRIPTION',
  HUMAN_INTERACTION = 'HUMAN_INTERACTION',
  FOREACH_TOOLS = 'FOREACH_TOOLS',
  WATCH_TOOLS = 'WATCH_TOOLS',

  // 任务相关组件
  TASK_CONTEXT = 'TASK_CONTEXT',
  TASK_NODES = 'TASK_NODES',
  TASK_EXAMPLES = 'TASK_EXAMPLES',

  // 规划相关组件
  PLANNING_GUIDELINES = 'PLANNING_GUIDELINES',
  PLANNING_EXAMPLES = 'PLANNING_EXAMPLES',
  OUTPUT_FORMAT = 'OUTPUT_FORMAT',

  // 对话相关组件
  DIALOGUE_CAPABILITIES = 'DIALOGUE_CAPABILITIES',
  DIALOGUE_GUIDELINES = 'DIALOGUE_GUIDELINES',

  // 扩展组件
  WORKSPACE_SNAPSHOT = 'WORKSPACE_SNAPSHOT',
  CUSTOM_INSTRUCTIONS = 'CUSTOM_INSTRUCTIONS',
  ADDITIONAL_CONTEXT = 'ADDITIONAL_CONTEXT',
  AGENT_RULES = 'AGENT_RULES',
  WORK_METHODOLOGY = 'WORK_METHODOLOGY',
}

/**
 * 组件上下文接口
 */
export interface ComponentContext {
  agent: Agent
  task?: Task
  context?: Context
  tools?: Tool[]
  extSysPrompt?: string
  _templateOverride?: string
  [key: string]: unknown
}

/**
 * 组件函数类型
 */
export type ComponentFunction = (context: ComponentContext) => Promise<string | undefined>

/**
 * 组件配置接口
 */
export interface ComponentConfig {
  id: PromptComponent
  name: string
  description: string
  required: boolean
  dependencies?: PromptComponent[]
  template?: string
  fn: ComponentFunction
}

/**
 * 组件注册表类型
 */
export type ComponentRegistry = Record<PromptComponent, ComponentConfig>

/**
 * 提示词构建选项
 */
export interface PromptBuildOptions {
  components?: PromptComponent[]
  templateOverrides?: Partial<Record<PromptComponent, string>>
  additionalContext?: TemplateContext
  skipMissing?: boolean
}

/**
 * 提示词类型枚举
 */
export enum PromptType {
  AGENT = 'agent',
  DIALOGUE = 'dialogue',
  PLANNING = 'planning',
}

/**
 * 提示词变体配置
 */
export interface PromptVariant {
  type: PromptType
  components: PromptComponent[]
  template: string
  defaultContext?: TemplateContext
}
