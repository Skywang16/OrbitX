/**
 * 提示词构建器
 * 负责组装各个组件生成完整的提示词
 */

import { ComponentContext, PromptComponent, PromptBuildOptions, PromptVariant } from '../components/types'
import { getComponentRegistry } from '../components/registry'
import { EkoTemplateEngine } from '../template-engine'

/**
 * 提示词构建器类
 */
export class PromptBuilder {
  private templateEngine: EkoTemplateEngine

  constructor() {
    this.templateEngine = EkoTemplateEngine.getInstance()
  }

  /**
   * 构建提示词
   * @param context 组件上下文
   * @param options 构建选项
   */
  async build(context: ComponentContext, options: PromptBuildOptions = {}): Promise<string> {
    const registry = getComponentRegistry()
    await registry.load()

    const {
      components = [],
      templateOverrides = {} as Partial<Record<PromptComponent, string>>,
      skipMissing = true,
    } = options

    // 验证依赖关系
    const errors = registry.validateDependencies(components)
    if (errors.length > 0 && !skipMissing) {
      throw new Error(`组件依赖验证失败: ${errors.join(', ')}`)
    }

    // 按依赖关系排序组件
    const sortedComponents = registry.sortComponentsByDependencies(components)

    // 构建各个组件
    const componentResults: Record<string, string> = {}

    for (const componentId of sortedComponents) {
      const component = registry.getComponent(componentId)
      if (!component) {
        if (!skipMissing) {
          throw new Error(`组件不存在: ${componentId}`)
        }
        continue
      }

      try {
        // 创建组件副本以避免修改共享对象
        const componentCopy = { ...component }
        if (templateOverrides[componentId]) {
          componentCopy.template = templateOverrides[componentId]
        }

        // 创建临时上下文，包含覆盖的模板
        const contextWithTemplate = {
          ...context,
          _templateOverride: componentCopy.template,
        }

        const result = await component.fn(contextWithTemplate)

        if (result !== undefined) {
          componentResults[componentId] = result
        }
      } catch (error) {
        if (!skipMissing) {
          throw new Error(`组件 ${componentId} 构建失败: ${error}`)
        }
        console.warn(`组件 ${componentId} 构建失败，已跳过:`, error)
      }
    }

    // 组装最终提示词
    return this.assemblePrompt(componentResults)
  }

  /**
   * 使用预定义变体构建提示词
   * @param variant 提示词变体
   * @param context 组件上下文
   * @param additionalContext 额外上下文
   */
  async buildFromVariant(
    variant: PromptVariant,
    context: ComponentContext,
    additionalContext: Record<string, unknown> = {}
  ): Promise<string> {
    // 构建组件
    const options: PromptBuildOptions = {
      components: variant.components,
      additionalContext: { ...variant.defaultContext, ...additionalContext },
    }

    const componentResults = await this.buildComponents(context, options)

    // 使用变体模板组装
    const allContext = {
      ...componentResults,
      ...additionalContext,
      ...variant.defaultContext,
    }

    return this.templateEngine.resolve(variant.template, allContext)
  }

  /**
   * 构建组件内容
   */
  private async buildComponents(
    context: ComponentContext,
    options: PromptBuildOptions
  ): Promise<Record<string, string>> {
    const registry = getComponentRegistry()
    await registry.load()

    const { components = [], templateOverrides = {}, skipMissing = true } = options

    const sortedComponents = registry.sortComponentsByDependencies(components)
    const componentResults: Record<string, string> = {}

    for (const componentId of sortedComponents) {
      const component = registry.getComponent(componentId)
      if (!component) {
        if (!skipMissing) {
          throw new Error(`组件不存在: ${componentId}`)
        }
        continue
      }

      try {
        const originalTemplate = component.template
        if (templateOverrides[componentId]) {
          component.template = templateOverrides[componentId]
        }

        const result = await component.fn(context)

        if (originalTemplate !== undefined) {
          component.template = originalTemplate
        }

        if (result !== undefined) {
          componentResults[componentId] = result
        }
      } catch (error) {
        if (!skipMissing) {
          throw new Error(`组件 ${componentId} 构建失败: ${error}`)
        }
        console.warn(`组件 ${componentId} 构建失败，已跳过:`, error)
      }
    }

    return componentResults
  }

  /**
   * 组装提示词
   */
  private assemblePrompt(componentResults: Record<string, string>): string {
    // 简单的组装方式：按顺序连接所有组件
    const sections = Object.values(componentResults).filter(Boolean)

    if (sections.length === 0) {
      return ''
    }

    // 用双换行符分隔各个部分
    return sections.join('\n\n').trim()
  }
}

/**
 * 便捷函数：快速构建提示词
 */
export async function buildPrompt(
  context: ComponentContext,
  components: PromptComponent[],
  options: Omit<PromptBuildOptions, 'components'> = {}
): Promise<string> {
  const builder = new PromptBuilder()
  return builder.build(context, { ...options, components })
}
