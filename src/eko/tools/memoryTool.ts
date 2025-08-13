/**
 * 内存管理工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { ValidationError } from './tool-error'
import { formatLocaleDateTime } from '@/utils/dateFormatter'

export interface MemoryEntry {
  key: string
  value: unknown
  type: string
  createdAt: Date
  updatedAt: Date
  expiresAt?: Date
  tags?: string[]
}

export interface MemorySetParams {
  key: string
  value: unknown
  ttl?: number
  tags?: string[]
}

export interface MemoryGetParams {
  key: string
}

export interface MemoryListParams {
  pattern?: string
  tags?: string[]
  includeExpired?: boolean
}

export interface MemoryDeleteParams {
  key?: string
  pattern?: string
  tags?: string[]
}

export interface MemoryStatsParams {
  detailed?: boolean
}

/**
 * 内存管理工具
 */
export class MemoryTool extends ModifiableTool {
  private memory = new Map<string, MemoryEntry>()

  constructor() {
    super(
      'memory',
      '🧠 内存管理：在Agent会话中存储和检索临时数据，支持过期时间、标签分类。用于存储上下文、缓存数据等',
      {
        type: 'object',
        properties: {
          operation: {
            type: 'string',
            enum: ['set', 'get', 'list', 'delete', 'clear', 'stats'],
            description: '操作类型',
          },
          key: {
            type: 'string',
            description: '数据键名（用于set、get、delete操作）',
          },
          value: {
            description: '要存储的值（用于set操作）',
          },
          ttl: {
            type: 'number',
            description: '生存时间（秒），用于set操作',
            minimum: 1,
          },
          tags: {
            type: 'array',
            items: { type: 'string' },
            description: '标签列表',
          },
          pattern: {
            type: 'string',
            description: '匹配模式（支持通配符*）',
          },
          includeExpired: {
            type: 'boolean',
            description: '是否包含过期的条目（用于list操作）',
            default: false,
          },
          detailed: {
            type: 'boolean',
            description: '是否显示详细统计信息（用于stats操作）',
            default: false,
          },
        },
        required: ['operation'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as { operation: string; [key: string]: unknown }

    switch (params.operation) {
      case 'set':
        return this.handleSet(params as unknown as MemorySetParams)
      case 'get':
        return this.handleGet(params as unknown as MemoryGetParams)
      case 'list':
        return this.handleList(params as unknown as MemoryListParams)
      case 'delete':
        return this.handleDelete(params as unknown as MemoryDeleteParams)
      case 'clear':
        return this.handleClear()
      case 'stats':
        return this.handleStats(params as unknown as MemoryStatsParams)
      default:
        throw new ValidationError(`不支持的操作: ${params.operation}`)
    }
  }

  private handleSet(params: MemorySetParams): ToolResult {
    if (!params.key) {
      throw new ValidationError('set操作需要key参数')
    }

    if (params.value === undefined) {
      throw new ValidationError('set操作需要value参数')
    }

    const now = new Date()
    const expiresAt = params.ttl ? new Date(now.getTime() + params.ttl * 1000) : undefined

    const entry: MemoryEntry = {
      key: params.key,
      value: params.value,
      type: this.getValueType(params.value),
      createdAt: now,
      updatedAt: now,
      expiresAt,
      tags: params.tags || [],
    }

    this.memory.set(params.key, entry)

    let resultText = `🧠 内存存储成功:\n`
    resultText += `- 键名: ${params.key}\n`
    resultText += `- 类型: ${entry.type}\n`
    resultText += `- 大小: ${this.getValueSize(params.value)}\n`

    if (expiresAt) {
      resultText += `- 过期时间: ${formatLocaleDateTime(expiresAt)}\n`
    }

    if (params.tags && params.tags.length > 0) {
      resultText += `- 标签: ${params.tags.join(', ')}\n`
    }

    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    }
  }

  private handleGet(params: MemoryGetParams): ToolResult {
    if (!params.key) {
      throw new ValidationError('get操作需要key参数')
    }

    const entry = this.memory.get(params.key)

    if (!entry) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 未找到键: ${params.key}`,
          },
        ],
      }
    }

    // 检查是否过期
    if (entry.expiresAt && entry.expiresAt < new Date()) {
      this.memory.delete(params.key)
      return {
        content: [
          {
            type: 'text',
            text: `⏰ 键已过期: ${params.key}`,
          },
        ],
      }
    }

    let resultText = `🧠 内存读取结果:\n`
    resultText += `- 键名: ${entry.key}\n`
    resultText += `- 类型: ${entry.type}\n`
    resultText += `- 创建时间: ${formatLocaleDateTime(entry.createdAt)}\n`
    resultText += `- 更新时间: ${formatLocaleDateTime(entry.updatedAt)}\n`

    if (entry.expiresAt) {
      resultText += `- 过期时间: ${entry.expiresAt.toLocaleString()}\n`
    }

    if (entry.tags && entry.tags.length > 0) {
      resultText += `- 标签: ${entry.tags.join(', ')}\n`
    }

    resultText += `\n📄 值内容:\n`
    resultText += this.formatValue(entry.value)

    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    }
  }

  private handleList(params: MemoryListParams): ToolResult {
    const now = new Date()
    const entries = Array.from(this.memory.values())

    let filteredEntries = entries.filter(entry => {
      // 检查过期
      if (!params.includeExpired && entry.expiresAt && entry.expiresAt < now) {
        return false
      }

      // 检查模式匹配
      if (params.pattern) {
        const regex = new RegExp(params.pattern.replace(/\*/g, '.*'))
        if (!regex.test(entry.key)) {
          return false
        }
      }

      // 检查标签匹配
      if (params.tags && params.tags.length > 0) {
        const hasMatchingTag = params.tags.some(tag => entry.tags?.includes(tag))
        if (!hasMatchingTag) {
          return false
        }
      }

      return true
    })

    // 排序
    filteredEntries.sort((a, b) => b.updatedAt.getTime() - a.updatedAt.getTime())

    let resultText = `🧠 内存条目列表 (${filteredEntries.length}/${entries.length}):\n\n`

    if (filteredEntries.length === 0) {
      resultText += '❌ 未找到匹配的条目'
    } else {
      for (const entry of filteredEntries) {
        const isExpired = entry.expiresAt && entry.expiresAt < now
        const status = isExpired ? '⏰' : '✅'

        resultText += `${status} ${entry.key} (${entry.type})`

        if (entry.tags && entry.tags.length > 0) {
          resultText += ` [${entry.tags.join(', ')}]`
        }

        if (entry.expiresAt) {
          resultText += ` - 过期: ${formatLocaleDateTime(entry.expiresAt)}`
        }

        resultText += `\n`
      }
    }

    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    }
  }

  private handleDelete(params: MemoryDeleteParams): ToolResult {
    let deletedCount = 0
    const deletedKeys: string[] = []

    if (params.key) {
      // 删除单个键
      if (this.memory.has(params.key)) {
        this.memory.delete(params.key)
        deletedCount = 1
        deletedKeys.push(params.key)
      }
    } else {
      // 批量删除
      const entries = Array.from(this.memory.entries())

      for (const [key, entry] of entries) {
        let shouldDelete = false

        // 检查模式匹配
        if (params.pattern) {
          const regex = new RegExp(params.pattern.replace(/\*/g, '.*'))
          if (regex.test(key)) {
            shouldDelete = true
          }
        }

        // 检查标签匹配
        if (params.tags && params.tags.length > 0) {
          const hasMatchingTag = params.tags.some(tag => entry.tags?.includes(tag))
          if (hasMatchingTag) {
            shouldDelete = true
          }
        }

        if (shouldDelete) {
          this.memory.delete(key)
          deletedCount++
          deletedKeys.push(key)
        }
      }
    }

    let resultText = `🧠 内存删除结果:\n`
    resultText += `- 删除数量: ${deletedCount}\n`

    if (deletedKeys.length > 0) {
      resultText += `- 删除的键: ${deletedKeys.join(', ')}\n`
    }

    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    }
  }

  private handleClear(): ToolResult {
    const count = this.memory.size
    this.memory.clear()

    return {
      content: [
        {
          type: 'text',
          text: `🧠 内存清空完成，已删除 ${count} 个条目`,
        },
      ],
    }
  }

  private handleStats(params: MemoryStatsParams): ToolResult {
    const now = new Date()
    const entries = Array.from(this.memory.values())

    const totalCount = entries.length
    const expiredCount = entries.filter(e => e.expiresAt && e.expiresAt < now).length
    const activeCount = totalCount - expiredCount

    // 按类型统计
    const typeStats = new Map<string, number>()
    for (const entry of entries) {
      typeStats.set(entry.type, (typeStats.get(entry.type) || 0) + 1)
    }

    // 按标签统计
    const tagStats = new Map<string, number>()
    for (const entry of entries) {
      if (entry.tags) {
        for (const tag of entry.tags) {
          tagStats.set(tag, (tagStats.get(tag) || 0) + 1)
        }
      }
    }

    let resultText = `🧠 内存统计信息:\n\n`
    resultText += `📊 总计: ${totalCount} 个条目\n`
    resultText += `✅ 活跃: ${activeCount} 个\n`
    resultText += `⏰ 过期: ${expiredCount} 个\n\n`

    if (params.detailed) {
      resultText += `📋 类型分布:\n`
      for (const [type, count] of typeStats) {
        resultText += `  ${type}: ${count}\n`
      }

      if (tagStats.size > 0) {
        resultText += `\n🏷️ 标签分布:\n`
        for (const [tag, count] of tagStats) {
          resultText += `  ${tag}: ${count}\n`
        }
      }

      // 内存使用估算
      const totalSize = entries.reduce((sum, entry) => sum + this.getValueSize(entry.value), 0)
      resultText += `\n💾 预估大小: ${this.formatSize(totalSize)}\n`
    }

    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    }
  }

  private getValueType(value: unknown): string {
    if (value === null) return 'null'
    if (Array.isArray(value)) return 'array'
    return typeof value
  }

  private getValueSize(value: unknown): number {
    // 简单的大小估算
    return JSON.stringify(value).length * 2 // UTF-16编码，每字符2字节
  }

  private formatSize(bytes: number): string {
    const sizes = ['B', 'KB', 'MB', 'GB']
    if (bytes === 0) return '0 B'
    const i = Math.floor(Math.log(bytes) / Math.log(1024))
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${sizes[i]}`
  }

  private formatValue(value: unknown): string {
    if (typeof value === 'string') {
      return value.length > 200 ? value.substring(0, 200) + '...' : value
    }

    const jsonStr = JSON.stringify(value, null, 2)
    return jsonStr.length > 500 ? jsonStr.substring(0, 500) + '...' : jsonStr
  }
}

// 导出工具实例
export const memoryTool = new MemoryTool()
