/**
 * 代码向量搜索工具 - 基于语义相似度的代码片段搜索
 *
 * 通过向量化技术和Qdrant数据库，实现基于语义相似度的代码搜索，
 * 支持自然语言描述查询相关代码片段。
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { ValidationError, ToolError } from '../tool-error'
import { vectorIndexApi } from '@/api'
import type { VectorSearchOptions, VectorSearchResult } from '@/api/vector-index'

// ===== 类型定义（统一复用 API 层） =====

type CodeSearchParams = VectorSearchOptions

export interface CodeSearchResponse {
  results: VectorSearchResult[]
  totalFound: number
  query: string
  searchTime: number
}

/**
 * 代码向量搜索工具
 *
 * 核心特性：
 * 1. 语义搜索：基于向量相似度的智能代码搜索
 * 2. 自然语言查询：支持用自然语言描述查找代码功能
 * 3. 多种过滤：支持目录、编程语言、相似度阈值过滤
 * 4. 丰富结果：返回代码片段、文件信息、相似度评分
 */
export class CodeSearchTool extends ModifiableTool {
  constructor() {
    super(
      'search_code',
      `在当前项目中搜索相关代码片段，支持语义搜索。使用自然语言描述你要找的代码功能，例如："用户认证逻辑"、"数据库连接配置"、"文件上传处理"等。工具会通过向量相似度找到最相关的代码片段。`,
      {
        type: 'object',
        properties: {
          query: {
            type: 'string',
            description:
              '要搜索的代码描述或功能描述，支持自然语言。例如："用户登录功能"、"配置文件加载"、"错误处理机制"等',
          },
          maxResults: {
            type: 'number',
            description: '最大返回结果数量，默认10，范围1-50',
            minimum: 1,
            maximum: 50,
            default: 10,
          },
          minScore: {
            type: 'number',
            description: '最小相似度阈值(0-1)，默认0.3，高于此分数的结果才会返回',
            minimum: 0,
            maximum: 1,
            default: 0.3,
          },
          directoryFilter: {
            type: 'string',
            description: '限制搜索的目录路径，例如："src/components"、"api"等，不填则搜索整个项目',
          },
          languageFilter: {
            type: 'string',
            description: '限制搜索的编程语言，例如："typescript"、"rust"、"python"等',
          },
        },
        required: ['query'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as CodeSearchParams
    const { query, maxResults = 10, minScore = 0.3, directoryFilter, languageFilter } = params

    // 参数验证
    if (!query || query.trim().length < 3) {
      throw new ValidationError('搜索查询至少需要3个字符')
    }

    if (maxResults < 1 || maxResults > 50) {
      throw new ValidationError('maxResults 必须在1-50之间')
    }

    if (minScore < 0 || minScore > 1) {
      throw new ValidationError('minScore 必须在0-1之间')
    }

    try {
      const startTime = Date.now()

      // 检查向量索引服务状态（统一 API）
      const indexStatus = await vectorIndexApi.getStatus()
      if (!indexStatus.isInitialized) {
        return {
          content: [
            {
              type: 'text',
              text: '代码向量索引未初始化。请先在设置中配置Qdrant数据库连接并构建代码索引。',
            },
          ],
        }
      }

      // 构建搜索选项（API 类型）
      const searchOptions: VectorSearchOptions = {
        query: query.trim(),
        maxResults,
        minScore,
        directoryFilter: directoryFilter?.trim() || undefined,
        languageFilter: languageFilter?.trim() || undefined,
      }

      // 统一通过 API 执行向量搜索
      const searchResults = await vectorIndexApi.search(searchOptions)

      const searchTime = Date.now() - startTime

      if (searchResults.length === 0) {
        return {
          content: [
            {
              type: 'text',
              text: this.formatNoResultsMessage(query, searchOptions),
            },
          ],
        }
      }

      // 格式化搜索结果
      const formattedResults = this.formatSearchResults({
        results: searchResults,
        totalFound: searchResults.length,
        query,
        searchTime,
      })

      return {
        content: [
          {
            type: 'text',
            text: formattedResults,
          },
        ],
      }
    } catch (error) {
      console.error('Code search failed:', error)

      // 处理特定错误类型
      if (error instanceof ValidationError) {
        throw error
      }

      // 处理后端错误
      const errorMessage = error instanceof Error ? error.message : String(error)

      if (errorMessage.includes('Vector index service not initialized')) {
        return {
          content: [
            {
              type: 'text',
              text: '代码向量索引服务未初始化。请先在设置中配置向量索引并构建代码索引。',
            },
          ],
        }
      }

      if (errorMessage.includes('Qdrant')) {
        return {
          content: [
            {
              type: 'text',
              text: `Qdrant数据库连接失败: ${errorMessage}。请检查数据库配置和连接状态。`,
            },
          ],
        }
      }

      throw new ToolError(`代码搜索失败: ${errorMessage}`)
    }
  }

  // 统一状态检查已通过 vectorIndexApi.getStatus() 完成，无需本地方法

  /**
   * 格式化无结果消息
   */
  private formatNoResultsMessage(
    query: string,
    options: { minScore?: number; directoryFilter?: string; languageFilter?: string }
  ): string {
    let message = `未找到与 "${query}" 相关的代码片段。\n\n`

    message += '建议：\n'
    message += '• 尝试使用更通用的描述，如"配置"、"认证"、"处理"等\n'
    message += '• 降低相似度阈值(minScore)，当前: ' + options.minScore + '\n'
    message += '• 检查是否有过滤条件限制了搜索范围\n'

    if (options.directoryFilter) {
      message += `• 当前目录过滤: ${options.directoryFilter}\n`
    }

    if (options.languageFilter) {
      message += `• 当前语言过滤: ${options.languageFilter}\n`
    }

    message += '\n如果问题持续，可能需要重新构建代码索引。'

    return message
  }

  /**
   * 格式化搜索结果
   */
  private formatSearchResults(response: CodeSearchResponse): string {
    const { results, totalFound, query, searchTime } = response

    let output = `🔍 **代码搜索结果** (查询: "${query}")\n\n`
    output += `找到 ${totalFound} 个相关代码片段，搜索用时 ${searchTime}ms\n\n`

    // 按相似度分组显示结果
    const highScoreResults = results.filter(r => r.score >= 0.7)
    const mediumScoreResults = results.filter(r => r.score >= 0.5 && r.score < 0.7)
    const lowScoreResults = results.filter(r => r.score < 0.5)

    if (highScoreResults.length > 0) {
      output += `### 🎯 高相关度匹配 (${highScoreResults.length}个)\n\n`
      output += this.formatResultSection(highScoreResults)
    }

    if (mediumScoreResults.length > 0) {
      output += `### 📋 中等相关度匹配 (${mediumScoreResults.length}个)\n\n`
      output += this.formatResultSection(mediumScoreResults)
    }

    if (lowScoreResults.length > 0) {
      output += `### 📌 低相关度匹配 (${lowScoreResults.length}个)\n\n`
      output += this.formatResultSection(lowScoreResults.slice(0, 3)) // 只显示前3个
    }

    // 添加使用提示
    output += '\n---\n\n'
    output += '💡 **使用提示**:\n'
    output += '• 可通过 `directoryFilter` 参数限制搜索目录\n'
    output += '• 可通过 `languageFilter` 参数限制编程语言\n'
    output += '• 可通过 `minScore` 参数调整相似度阈值\n'

    return output
  }

  /**
   * 格式化结果段落
   */
  private formatResultSection(results: VectorSearchResult[]): string {
    return results
      .map((result, index) => {
        const { filePath, content, startLine, endLine, language, chunkType, score } = result

        // 生成相对路径显示
        const displayPath = this.getDisplayPath(filePath)

        let section = `**${index + 1}. ${displayPath}** `
        section += `(${language}, 行 ${startLine}-${endLine})\n`
        section += `相似度: ${(score * 100).toFixed(1)}% | 类型: ${chunkType}\n\n`

        // 格式化代码内容
        const codeBlock = '```' + language + '\n' + content.trim() + '\n```\n\n'
        section += codeBlock

        return section
      })
      .join('---\n\n')
  }

  /**
   * 生成显示用的相对路径
   */
  private getDisplayPath(fullPath: string): string {
    // 简化路径显示，优先显示相对于项目的路径
    const pathParts = fullPath.split('/')
    const projectIndicators = ['src', 'lib', 'components', 'pages', 'api', 'utils']

    let startIndex = -1
    for (let i = pathParts.length - 1; i >= 0; i--) {
      if (projectIndicators.includes(pathParts[i])) {
        startIndex = i
        break
      }
    }

    if (startIndex !== -1) {
      return pathParts.slice(startIndex).join('/')
    }

    // 如果没找到项目指示符，显示最后3级目录
    return pathParts.slice(-3).join('/')
  }
}

// 导出工具实例
export const codeSearchTool = new CodeSearchTool()
