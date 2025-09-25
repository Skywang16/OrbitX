/**
 * 组件注册表
 * 管理所有提示词组件的注册和获取
 */

import { ComponentRegistry, ComponentConfig, PromptComponent } from './types'
import * as agentComponents from './agent'
import * as systemComponents from './system'
import * as toolComponents from './tools'
import * as taskComponents from './task'
import * as planningComponents from './planning'
import * as dialogueComponents from './dialogue'
import * as workspaceComponents from './workspace'

/**
 * 全局组件注册表
 */
class PromptComponentRegistry {
  private static instance: PromptComponentRegistry
  private registry: ComponentRegistry = {} as ComponentRegistry
  private loaded = false

  static getInstance(): PromptComponentRegistry {
    if (!PromptComponentRegistry.instance) {
      PromptComponentRegistry.instance = new PromptComponentRegistry()
    }
    return PromptComponentRegistry.instance
  }

  /**
   * 加载所有组件
   */
  async load(): Promise<void> {
    if (this.loaded) return

    // 注册各类组件
    const isComponentConfig = (v: unknown): v is ComponentConfig => {
      return (
        typeof v === 'object' &&
        v !== null &&
        'id' in (v as Record<string, unknown>) &&
        'fn' in (v as Record<string, unknown>)
      )
    }

    const all = [
      ...Object.values(agentComponents),
      ...Object.values(systemComponents),
      ...Object.values(toolComponents),
      ...Object.values(taskComponents),
      ...Object.values(planningComponents),
      ...Object.values(dialogueComponents),
      ...Object.values(workspaceComponents),
    ]

    this.registerComponents(all.filter(isComponentConfig))

    this.loaded = true
  }

  /**
   * 注册组件
   */
  private registerComponents(components: ComponentConfig[]): void {
    for (const component of components) {
      this.registry[component.id] = component
    }
  }

  /**
   * 获取组件
   */
  getComponent(id: PromptComponent): ComponentConfig | undefined {
    return this.registry[id]
  }

  /**
   * 获取所有组件
   */
  getAllComponents(): ComponentRegistry {
    return { ...this.registry }
  }

  /**
   * 检查组件是否存在
   */
  hasComponent(id: PromptComponent): boolean {
    return id in this.registry
  }

  /**
   * 获取组件列表
   */
  getComponentIds(): PromptComponent[] {
    return Object.keys(this.registry) as PromptComponent[]
  }

  /**
   * 根据依赖关系排序组件
   */
  sortComponentsByDependencies(components: PromptComponent[]): PromptComponent[] {
    const sorted: PromptComponent[] = []
    const visited = new Set<PromptComponent>()
    const visiting = new Set<PromptComponent>()

    const visit = (componentId: PromptComponent) => {
      if (visiting.has(componentId)) {
        throw new Error(`检测到循环依赖: ${componentId}`)
      }
      if (visited.has(componentId)) {
        return
      }

      visiting.add(componentId)
      const component = this.registry[componentId]

      if (component?.dependencies) {
        for (const dep of component.dependencies) {
          if (components.includes(dep)) {
            visit(dep)
          }
        }
      }

      visiting.delete(componentId)
      visited.add(componentId)
      sorted.push(componentId)
    }

    for (const componentId of components) {
      if (!visited.has(componentId)) {
        visit(componentId)
      }
    }

    return sorted
  }

  /**
   * 验证组件依赖
   */
  validateDependencies(components: PromptComponent[]): string[] {
    const errors: string[] = []

    for (const componentId of components) {
      const component = this.registry[componentId]
      if (!component) {
        errors.push(`组件不存在: ${componentId}`)
        continue
      }

      if (component.dependencies) {
        for (const dep of component.dependencies) {
          if (!components.includes(dep)) {
            errors.push(`组件 ${componentId} 依赖的组件 ${dep} 未包含在列表中`)
          }
        }
      }
    }

    return errors
  }
}

/**
 * 获取组件注册表实例
 */
export function getComponentRegistry(): PromptComponentRegistry {
  return PromptComponentRegistry.getInstance()
}

/**
 * 便捷函数：获取组件
 */
export async function getComponent(id: PromptComponent): Promise<ComponentConfig | undefined> {
  const registry = getComponentRegistry()
  await registry.load()
  return registry.getComponent(id)
}

/**
 * 便捷函数：获取所有组件
 */
export async function getAllComponents(): Promise<ComponentRegistry> {
  const registry = getComponentRegistry()
  await registry.load()
  return registry.getAllComponents()
}
