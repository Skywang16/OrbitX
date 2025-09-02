/**
 * Enhanced Template Engine for OrbitX
 * Supports nested object access and advanced template features
 */

export interface TemplateContext {
  [key: string]: any
}

export interface TemplateOptions {
  preserveUnresolved?: boolean // 是否保留未解析的占位符
  throwOnMissing?: boolean     // 遇到缺失值时是否抛出错误
}

export class EkoTemplateEngine {
  private static instance: EkoTemplateEngine
  
  /**
   * 获取单例实例
   */
  static getInstance(): EkoTemplateEngine {
    if (!EkoTemplateEngine.instance) {
      EkoTemplateEngine.instance = new EkoTemplateEngine()
    }
    return EkoTemplateEngine.instance
  }

  /**
   * 解析模板占位符，格式为 {PLACEHOLDER} 或 {object.property}
   * @param template 模板字符串
   * @param context 上下文对象
   * @param options 解析选项
   */
  resolve(template: string, context: TemplateContext, options: TemplateOptions = {}): string {
    const { preserveUnresolved = true, throwOnMissing = false } = options

    return template.replace(/\{([^}]+)\}/g, (match, key) => {
      const trimmedKey = key.trim()
      
      try {
        // 支持点号嵌套访问
        const value = this.getNestedValue(context, trimmedKey)
        
        if (value !== undefined && value !== null) {
          return typeof value === 'string' ? value : JSON.stringify(value)
        }
        
        // 处理缺失值
        if (throwOnMissing) {
          throw new Error(`Template placeholder '${trimmedKey}' not found in context`)
        }
        
        // 如果未找到且允许保留，则保留占位符
        return preserveUnresolved ? match : ''
      } catch (error) {
        if (throwOnMissing) {
          throw error
        }
        return preserveUnresolved ? match : ''
      }
    })
  }

  /**
   * 获取嵌套对象的值
   * @param obj 对象
   * @param path 路径，如 'user.profile.name'
   */
  private getNestedValue(obj: any, path: string): any {
    if (!path) return undefined
    
    return path.split('.').reduce((current, key) => {
      if (current === null || current === undefined) {
        return undefined
      }
      
      // 处理数组索引访问，如 items[0]
      const arrayMatch = key.match(/^(\w+)\[(\d+)\]$/)
      if (arrayMatch) {
        const [, arrayKey, index] = arrayMatch
        const array = current[arrayKey]
        return Array.isArray(array) ? array[parseInt(index, 10)] : undefined
      }
      
      return current[key]
    }, obj)
  }

  /**
   * 验证模板中是否包含所有必需的占位符
   * @param template 模板字符串
   * @param requiredPlaceholders 必需的占位符列表
   */
  validate(template: string, requiredPlaceholders: string[]): string[] {
    const missingPlaceholders: string[] = []
    
    for (const placeholder of requiredPlaceholders) {
      const regex = new RegExp(`\\{\\s*${this.escapeRegex(placeholder)}\\s*\\}`, 'g')
      if (!regex.test(template)) {
        missingPlaceholders.push(placeholder)
      }
    }
    
    return missingPlaceholders
  }

  /**
   * 提取模板中的所有占位符
   * @param template 模板字符串
   */
  extractPlaceholders(template: string): string[] {
    const placeholders: string[] = []
    const regex = /\{([^}]+)\}/g
    let match: RegExpExecArray | null
    
    while ((match = regex.exec(template)) !== null) {
      const placeholder = match[1].trim()
      if (!placeholders.includes(placeholder)) {
        placeholders.push(placeholder)
      }
    }
    
    return placeholders
  }

  /**
   * 转义正则表达式特殊字符
   */
  private escapeRegex(str: string): string {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  /**
   * 批量解析多个模板
   * @param templates 模板对象
   * @param context 上下文对象
   * @param options 解析选项
   */
  resolveMultiple(
    templates: Record<string, string>, 
    context: TemplateContext, 
    options: TemplateOptions = {}
  ): Record<string, string> {
    const result: Record<string, string> = {}
    
    for (const [key, template] of Object.entries(templates)) {
      result[key] = this.resolve(template, context, options)
    }
    
    return result
  }

  /**
   * 检查模板是否有效（语法正确）
   * @param template 模板字符串
   */
  isValidTemplate(template: string): boolean {
    try {
      // 检查括号是否匹配
      let openCount = 0
      for (const char of template) {
        if (char === '{') openCount++
        if (char === '}') openCount--
        if (openCount < 0) return false
      }
      return openCount === 0
    } catch {
      return false
    }
  }
}

/**
 * 便捷函数：快速解析模板
 */
export function resolveTemplate(
  template: string, 
  context: TemplateContext, 
  options?: TemplateOptions
): string {
  return EkoTemplateEngine.getInstance().resolve(template, context, options)
}

/**
 * 便捷函数：提取占位符
 */
export function extractPlaceholders(template: string): string[] {
  return EkoTemplateEngine.getInstance().extractPlaceholders(template)
}
