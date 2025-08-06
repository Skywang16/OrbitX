/**
 * 提示词引擎
 *
 * 提供提示词模板管理、动态生成和格式化功能
 */

import type { PromptTemplate, PromptGenerationOptions } from '../types/prompt'
import type { ToolDefinition } from '../types/tool'

/**
 * 提示词引擎类
 */
export class PromptEngine {
  private _templates: Map<string, PromptTemplate> = new Map()
  private _globalVariables: Record<string, unknown> = {}

  /**
   * 创建一个PromptEngine实例并自动从模块加载模板
   */
  static async createWithTemplates(): Promise<PromptEngine> {
    const engine = new PromptEngine()
    await engine.loadTemplatesFromModules()
    return engine
  }

  /**
   * 从文件模块动态加载模板
   * 使用 Vite 的 import.meta.glob 功能
   */
  async loadTemplatesFromModules(): Promise<void> {
    // Vite/Webpack specific feature to load all .txt files from a directory
    const templateModules = import.meta.glob('../prompt/templates/*.txt', { as: 'raw' })

    for (const path in templateModules) {
      const content = await templateModules[path]()
      const id = path.split('/').pop()?.replace('.txt', '')

      if (id && content) {
        this.registerTemplate({
          id,
          name: id.replace('-', ' '),
          template: content,
          variables: [], // 在这里可以添加对模板变量的解析逻辑
        })
      }
    }
  }
  // ===== 模板管理 =====

  /**
   * 注册提示词模板
   */
  registerTemplate(template: PromptTemplate): void {
    // 简化验证，因为模板现在是动态加载的
    if (!template.id || !template.template) {
      console.error('Template must have an id and content.', template)
      return
    }
    this._templates.set(template.id, template)
  }

  /**
   * 批量注册模板
   */
  registerTemplates(templates: PromptTemplate[]): void {
    for (const template of templates) {
      this.registerTemplate(template)
    }
  }

  /**
   * 获取模板
   */
  getTemplate(id: string): PromptTemplate | undefined {
    return this._templates.get(id)
  }

  /**
   * 删除模板
   */
  removeTemplate(id: string): boolean {
    return this._templates.delete(id)
  }

  /**
   * 获取所有模板
   */
  getAllTemplates(): PromptTemplate[] {
    return Array.from(this._templates.values())
  }

  // ===== 全局变量管理 =====

  /**
   * 设置全局变量
   */
  setGlobalVariable(name: string, value: unknown): void {
    this._globalVariables[name] = value
  }

  /**
   * 设置多个全局变量
   */
  setGlobalVariables(variables: Record<string, unknown>): void {
    this._globalVariables = { ...this._globalVariables, ...variables }
  }

  /**
   * 获取全局变量
   */
  getGlobalVariable(name: string): unknown {
    return this._globalVariables[name]
  }

  /**
   * 获取所有全局变量
   */
  getAllGlobalVariables(): Record<string, unknown> {
    return { ...this._globalVariables }
  }

  // ===== 提示词生成 =====

  /**
   * 生成提示词
   */
  generate(templateId: string, options?: PromptGenerationOptions): string {
    const template = this._templates.get(templateId)
    if (!template) {
      throw new Error(`Template '${templateId}' not found`)
    }

    // 合并变量：全局变量 + 传入变量
    const variables = { ...this._globalVariables, ...(options?.variables || {}) }

    // 渲染模板
    return this.renderTemplate(template.template, variables)
  }

  // ===== 模板渲染 =====

  /**
   * 渲染模板字符串
   */
  private renderTemplate(template: string, variables: Record<string, unknown>): string {
    let result = template

    // 替换变量 {{variableName}}
    result = result.replace(/\{\{([a-zA-Z0-9_]+)\}\}/g, (match, variableName) => {
      const value = variables[variableName]
      if (value === undefined || value === null) {
        return match // 保留原始占位符
      }
      return String(value)
    })

    // 替换条件块 {{#if condition}}...{{/if}}
    result = result.replace(/\{\{#if\s+([a-zA-Z0-9_]+)\}\}([\s\S]*?)\{\{\/if\}\}/g, (match, condition, content) => {
      const value = variables[condition]
      return value ? content : ''
    })

    return result
  }
}

// 导出单例实例，异步创建
export const promptEngine = await PromptEngine.createWithTemplates()
