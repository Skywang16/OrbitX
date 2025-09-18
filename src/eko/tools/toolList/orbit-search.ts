/**
 * Semantic code search tool
 *
 * Provides semantic code search functionality,
 * allowing natural language queries to find relevant code fragments.
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { ValidationError, ToolError } from '../tool-error'
import { ckApi, type CkSearchResult } from '@/api/ck'
import { terminalContextApi } from '@/api/terminal-context'

// ===== Type Definitions =====

interface OrbitSearchParams {
  query: string
  maxResults?: number
  path?: string
  mode?: 'semantic' | 'hybrid' | 'regex'
}

export interface OrbitSearchResponse {
  results: CkSearchResult[]
  totalFound: number
  query: string
  searchTime: number
}

/**
 * Semantic code search tool
 */
export class OrbitSearchTool extends ModifiableTool {
  constructor() {
    super(
      'orbit_search',
      `Search for code snippets in the current project. Describe the functionality you're looking for in natural language. Examples: "user authentication logic", "database connection config", "file upload handling". Returns the most relevant code fragments.`,
      {
        type: 'object',
        properties: {
          query: {
            type: 'string',
            description:
              'Natural language description of the code functionality you are looking for. Examples: "user login function", "config file loading", "error handling mechanism"',
          },
          maxResults: {
            type: 'number',
            description: 'Maximum number of results to return, defaults to 10, range 1-50',
            minimum: 1,
            maximum: 50,
          },
          path: {
            type: 'string',
            description: 'Optional path to search within, defaults to current working directory',
          },
          mode: {
            type: 'string',
            enum: ['semantic', 'hybrid', 'regex'],
            description: 'Search mode: semantic (default), hybrid (combines keywords + semantics), or regex',
          },
        },
        required: ['query'],
      }
    )
  }

  async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as OrbitSearchParams
    try {
      // 参数验证
      if (!params.query || params.query.trim().length === 0) {
        throw new ValidationError('Query cannot be empty')
      }
      if (params.query.trim().length < 3) {
        throw new ValidationError('Query must be at least 3 characters')
      }

      if (params.maxResults && (params.maxResults < 1 || params.maxResults > 50)) {
        throw new ValidationError('maxResults must be between 1 and 50')
      }

      // 获取搜索路径
      let searchPath = params.path
      if (!searchPath) {
        try {
          const terminalContext = await terminalContextApi.getActiveTerminalContext()
          searchPath = terminalContext?.currentWorkingDirectory || '.'
        } catch (error) {
          console.warn('无法获取终端上下文，使用当前工作目录')
          searchPath = '.'
        }
      }

      // 搜索模式
      const mode: 'semantic' | 'hybrid' | 'regex' = params.mode || 'semantic'

      // 对需要索引的模式提前检查索引是否就绪，避免依赖错误信息内容
      if (mode !== 'regex' && searchPath && searchPath !== '.') {
        try {
          const status = await ckApi.getIndexStatus({ path: searchPath })
          if (!status.isReady) {
            throw new ToolError(
              'No semantic index found. Please build an index first using the CK index button in the chat interface.',
              'INDEX_NOT_FOUND'
            )
          }
        } catch (e) {
          if (e instanceof ToolError) throw e
          // 如果索引状态检查失败，继续搜索流程，由后端返回更准确的错误
        }
      }

      // 执行CK搜索
      const startTime = Date.now()
      const searchResults = await ckApi.search({
        query: params.query.trim(),
        path: searchPath,
        mode,
        maxResults: params.maxResults || 10,
      })

      const searchTime = Date.now() - startTime

      // 格式化结果
      const response: OrbitSearchResponse = {
        results: searchResults,
        totalFound: searchResults.length,
        query: params.query.trim(),
        searchTime,
      }

      // 生成用户友好的结果摘要
      if (response.results.length === 0) {
        return {
          content: [
            {
              type: 'text',
              text: `No code found matching "${params.query}". Try using different keywords or check if the directory is indexed.`,
            },
          ],
        }
      }

      const summary = this.formatSearchSummary(response)
      const details = this.formatSearchDetails(response)

      return {
        content: [
          {
            type: 'text',
            text: `${summary}\n\n${details}`,
          },
        ],
      }
    } catch (error) {
      console.error('Orbit search failed:', error)

      if (error instanceof ValidationError) {
        throw error
      }

      // 处理CK相关错误（更稳健的处理已通过索引就绪预检查完成；此处保留兜底）
      if (error instanceof Error && /ck\s+not\s+found/i.test(error.message || '')) {
        throw new ToolError(
          'CK search engine not found. Please ensure ck-main is properly compiled and available.',
          'CK_NOT_FOUND'
        )
      }

      throw new ToolError(`Search failed: ${error instanceof Error ? error.message : String(error)}`, 'SEARCH_FAILED')
    }
  }

  private formatSearchSummary(response: OrbitSearchResponse): string {
    const { totalFound, query, searchTime } = response
    return `Found ${totalFound} code snippet${totalFound !== 1 ? 's' : ''} matching "${query}" (${searchTime}ms)`
  }

  private formatSearchDetails(response: OrbitSearchResponse): string {
    return response.results
      .map((result, index) => {
        const scoreText = result.score ? ` (${(result.score * 100).toFixed(1)}%)` : ''
        const location = `${result.filePath}:${result.startLine}-${result.endLine}`
        const snippet = result.content.length > 200 ? result.content.substring(0, 200) + '...' : result.content

        return `${index + 1}. ${location}${scoreText}\n   ${snippet.replace(/\n/g, '\n   ')}`
      })
      .join('\n\n')
  }
}

export const orbitSearchTool = new OrbitSearchTool()
