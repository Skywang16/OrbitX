/**
 * 网络搜索工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { NetworkError, ValidationError } from './tool-error'

export interface WebSearchParams {
  query: string
  searchEngine?: 'google' | 'bing' | 'duckduckgo'
  maxResults?: number
  language?: string
  region?: string
  timeRange?: 'day' | 'week' | 'month' | 'year' | 'all'
  safeSearch?: 'strict' | 'moderate' | 'off'
}

export interface SearchResult {
  title: string
  url: string
  snippet: string
  displayUrl?: string
  thumbnail?: string
}

export interface WebSearchResponse {
  query: string
  searchEngine: string
  totalResults: number
  searchTime: number
  results: SearchResult[]
}

/**
 * 网络搜索工具
 */
export class WebSearchTool extends ModifiableTool {
  constructor() {
    super(
      'web_search',
      '🔍 网络搜索：在互联网上搜索信息，支持多个搜索引擎、语言地区设置。用于获取最新信息、查找资料等',
      {
        type: 'object',
        properties: {
          query: {
            type: 'string',
            description: '搜索查询词',
            minLength: 1,
          },
          searchEngine: {
            type: 'string',
            enum: ['google', 'bing', 'duckduckgo'],
            description: '搜索引擎，默认google',
            default: 'google',
          },
          maxResults: {
            type: 'number',
            description: '最大结果数量，默认10',
            default: 10,
            minimum: 1,
            maximum: 50,
          },
          language: {
            type: 'string',
            description: '搜索语言，默认zh-CN',
            default: 'zh-CN',
          },
          region: {
            type: 'string',
            description: '搜索地区，默认CN',
            default: 'CN',
          },
          timeRange: {
            type: 'string',
            enum: ['day', 'week', 'month', 'year', 'all'],
            description: '时间范围，默认all',
            default: 'all',
          },
          safeSearch: {
            type: 'string',
            enum: ['strict', 'moderate', 'off'],
            description: '安全搜索级别，默认moderate',
            default: 'moderate',
          },
        },
        required: ['query'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { query, searchEngine = 'google', maxResults = 10 } = context.parameters as unknown as WebSearchParams

    // 验证查询
    this.validateQuery(query)

    const startTime = Date.now()

    try {
      let results: SearchResult[]

      switch (searchEngine) {
        case 'google':
          results = await this.searchGoogle(query, maxResults, 'zh-CN', 'CN', 'all', 'moderate')
          break
        case 'bing':
          results = await this.searchBing(query, maxResults, 'zh-CN', 'CN', 'all', 'moderate')
          break
        case 'duckduckgo':
          results = await this.searchDuckDuckGo(query, maxResults, 'zh-CN', 'CN')
          break
        default:
          throw new ValidationError(`不支持的搜索引擎: ${searchEngine}`)
      }

      const searchTime = Date.now() - startTime

      const response: WebSearchResponse = {
        query,
        searchEngine,
        totalResults: results.length,
        searchTime,
        results,
      }

      // 格式化输出
      let resultText = `🔍 网络搜索结果:\n\n`
      resultText += `📝 查询: ${query}\n`
      resultText += `🌐 搜索引擎: ${searchEngine}\n`
      resultText += `📊 找到结果: ${response.totalResults} 条\n`
      resultText += `⏱️ 搜索时间: ${response.searchTime}ms\n\n`

      if (results.length === 0) {
        resultText += '❌ 未找到相关结果'
      } else {
        resultText += '📋 搜索结果:\n\n'

        for (let i = 0; i < results.length; i++) {
          const result = results[i]
          resultText += `${i + 1}. **${result.title}**\n`
          resultText += `   🔗 ${result.url}\n`
          if (result.snippet) {
            resultText += `   📄 ${result.snippet}\n`
          }
          resultText += '\n'
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
    } catch (error) {
      if (error instanceof ValidationError || error instanceof NetworkError) {
        throw error
      }
      throw new NetworkError(`搜索失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private validateQuery(query: string): void {
    if (!query || query.trim() === '') {
      throw new ValidationError('搜索查询不能为空')
    }

    if (query.length > 200) {
      throw new ValidationError('搜索查询过长，最多200个字符')
    }

    // 检查是否包含恶意内容
    const maliciousPatterns = [
      'site:localhost',
      'site:127.0.0.1',
      'site:192.168.',
      'site:10.',
      'filetype:exe',
      'filetype:dll',
    ]

    for (const pattern of maliciousPatterns) {
      if (query.toLowerCase().includes(pattern)) {
        throw new ValidationError(`搜索查询包含不允许的内容: ${pattern}`)
      }
    }
  }

  private async searchGoogle(
    query: string,
    maxResults: number,
    _language: string,
    _region: string,
    _timeRange: string,
    _safeSearch: string
  ): Promise<SearchResult[]> {
    // 注意：这里使用模拟的搜索结果，实际实现需要集成真实的搜索API
    // 真实实现应该使用Google Custom Search API或其他搜索服务

    const mockResults: SearchResult[] = [
      {
        title: `关于 "${query}" 的搜索结果`,
        url: `https://www.google.com/search?q=${encodeURIComponent(query)}`,
        snippet: `这是一个模拟的搜索结果。实际实现需要集成真实的搜索API来获取真实的搜索结果。`,
        displayUrl: 'google.com',
      },
    ]

    // 模拟网络延迟
    await this.sleep(100)

    return mockResults.slice(0, maxResults)
  }

  private async searchBing(
    query: string,
    maxResults: number,
    _language: string,
    _region: string,
    _timeRange: string,
    _safeSearch: string
  ): Promise<SearchResult[]> {
    // 注意：这里使用模拟的搜索结果，实际实现需要集成Bing Search API

    const mockResults: SearchResult[] = [
      {
        title: `Bing搜索: "${query}"`,
        url: `https://www.bing.com/search?q=${encodeURIComponent(query)}`,
        snippet: `这是一个模拟的Bing搜索结果。实际实现需要集成Bing Search API来获取真实的搜索结果。`,
        displayUrl: 'bing.com',
      },
    ]

    await this.sleep(120)

    return mockResults.slice(0, maxResults)
  }

  private async searchDuckDuckGo(
    query: string,
    maxResults: number,
    _language: string,
    _region: string
  ): Promise<SearchResult[]> {
    // 注意：这里使用模拟的搜索结果，实际实现可以使用DuckDuckGo的API

    const mockResults: SearchResult[] = [
      {
        title: `DuckDuckGo搜索: "${query}"`,
        url: `https://duckduckgo.com/?q=${encodeURIComponent(query)}`,
        snippet: `这是一个模拟的DuckDuckGo搜索结果。实际实现需要集成DuckDuckGo API来获取真实的搜索结果。`,
        displayUrl: 'duckduckgo.com',
      },
    ]

    await this.sleep(80)

    return mockResults.slice(0, maxResults)
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }
}

// 导出工具实例
export const webSearchTool = new WebSearchTool()
