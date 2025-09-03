/**
 * 工具注册管理系统
 */

import type { Tool } from '../types'
import { ToolError } from './tool-error'

export interface ToolMetadata {
  name: string
  description: string
  category: string
  author?: string
  tags?: string[]
}

export interface RegisteredTool {
  tool: Tool
  metadata: ToolMetadata
  registeredAt: Date
}

/**
 * 工具注册表
 */
export class ToolRegistry {
  private tools = new Map<string, RegisteredTool>()
  private categories = new Map<string, Set<string>>()

  /**
   * 注册工具
   */
  register(tool: Tool, metadata: Omit<ToolMetadata, 'name'>): void {
    if (this.tools.has(tool.name)) {
      throw new ToolError(`工具已存在: ${tool.name}`, 'TOOL_ALREADY_EXISTS')
    }

    const fullMetadata: ToolMetadata = {
      name: tool.name,
      ...metadata,
    }

    const registeredTool: RegisteredTool = {
      tool,
      metadata: fullMetadata,
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
  registerBatch(toolsToRegister: Array<{ tool: Tool; metadata: Omit<ToolMetadata, 'name'> }>): void {
    for (const { tool, metadata } of toolsToRegister) {
      this.register(tool, metadata)
    }
  }

  /**
   * 获取工具
   */
  get(name: string): Tool | undefined {
    return this.tools.get(name)?.tool
  }

  /**
   * 获取工具元数据
   */
  getMetadata(name: string): ToolMetadata | undefined {
    return this.tools.get(name)?.metadata
  }

  /**
   * 获取注册信息
   */
  getRegisteredTool(name: string): RegisteredTool | undefined {
    return this.tools.get(name)
  }

  /**
   * 检查工具是否存在
   */
  has(name: string): boolean {
    return this.tools.has(name)
  }

  /**
   * 注销工具
   */
  unregister(name: string): boolean {
    const registeredTool = this.tools.get(name)
    if (!registeredTool) return false

    this.tools.delete(name)

    // 更新分类索引
    const category = registeredTool.metadata.category
    const categoryTools = this.categories.get(category)
    if (categoryTools) {
      categoryTools.delete(name)
      if (categoryTools.size === 0) {
        this.categories.delete(category)
      }
    }

    return true
  }

  /**
   * 获取所有工具名称
   */
  list(): string[] {
    return Array.from(this.tools.keys())
  }

  /**
   * 获取所有工具
   */
  getAllTools(): Tool[] {
    return Array.from(this.tools.values()).map(registered => registered.tool)
  }

  /**
   * 按分类获取工具
   */
  getByCategory(category: string): Tool[] {
    const toolNames = this.categories.get(category)
    if (!toolNames) return []

    return Array.from(toolNames)
      .map(name => this.tools.get(name)?.tool)
      .filter(Boolean) as Tool[]
  }

  /**
   * 获取所有分类
   */
  getCategories(): string[] {
    return Array.from(this.categories.keys())
  }

  /**
   * 按标签搜索工具
   */
  searchByTag(tag: string): Tool[] {
    return Array.from(this.tools.values())
      .filter(registered => registered.metadata.tags?.includes(tag))
      .map(registered => registered.tool)
  }

  /**
   * 搜索工具（按名称或描述）
   */
  search(query: string): Tool[] {
    const lowerQuery = query.toLowerCase()
    return Array.from(this.tools.values())
      .filter(
        registered =>
          registered.metadata.name.toLowerCase().includes(lowerQuery) ||
          registered.metadata.description.toLowerCase().includes(lowerQuery)
      )
      .map(registered => registered.tool)
  }


  /**
   * 清空注册表
   */
  clear(): void {
    this.tools.clear()
    this.categories.clear()
  }

  /**
   * 获取统计信息
   */
  getStats(): {
    totalTools: number
    totalCategories: number
    activeTools: number
    toolsByCategory: Record<string, number>
  } {
    const totalTools = this.tools.size
    const totalCategories = this.categories.size
    const activeTools = totalTools

    const toolsByCategory: Record<string, number> = {}
    for (const [category, toolNames] of this.categories) {
      toolsByCategory[category] = toolNames.size
    }

    return {
      totalTools,
      totalCategories,
      activeTools,
      toolsByCategory,
    }
  }
}

// 全局工具注册表实例
export const globalToolRegistry = new ToolRegistry()
