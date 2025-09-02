/**
 * 动态工具描述生成器
 * 根据上下文动态生成工具描述和过滤可用工具
 */

import { Tool } from '../types'
import { AgentContext } from '../core/context'
import { resolveTemplate } from '../prompt/template-engine'

/**
 * 工具上下文要求接口
 */
export interface ToolContextRequirements {
  // 平台要求
  platforms?: string[]
  // 环境变量要求
  envVars?: string[]
  // 依赖工具
  dependencies?: string[]
  // 自定义检查函数
  customCheck?: (context: AgentContext) => boolean
}

/**
 * 工具描述配置接口
 */
export interface ToolDescriptorConfig {
  id: string
  name: string
  description: string
  detailedDescription?: string
  usage?: string
  examples?: string[]
  contextRequirements?: ToolContextRequirements
  category?: string
  priority?: number
}

/**
 * 工具描述生成器
 */
export class ToolDescriptor {
  
  /**
   * 根据上下文过滤可用工具
   * @param tools 工具列表
   * @param context Agent上下文
   */
  static filterToolsByContext(tools: Tool[], context: AgentContext): Tool[] {
    return tools.filter(tool => {
      // 检查工具是否有上下文要求
      const requirements = (tool as any).contextRequirements as ToolContextRequirements
      if (!requirements) return true

      return this.checkContextRequirements(requirements, context)
    })
  }

  /**
   * 生成工具描述
   * @param tool 工具对象
   * @param context Agent上下文
   * @param format 描述格式 ('simple' | 'detailed' | 'markdown')
   */
  static generateDescription(
    tool: Tool, 
    context: AgentContext, 
    format: 'simple' | 'detailed' | 'markdown' = 'simple'
  ): string {
    const config = (tool as any).descriptorConfig as ToolDescriptorConfig
    
    if (!config) {
      // 回退到基本描述
      return `${tool.name}: ${tool.description || 'No description available'}`
    }

    switch (format) {
      case 'simple':
        return this.generateSimpleDescription(config, context)
      case 'detailed':
        return this.generateDetailedDescription(config, context)
      case 'markdown':
        return this.generateMarkdownDescription(config, context)
      default:
        return this.generateSimpleDescription(config, context)
    }
  }

  /**
   * 批量生成工具描述
   * @param tools 工具列表
   * @param context Agent上下文
   * @param format 描述格式
   */
  static generateToolsDescription(
    tools: Tool[], 
    context: AgentContext,
    format: 'simple' | 'detailed' | 'markdown' = 'simple'
  ): string {
    const filteredTools = this.filterToolsByContext(tools, context)
    
    if (filteredTools.length === 0) {
      return 'No tools available in current context.'
    }

    const descriptions = filteredTools.map(tool => 
      this.generateDescription(tool, context, format)
    )

    return descriptions.join('\n')
  }

  /**
   * 按类别分组工具描述
   * @param tools 工具列表
   * @param context Agent上下文
   */
  static generateCategorizedDescription(tools: Tool[], context: AgentContext): string {
    const filteredTools = this.filterToolsByContext(tools, context)
    
    // 按类别分组
    const categories = new Map<string, Tool[]>()
    
    for (const tool of filteredTools) {
      const config = (tool as any).descriptorConfig as ToolDescriptorConfig
      const category = config?.category || 'General'
      
      if (!categories.has(category)) {
        categories.set(category, [])
      }
      categories.get(category)!.push(tool)
    }

    // 生成分类描述
    const sections: string[] = []
    
    for (const [category, categoryTools] of categories.entries()) {
      const toolDescriptions = categoryTools.map(tool => 
        this.generateDescription(tool, context, 'simple')
      )
      
      sections.push(`## ${category}\n${toolDescriptions.join('\n')}`)
    }

    return sections.join('\n\n')
  }

  /**
   * 检查上下文要求
   */
  private static checkContextRequirements(
    requirements: ToolContextRequirements, 
    context: AgentContext
  ): boolean {
    // 检查平台要求
    if (requirements.platforms) {
      const currentPlatform = process.platform
      if (!requirements.platforms.includes(currentPlatform)) {
        return false
      }
    }

    // 检查环境变量要求
    if (requirements.envVars) {
      for (const envVar of requirements.envVars) {
        if (!process.env[envVar]) {
          return false
        }
      }
    }

    // 检查依赖工具
    if (requirements.dependencies) {
      const availableTools = context.agent.Tools.map(tool => tool.name)
      for (const dep of requirements.dependencies) {
        if (!availableTools.includes(dep)) {
          return false
        }
      }
    }

    // 自定义检查
    if (requirements.customCheck) {
      return requirements.customCheck(context)
    }

    return true
  }

  /**
   * 生成简单描述
   */
  private static generateSimpleDescription(config: ToolDescriptorConfig, context: AgentContext): string {
    return `${config.name}: ${config.description}`
  }

  /**
   * 生成详细描述
   */
  private static generateDetailedDescription(config: ToolDescriptorConfig, context: AgentContext): string {
    let description = `${config.name}: ${config.description}`
    
    if (config.detailedDescription) {
      description += `\n  Details: ${config.detailedDescription}`
    }
    
    if (config.usage) {
      description += `\n  Usage: ${config.usage}`
    }
    
    if (config.examples && config.examples.length > 0) {
      description += `\n  Examples: ${config.examples.join(', ')}`
    }
    
    return description
  }

  /**
   * 生成Markdown格式描述
   */
  private static generateMarkdownDescription(config: ToolDescriptorConfig, context: AgentContext): string {
    let markdown = `### ${config.name}\n\n${config.description}\n`
    
    if (config.detailedDescription) {
      markdown += `\n${config.detailedDescription}\n`
    }
    
    if (config.usage) {
      markdown += `\n**Usage:** ${config.usage}\n`
    }
    
    if (config.examples && config.examples.length > 0) {
      markdown += `\n**Examples:**\n`
      for (const example of config.examples) {
        markdown += `- ${example}\n`
      }
    }
    
    return markdown
  }
}

/**
 * 便捷函数：过滤工具
 */
export function filterToolsByContext(tools: Tool[], context: AgentContext): Tool[] {
  return ToolDescriptor.filterToolsByContext(tools, context)
}

/**
 * 便捷函数：生成工具描述
 */
export function generateToolsDescription(
  tools: Tool[], 
  context: AgentContext,
  format: 'simple' | 'detailed' | 'markdown' = 'simple'
): string {
  return ToolDescriptor.generateToolsDescription(tools, context, format)
}
