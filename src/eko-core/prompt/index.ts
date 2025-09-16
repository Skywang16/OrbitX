/**
 * 提示词模块统一导出
 * 基于组件化架构的现代提示词工程系统
 */

export {
  EkoTemplateEngine,
  resolveTemplate,
  extractPlaceholders,
  type TemplateContext,
  type TemplateOptions,
} from './template-engine'

export {
  PromptComponent,
  PromptType,
  type ComponentContext,
  type ComponentFunction,
  type ComponentConfig,
  type ComponentRegistry,
  type PromptBuildOptions,
  type PromptVariant,
} from './components/types'

export { getComponentRegistry, getComponent, getAllComponents } from './components/registry'

export { PromptBuilder, buildPrompt } from './builders/prompt-builder'
export {
  AgentPromptBuilder,
  buildAgentSystemPrompt,
  buildAgentUserPrompt,
  // 主要API别名
  buildAgentSystemPrompt as getAgentSystemPrompt,
  buildAgentUserPrompt as getAgentUserPrompt,
} from './builders/agent-builder'
export { DialoguePromptBuilder, buildDialogueSystemPrompt, getDialogueSystemPrompt } from './builders/dialogue-builder'
export {
  PlanPromptBuilder,
  buildPlanSystemPrompt,
  buildPlanUserPrompt,
  // 主要API别名
  buildPlanSystemPrompt as getPlanSystemPrompt,
  buildPlanUserPrompt as getPlanUserPrompt,
} from './builders/plan-builder'

export {
  PromptConfigManager,
  getPromptConfig,
  defaultPromptConfig,
  type PromptConfig,
  type ComponentConfigOptions,
  type ConditionalRule,
} from '../config/prompt.config'
