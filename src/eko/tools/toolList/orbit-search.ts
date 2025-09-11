/**
 * Semantic code search tool - Vector-based code snippet search
 *
 * Provides semantic code search using vector embeddings,
 * allowing natural language queries to find relevant code fragments.
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { ValidationError, ToolError } from '../tool-error'
import { ckApi } from '@/api/workspace-index'
import type { VectorSearchResult } from '@/api/workspace-index'
import { terminalContextApi } from '@/api/terminal-context'
import { windowApi } from '@/api/window'

// ===== Type Definitions =====

interface OrbitSearchParams {
  query: string
  maxResults?: number
  minScore?: number
  directoryFilter?: string
  languageFilter?: string
}

export interface OrbitSearchResponse {
  results: VectorSearchResult[]
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
      `Search for code snippets in the current project using semantic vector search. Describe the functionality you're looking for in natural language. Examples: "user authentication logic", "database connection config", "file upload handling". Returns the most relevant code fragments based on semantic similarity.`,
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
          },
          minScore: {
            type: 'number',
            description:
              'Minimum similarity threshold (0-1), defaults to 0.3. Only results above this score will be returned',
          },
          directoryFilter: {
            type: 'string',
            description:
              'Limit search to specific directory path. Examples: "src/components", "api". If omitted, searches entire project',
          },
          languageFilter: {
            type: 'string',
            description: 'Limit search to specific programming language. Examples: "typescript", "rust", "python"',
          },
        },
        required: ['query'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as OrbitSearchParams
    const { query, maxResults = 10, minScore = 0.3, directoryFilter, languageFilter } = params

    // Parameter validation
    if (!query || query.trim().length < 3) {
      throw new ValidationError('Query must be at least 3 characters')
    }

    if (maxResults < 1 || maxResults > 50) {
      throw new ValidationError('maxResults must be between 1-50')
    }

    if (minScore < 0 || minScore > 1) {
      throw new ValidationError('minScore must be between 0-1')
    }

    try {
      const startTime = Date.now()

      // Resolve working directory (prefer active terminal CWD, fallback to app current dir)
      let cwd = ''
      try {
        const ctx = await terminalContextApi.getActiveTerminalContext()
        cwd = ctx.currentWorkingDirectory || ''
      } catch (e) {
        console.warn('orbit-search: no active terminal context', e)
      }
      if (!cwd) {
        try {
          cwd = await windowApi.getCurrentDirectory({ useCache: true })
        } catch (e) {
          console.warn('orbit-search: getCurrentDirectory fallback failed', e)
        }
      }

      // Build search params for ck
      let resolvedDirectory = cwd
      if (directoryFilter && directoryFilter.trim()) {
        const dirf = directoryFilter.trim()
        try {
          resolvedDirectory = cwd ? await windowApi.joinPaths(cwd, dirf) : dirf
        } catch {
          resolvedDirectory = dirf
        }
      }

      const ckParams = {
        query: query.trim(),
        maxResults,
        minScore,
        directory: resolvedDirectory,
        languageFilter: languageFilter?.trim() || undefined,
        mode: 'semantic' as const,
      }

      const searchOptions = {
        minScore,
        directoryFilter,
        languageFilter,
      }

      // Execute search through ck API
      const searchResults = await ckApi.search(ckParams)

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

      // Format search results
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

      // Handle specific error types
      if (error instanceof ValidationError) {
        throw error
      }

      // Handle backend errors
      const errorMessage = error instanceof Error ? error.message : String(error)

      if (errorMessage.includes('Vector index service not initialized')) {
        return {
          content: [
            {
              type: 'text',
              text: 'Vector index service not initialized. Please configure the vector index and build the code index in settings.',
            },
          ],
        }
      }

      if (errorMessage.includes('index')) {
        return {
          content: [
            {
              type: 'text',
              text: `Vector index error: ${errorMessage}. Please check if the code index needs to be built or rebuilt.`,
            },
          ],
        }
      }

      throw new ToolError(`Code search failed: ${errorMessage}`)
    }
  }

  // Status check is handled through vectorIndexApi.getStatus(), no local methods needed

  /**
   * Format no results message
   */
  private formatNoResultsMessage(
    query: string,
    options: { minScore?: number; directoryFilter?: string; languageFilter?: string }
  ): string {
    let message = `No code snippets found related to "${query}".\n\n`

    message += 'Suggestions:\n'
    message += '• Try using more general terms like "config", "auth", "handler"\n'
    message += '• Lower similarity threshold (minScore), current: ' + options.minScore + '\n'
    message += '• Check if filters are limiting the search scope\n'

    if (options.directoryFilter) {
      message += `• Current directory filter: ${options.directoryFilter}\n`
    }

    if (options.languageFilter) {
      message += `• Current language filter: ${options.languageFilter}\n`
    }

    message += '\nIf the issue persists, consider rebuilding the code index.'

    return message
  }

  /**
   * Format search results
   */
  private formatSearchResults(response: OrbitSearchResponse): string {
    const { results, totalFound, query, searchTime } = response

    let output = `🔍 **Code Search Results** (Query: "${query}")\n\n`
    output += `Found ${totalFound} relevant code snippets in ${searchTime}ms\n\n`

    // Group results by similarity score
    const highScoreResults = results.filter(r => r.score >= 0.7)
    const mediumScoreResults = results.filter(r => r.score >= 0.5 && r.score < 0.7)
    const lowScoreResults = results.filter(r => r.score < 0.5)

    if (highScoreResults.length > 0) {
      output += `### 🎯 High Relevance Matches (${highScoreResults.length})\n\n`
      output += this.formatResultSection(highScoreResults)
    }

    if (mediumScoreResults.length > 0) {
      output += `### 📋 Medium Relevance Matches (${mediumScoreResults.length})\n\n`
      output += this.formatResultSection(mediumScoreResults)
    }

    if (lowScoreResults.length > 0) {
      output += `### 📌 Low Relevance Matches (${lowScoreResults.length})\n\n`
      output += this.formatResultSection(lowScoreResults.slice(0, 3)) // Show only first 3
    }

    // Add usage tips
    output += '\n---\n\n'
    output += '💡 **Usage Tips**:\n'
    output += '• Use `directoryFilter` parameter to limit search directory\n'
    output += '• Use `languageFilter` parameter to limit programming language\n'
    output += '• Use `minScore` parameter to adjust similarity threshold\n'

    return output
  }

  /**
   * Format result section
   */
  private formatResultSection(results: VectorSearchResult[]): string {
    return results
      .map((result, index) => {
        const { filePath, content, startLine, endLine, language, chunkType, score } = result

        // 生成相对路径显示
        const displayPath = this.getDisplayPath(filePath)

        let section = `**${index + 1}. ${displayPath}** `
        section += `(${language}, lines ${startLine}-${endLine})\n`
        section += `Similarity: ${(score * 100).toFixed(1)}% | Type: ${chunkType}\n\n`

        // Format code content
        const codeBlock = '```' + language + '\n' + content.trim() + '\n```\n\n'
        section += codeBlock

        return section
      })
      .join('---\n\n')
  }

  /**
   * Generate display path
   */
  private getDisplayPath(fullPath: string): string {
    // Simplify path display, prioritize project-relative paths
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

    // If no project indicators found, show last 3 directory levels
    return pathParts.slice(-3).join('/')
  }
}

// Export tool instance
export const orbitSearchTool = new OrbitSearchTool()
