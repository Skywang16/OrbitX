/**
 * Eko原生工具管理系统
 */

import type { Tool as EkoTool } from '@eko-ai/eko/types'

// 导入所有工具
import { readFileTool } from './read-file'
import { createFileTool } from './create-file'

/**
 * 工具分类定义
 */
export enum ToolCategory {
  FILE = 'file',
  SYSTEM = 'system',
  NETWORK = 'network',
  SEARCH = 'search',
  ANALYSIS = 'analysis',
}

/**
 * 工具元数据
 */
export interface ToolMetadata {
  category: ToolCategory
  tags: string[]
  author?: string
  version?: string
  deprecated?: boolean
  safeMode?: boolean // 是否在安全模式下可用
}

/**
 * 已注册的工具
 */
export interface RegisteredTool {
  tool: EkoTool
  metadata: ToolMetadata
  registeredAt: Date
}

/**
 * Eko工具注册表
 */
export class EkoToolRegistry {
  private tools = new Map<string, RegisteredTool>()
  private categories = new Map<ToolCategory, Set<string>>()

  /**
   * 注册工具
   */
  register(tool: EkoTool, metadata: ToolMetadata): void {
    if (this.tools.has(tool.name)) {
      throw new Error(`工具已存在: ${tool.name}`)
    }

    const registeredTool: RegisteredTool = {
      tool,
      metadata,
      registeredAt: new Date(),
    }

    this.tools.set(tool.name, registeredTool)

    // 更新分类索引
    if (!this.categories.has(metadata.category)) {
      this.categories.set(metadata.category, new Set())
    }
    this.categories.get(metadata.category)!.add(tool.name)
  }

  /**
   * 批量注册工具
   */
  registerBatch(toolsToRegister: Array<{ tool: EkoTool; metadata: ToolMetadata }>): void {
    for (const { tool, metadata } of toolsToRegister) {
      this.register(tool, metadata)
    }
  }

  /**
   * 获取工具
   */
  getTool(name: string): EkoTool | undefined {
    return this.tools.get(name)?.tool
  }

  /**
   * 获取所有工具
   */
  getAllTools(): EkoTool[] {
    return Array.from(this.tools.values()).map(registered => registered.tool)
  }

  /**
   * 按分类获取工具
   */
  getToolsByCategory(category: ToolCategory): EkoTool[] {
    const toolNames = this.categories.get(category)
    if (!toolNames) return []

    return Array.from(toolNames)
      .map(name => this.tools.get(name)?.tool)
      .filter(Boolean) as EkoTool[]
  }

  /**
   * 按标签获取工具
   */
  getToolsByTag(tag: string): EkoTool[] {
    return Array.from(this.tools.values())
      .filter(registered => registered.metadata.tags.includes(tag))
      .map(registered => registered.tool)
  }

  /**
   * 获取安全模式工具
   */
  getSafeModeTools(): EkoTool[] {
    return Array.from(this.tools.values())
      .filter(registered => registered.metadata.safeMode === true)
      .map(registered => registered.tool)
  }

  /**
   * 获取非废弃工具
   */
  getActiveTools(): EkoTool[] {
    return Array.from(this.tools.values())
      .filter(registered => !registered.metadata.deprecated)
      .map(registered => registered.tool)
  }

  /**
   * 检查工具是否存在
   */
  hasTool(name: string): boolean {
    return this.tools.has(name)
  }

  /**
   * 获取工具元数据
   */
  getToolMetadata(name: string): ToolMetadata | undefined {
    return this.tools.get(name)?.metadata
  }

  /**
   * 获取工具统计信息
   */
  getStats(): {
    totalTools: number
    activeTools: number
    deprecatedTools: number
    safeModeTools: number
    categoryCounts: Record<string, number>
    tagCounts: Record<string, number>
  } {
    const totalTools = this.tools.size
    let activeTools = 0
    let deprecatedTools = 0
    let safeModeTools = 0
    const categoryCounts: Record<string, number> = {}
    const tagCounts: Record<string, number> = {}

    for (const registered of this.tools.values()) {
      if (registered.metadata.deprecated) {
        deprecatedTools++
      } else {
        activeTools++
      }

      if (registered.metadata.safeMode) {
        safeModeTools++
      }

      // 统计分类
      const category = registered.metadata.category
      categoryCounts[category] = (categoryCounts[category] || 0) + 1

      // 统计标签
      for (const tag of registered.metadata.tags) {
        tagCounts[tag] = (tagCounts[tag] || 0) + 1
      }
    }

    return {
      totalTools,
      activeTools,
      deprecatedTools,
      safeModeTools,
      categoryCounts,
      tagCounts,
    }
  }

  /**
   * 清空注册表
   */
  clear(): void {
    this.tools.clear()
    this.categories.clear()
  }
}

/**
 * 工具管理器
 */
export class EkoToolManager {
  private registry = new EkoToolRegistry()
  private initialized = false

  /**
   * 初始化工具系统
   */
  initialize(): void {
    if (this.initialized) return

    // 注册所有核心工具
    this.registerCoreTools()

    this.initialized = true
  }

  /**
   * 注册核心工具
   */
  private registerCoreTools(): void {
    const coreTools = [
      {
        tool: readFileTool,
        metadata: {
          category: ToolCategory.FILE,
          tags: ['file', 'read'],
          safeMode: true,
        },
      },
      {
        tool: createFileTool,
        metadata: {
          category: ToolCategory.FILE,
          tags: ['file', 'write'],
          safeMode: false, // 写入操作不在安全模式
        },
      },
    ]

    this.registry.registerBatch(coreTools)
  }

  /**
   * 获取所有工具
   */
  getAllTools(): EkoTool[] {
    this.ensureInitialized()
    return this.registry.getAllTools()
  }

  /**
   * 按模式获取工具
   */
  getToolsForMode(mode: 'safe' | 'full'): EkoTool[] {
    this.ensureInitialized()

    if (mode === 'safe') {
      return this.registry.getSafeModeTools()
    } else {
      return this.registry.getActiveTools()
    }
  }

  /**
   * 按分类获取工具
   */
  getToolsByCategory(category: ToolCategory): EkoTool[] {
    this.ensureInitialized()
    return this.registry.getToolsByCategory(category)
  }

  /**
   * 获取特定工具
   */
  getTool(name: string): EkoTool | undefined {
    this.ensureInitialized()
    return this.registry.getTool(name)
  }

  /**
   * 注册自定义工具
   */
  registerTool(tool: EkoTool, metadata: ToolMetadata): void {
    this.ensureInitialized()
    this.registry.register(tool, metadata)
  }

  /**
   * 获取工具统计信息
   */
  getStats(): ReturnType<typeof this.registry.getStats> {
    this.ensureInitialized()
    return this.registry.getStats()
  }

  /**
   * 验证工具系统
   */
  validateTools(): {
    valid: boolean
    errors: string[]
    warnings: string[]
    stats: {
      totalTools: number
      activeTools: number
      deprecatedTools: number
      safeModeTools: number
      categoryCounts: Record<string, number>
      tagCounts: Record<string, number>
    }
  } {
    this.ensureInitialized()

    const errors: string[] = []
    const warnings: string[] = []
    const allTools = this.registry.getAllTools()

    // 检查工具名称唯一性
    const names = new Set<string>()
    for (const tool of allTools) {
      if (names.has(tool.name)) {
        errors.push(`重复的工具名称: ${tool.name}`)
      }
      names.add(tool.name)
    }

    // 检查工具完整性
    for (const tool of allTools) {
      if (!tool.name || tool.name.trim() === '') {
        errors.push('发现工具名称为空')
      }

      if (!tool.description || tool.description.trim() === '') {
        warnings.push(`工具 ${tool.name} 缺少描述`)
      }

      if (!tool.parameters) {
        warnings.push(`工具 ${tool.name} 缺少参数定义`)
      }

      if (typeof tool.execute !== 'function') {
        errors.push(`工具 ${tool.name} 缺少execute方法`)
      }
    }

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      stats: this.registry.getStats(),
    }
  }

  /**
   * 生成 LLM 友好的工具文档
   */
  generateDocumentation(): string {
    this.ensureInitialized()

    // 只获取活跃的（非废弃）工具
    const activeTools = this.registry.getActiveTools()

    if (activeTools.length === 0) {
      return '无可用工具'
    }

    let doc = '可用工具:\n\n'

    for (const tool of activeTools) {
      doc += `${tool.name}: ${tool.description}\n`

      // 简化参数信息
      if (tool.parameters?.properties) {
        const params = Object.entries(tool.parameters.properties as Record<string, unknown>)
        if (params.length > 0) {
          const paramList = params
            .map(([name]) => {
              const required = tool.parameters.required?.includes(name) ? '*' : ''
              return `${name}${required}`
            })
            .join(', ')
          doc += `参数: ${paramList}\n`
        }
      }
      doc += '\n'
    }

    return doc.trim()
  }

  /**
   * 确保已初始化
   */
  private ensureInitialized(): void {
    if (!this.initialized) {
      this.initialize()
    }
  }
}

// 导出全局工具管理器实例
export const ekoToolManager = new EkoToolManager()

// 便捷函数
export function getAllTools(): EkoTool[] {
  return ekoToolManager.getAllTools()
}

export function getTool(name: string): EkoTool | undefined {
  return ekoToolManager.getTool(name)
}

export function getToolsForMode(mode: 'safe' | 'full'): EkoTool[] {
  return ekoToolManager.getToolsForMode(mode)
}

export function getToolsByCategory(category: ToolCategory): EkoTool[] {
  return ekoToolManager.getToolsByCategory(category)
}

export function registerTool(tool: EkoTool, metadata: ToolMetadata): void {
  ekoToolManager.registerTool(tool, metadata)
}

export function validateTools(): ReturnType<typeof ekoToolManager.validateTools> {
  return ekoToolManager.validateTools()
}

export function generateToolsDocumentation(): string {
  return ekoToolManager.generateDocumentation()
}

// 自动初始化
ekoToolManager.initialize()
