/**
 * 语义搜索工具 - 整合文本搜索、AST分析和语义理解的强大搜索引擎
 *
 * 融合了 orbit_context 的动态搜索能力和 analyze-code 的结构分析能力
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { FileNotFoundError, ValidationError, ToolError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'
import { aiApi, type AnalyzeCodeParams, type AnalysisResult, type CodeSymbol } from '@/api'

// ===== 类型定义 =====

export interface orbitSearchParams {
  query: string
  path: string
}

export interface SearchMatch {
  filePath: string
  lineNumber: number
  line: string
  relevanceScore: number
  context: {
    before: string[]
    after: string[]
  }
  highlightedLine: string
}

export interface SymbolMatch {
  symbol: CodeSymbol
  filePath: string
  relevanceScore: number
  context: string[]
}

export interface orbitSearchResponse {
  query: string[]
  targetPath: string
  totalMatches: number
  filesSearched: number
  searchTime: number
  textMatches: SearchMatch[]
  symbolMatches: SymbolMatch[]
  summary: string
}

/**
 * 语义搜索工具
 *
 * 核心特性：
 * 1. 多模式搜索：文本、语义、代码结构、符号定义
 * 2. 智能相关性评分：结合文本匹配度、符号重要性、上下文相关性
 * 3. 结构化结果：按类型、重要性、文件组织搜索结果
 * 4. 上下文理解：提供丰富的代码上下文和建议
 */
export class OrbitSearchTool extends ModifiableTool {
  constructor() {
    super(
      'orbit_search',
      `Code search tool for finding specific code elements like functions, classes, variables, and patterns. Use specific code keywords, not natural language descriptions. Examples: search "Agent" to find Agent classes, "execute" to find execution methods, "workflow" to find workflow-related code. Supports single keyword or multi-keyword OR search (e.g., "Agent OR workflow"). Must use absolute paths.`,
      {
        type: 'object',
        properties: {
          query: {
            type: 'string',
            description:
              'Search query content. Supports single keyword or multi-keyword OR search. For example: "createUser" or "gemini-cli OR xxxgemini OR gemini"',
          },
          path: {
            type: 'string',
            description:
              'Absolute path for the search directory. Must be a complete path, for example: "/Users/user/project/src", "/home/user/workspace"',
          },
        },
        required: ['query', 'path'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as orbitSearchParams
    const { query, path } = params

    // 解析查询参数
    const keywords = this.parseQuery(query)
    if (keywords.length === 0) {
      throw new ValidationError('搜索查询不能为空')
    }

    if (!path || path.trim() === '') {
      throw new ValidationError('搜索路径不能为空')
    }

    try {
      const startTime = Date.now()
      const searchPath = await this.resolveSearchPath(path)
      const searchResult = await this.performorbitSearch({
        query: keywords,
        searchPath,
        fileExtensions: undefined,
        maxResults: 20, // 减少结果数量，避免输出过长
        includeContext: true,
        contextLines: 2,
      })

      const searchTime = Date.now() - startTime
      searchResult.searchTime = searchTime
      const resultText = this.formatSearchResults(searchResult)

      console.log('🔍 orbit_search:', query, '→', searchResult.totalMatches, 'matches')
      console.log('📄 LLM输出内容:')
      console.log(resultText)

      return {
        content: [
          {
            type: 'text',
            text: resultText,
          },
        ],
      }
    } catch (error) {
      if (error instanceof ValidationError || error instanceof FileNotFoundError) {
        throw error
      }
      throw new ToolError(`Semantic search failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 解析搜索路径
   */
  private async resolveSearchPath(targetPath: string): Promise<string> {
    const exists = await this.checkPathExists(targetPath)
    if (!exists) {
      throw new FileNotFoundError(targetPath)
    }
    return targetPath
  }

  /**
   * 检查路径是否存在
   */
  private async checkPathExists(path: string): Promise<boolean> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      return exists
    } catch (error) {
      return false
    }
  }

  /**
   * 解析查询参数，支持 "keyword1 OR keyword2 OR keyword3" 格式
   */
  private parseQuery(query: string): string[] {
    if (!query || query.trim() === '') {
      return []
    }

    // 检查是否包含 OR 操作符
    if (query.includes(' OR ')) {
      // 分割并清理关键词
      return query
        .split(' OR ')
        .map(keyword => keyword.trim())
        .filter(keyword => keyword !== '')
    }

    // 单个关键词
    return [query.trim()]
  }

  /**
   * 执行语义搜索 - 核心搜索引擎
   */
  private async performorbitSearch(options: {
    query: string[]
    searchPath: string
    fileExtensions?: string[]
    maxResults: number
    includeContext: boolean
    contextLines: number
  }): Promise<orbitSearchResponse> {
    const { query, searchPath, fileExtensions, maxResults, includeContext, contextLines } = options

    const response: orbitSearchResponse = {
      query,
      targetPath: searchPath,
      totalMatches: 0,
      filesSearched: 0,
      searchTime: 0,
      textMatches: [],
      symbolMatches: [],
      summary: '',
    }

    try {
      let codeAnalysis: AnalysisResult | null = null
      try {
        codeAnalysis = await this.performCodeAnalysis(searchPath, fileExtensions)
      } catch (error) {
        // AST分析失败时继续执行文本搜索
      }

      if (codeAnalysis) {
        const symbolMatches = await this.searchSymbols(query, codeAnalysis, maxResults)
        response.symbolMatches = symbolMatches
      }

      const textMatches = await this.searchText({
        query,
        searchPath,
        fileExtensions,
        maxResults: maxResults - response.symbolMatches.length,
        contextLines: includeContext ? contextLines : 0,
      })
      response.textMatches = textMatches

      response.totalMatches = response.textMatches.length + response.symbolMatches.length
      const allFiles = new Set([
        ...response.textMatches.map(m => m.filePath),
        ...response.symbolMatches.map(m => m.filePath),
      ])
      response.filesSearched = allFiles.size
      response.summary = `找到 ${response.totalMatches} 个匹配项`

      return response
    } catch (error) {
      throw new ToolError(`Search execution failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 执行代码结构分析
   */
  private async performCodeAnalysis(searchPath: string, fileExtensions?: string[]): Promise<AnalysisResult> {
    const params: AnalyzeCodeParams = {
      path: searchPath,
      recursive: true,
      include: fileExtensions?.map(ext => `**/*${ext}`) || [],
      exclude: [
        '**/node_modules/**',
        '**/dist/**',
        '**/build/**',
        '**/.git/**',
        '**/coverage/**',
        '**/*.min.js',
        '**/*.min.css',
      ],
    }

    return await aiApi.analyzeCode(params)
  }

  /**
   * 搜索符号定义和引用
   */
  private async searchSymbols(
    keywords: string[],
    analysis: AnalysisResult,
    maxResults: number
  ): Promise<SymbolMatch[]> {
    const matches: SymbolMatch[] = []

    for (const fileAnalysis of analysis.analyses) {
      for (const symbol of fileAnalysis.symbols) {
        const symbolName = symbol.name.toLowerCase()

        // 计算相关性评分 - 检查所有关键词
        let bestScore = 0
        let matchedKeywords = 0

        for (const keyword of keywords) {
          const keywordLower = keyword.toLowerCase()
          let keywordScore = 0

          // 精确匹配
          if (symbolName === keywordLower) {
            keywordScore = 100
          }
          // 开头匹配
          else if (symbolName.startsWith(keywordLower)) {
            keywordScore = 80
          }
          // 包含匹配
          else if (symbolName.includes(keywordLower)) {
            keywordScore = 60
          }
          // 驼峰匹配
          else if (this.matchesCamelCase(symbol.name, keyword)) {
            keywordScore = 70
          }
          // 模糊匹配
          else if (this.fuzzyMatch(symbolName, keywordLower)) {
            keywordScore = 40
          }

          if (keywordScore > 0) {
            matchedKeywords++
            bestScore = Math.max(bestScore, keywordScore)
          }
        }

        if (bestScore > 0) {
          // 根据符号类型调整评分
          let finalScore = this.adjustScoreBySymbolType(bestScore, symbol.type)

          // 匹配多个关键词的额外加分
          if (matchedKeywords > 1) {
            finalScore += (matchedKeywords - 1) * 10
          }

          matches.push({
            symbol,
            filePath: fileAnalysis.file,
            relevanceScore: Math.min(finalScore, 100),
            context: await this.getSymbolContext(fileAnalysis.file, symbol),
          })
        }
      }
    }

    // 按相关性评分排序并限制结果数量
    return matches.sort((a, b) => b.relevanceScore - a.relevanceScore).slice(0, maxResults)
  }

  /**
   * 驼峰命名法匹配
   */
  private matchesCamelCase(symbolName: string, query: string): boolean {
    const upperCaseLetters = symbolName.match(/[A-Z]/g) || []
    const camelCaseAbbrev = upperCaseLetters.join('').toLowerCase()
    return camelCaseAbbrev.includes(query.toLowerCase())
  }

  /**
   * 模糊匹配
   */
  private fuzzyMatch(text: string, pattern: string): boolean {
    let patternIndex = 0
    for (let i = 0; i < text.length && patternIndex < pattern.length; i++) {
      if (text[i] === pattern[patternIndex]) {
        patternIndex++
      }
    }
    return patternIndex === pattern.length
  }

  /**
   * 根据符号类型调整评分
   */
  private adjustScoreBySymbolType(score: number, symbolType: string): number {
    const typeWeights: Record<string, number> = {
      function: 1.2,
      class: 1.3,
      interface: 1.1,
      type: 1.1,
      method: 1.0,
      property: 0.9,
      variable: 0.8,
      constant: 0.9,
      enum: 1.1,
    }

    const weight = typeWeights[symbolType] || 1.0
    return Math.round(score * weight)
  }

  /**
   * 获取符号上下文
   */

  private async getSymbolContext(filePath: string, symbol: CodeSymbol): Promise<string[]> {
    try {
      // 直接读取文件内容
      const content = await this.readFileContent(filePath)
      const lines = content.split('\n')
      const symbolLine = symbol.line - 1 // 转换为0基索引

      // 边界检查
      if (symbolLine < 0 || symbolLine >= lines.length) {
        return []
      }

      const contextBefore = lines.slice(Math.max(0, symbolLine - 2), symbolLine)
      const contextAfter = lines.slice(symbolLine + 1, Math.min(lines.length, symbolLine + 3))

      return [...contextBefore, lines[symbolLine], ...contextAfter].filter(line => line.trim())
    } catch (error) {
      return []
    }
  }

  /**
   * 读取文件内容
   */
  private async readFileContent(filePath: string): Promise<string> {
    try {
      const rawContent = await invoke<ArrayBuffer>('plugin:fs|read_text_file', {
        path: filePath,
      })
      const content = new TextDecoder('utf-8').decode(rawContent)
      return content
    } catch (error) {
      throw new ToolError(`无法读取文件 ${filePath}: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 执行文本搜索
   */
  private async searchText(options: {
    query: string[]
    searchPath: string
    fileExtensions?: string[]
    maxResults: number
    contextLines: number
  }): Promise<SearchMatch[]> {
    const { query, searchPath, fileExtensions, maxResults, contextLines } = options
    const matches: SearchMatch[] = []

    try {
      const files = await this.getSearchableFiles(searchPath, fileExtensions)
      const searchFunction = this.createSearchFunction(query)

      for (const filePath of files) {
        if (matches.length >= maxResults) {
          break
        }

        try {
          const fileMatches = await this.searchInFile(filePath, searchFunction, contextLines, query)
          matches.push(...fileMatches.slice(0, maxResults - matches.length))
        } catch (error) {
          // 忽略单个文件的错误，继续搜索其他文件
        }
      }

      return matches.sort((a, b) => b.relevanceScore - a.relevanceScore)
    } catch (error) {
      throw new ToolError(`Text search failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 获取可搜索的文件列表
   */
  private async getSearchableFiles(searchPath: string, fileExtensions?: string[]): Promise<string[]> {
    const files: string[] = []
    const defaultIgnorePatterns = [
      'node_modules',
      '.git',
      '.svn',
      '.hg',
      'target',
      'build',
      'dist',
      '*.min.js',
      '*.min.css',
      '.DS_Store',
      'Thumbs.db',
    ]

    try {
      await this.walkDirectory(searchPath, files, fileExtensions, defaultIgnorePatterns, false)
      return files
    } catch (error) {
      throw new Error(`Failed to get file list: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 递归遍历目录
   */
  private async walkDirectory(
    dirPath: string,
    files: string[],
    allowedExtensions?: string[],
    ignorePatterns: string[] = [],
    includeHidden: boolean = false
  ): Promise<void> {
    try {
      const entries = await invoke<{ name: string; isDirectory: boolean; isFile: boolean; isSymlink: boolean }[]>(
        'plugin:fs|read_dir',
        {
          path: dirPath,
        }
      )

      for (const entry of entries) {
        const { name, isDirectory } = entry

        // 跳过隐藏文件/目录
        if (!includeHidden && name.startsWith('.')) {
          continue
        }

        // 构建完整路径
        const fullPath = `${dirPath}/${name}`.replace(/\/+/g, '/')

        // 检查忽略模式
        if (this.shouldIgnore(fullPath, name, ignorePatterns)) {
          continue
        }

        if (isDirectory) {
          // 递归搜索子目录
          await this.walkDirectory(fullPath, files, allowedExtensions, ignorePatterns, includeHidden)
        } else {
          // 检查文件扩展名
          if (this.shouldIncludeFile(name, allowedExtensions)) {
            files.push(fullPath)
          }
        }
      }
    } catch (error) {
      // 如果无法读取目录，跳过但不抛出错误
    }
  }

  /**
   * 检查是否应该忽略文件/目录
   */
  private shouldIgnore(path: string, name: string, ignorePatterns: string[]): boolean {
    for (const pattern of ignorePatterns) {
      if (pattern.includes('*')) {
        const regex = new RegExp(pattern.replace(/\*/g, '.*'))
        if (regex.test(name) || regex.test(path)) {
          return true
        }
      } else {
        if (name.includes(pattern) || path.includes(pattern)) {
          return true
        }
      }
    }
    return false
  }

  /**
   * 检查是否应该包含文件
   */
  private shouldIncludeFile(fileName: string, allowedExtensions?: string[]): boolean {
    if (!allowedExtensions || allowedExtensions.length === 0) {
      // 默认文本文件扩展名
      const textExtensions = [
        '.ts',
        '.js',
        '.tsx',
        '.jsx',
        '.vue',
        '.py',
        '.java',
        '.cpp',
        '.c',
        '.h',
        '.css',
        '.scss',
        '.sass',
        '.less',
        '.html',
        '.xml',
        '.json',
        '.yaml',
        '.yml',
        '.md',
        '.txt',
        '.sh',
        '.bash',
        '.zsh',
        '.fish',
        '.ps1',
        '.bat',
        '.cmd',
        '.sql',
        '.go',
        '.rs',
        '.rb',
        '.php',
        '.swift',
        '.kt',
        '.dart',
        '.cs',
        '.toml',
        '.ini',
        '.conf',
        '.config',
        '.env',
        '.gitignore',
        '.dockerignore',
      ]
      return textExtensions.some(ext => fileName.toLowerCase().endsWith(ext))
    }

    return allowedExtensions.some(ext => fileName.toLowerCase().endsWith(ext.toLowerCase()))
  }

  private createSearchFunction(keywords: string[]): (line: string, lineNumber: number) => boolean {
    const keywordsLower = keywords.map(k => k.toLowerCase())
    return (line: string) => {
      const lineLower = line.toLowerCase()
      // OR 逻辑：任一关键词匹配即返回 true
      return keywordsLower.some(keyword => lineLower.includes(keyword))
    }
  }

  /**
   * 在文件中搜索
   */
  private async searchInFile(
    filePath: string,
    searchFunction: (line: string, lineNumber: number) => boolean,
    contextLines: number,
    keywords: string[]
  ): Promise<SearchMatch[]> {
    try {
      const content = await this.readFileContent(filePath)
      const lines = content.split('\n')
      const matches: SearchMatch[] = []
      const maxMatchesPerFile = 20

      for (let i = 0; i < lines.length && matches.length < maxMatchesPerFile; i++) {
        const line = lines[i]

        if (!line.trim() || line.length > 1000) {
          continue
        }

        if (searchFunction(line, i + 1)) {
          const relevanceScore = this.calculateTextRelevanceScore(line, keywords, filePath)
          const contextBefore = lines.slice(Math.max(0, i - contextLines), i)
          const contextAfter = lines.slice(i + 1, Math.min(lines.length, i + 1 + contextLines))

          matches.push({
            filePath,
            lineNumber: i + 1,
            line: line,
            relevanceScore,
            context: {
              before: contextBefore,
              after: contextAfter,
            },
            highlightedLine: this.highlightMatch(line, keywords),
          })
        }
      }

      return matches
    } catch (error) {
      return []
    }
  }

  /**
   * 计算文本相关性评分
   */
  private calculateTextRelevanceScore(line: string, keywords: string[], filePath: string): number {
    const lineLower = line.toLowerCase()
    let totalScore = 0
    let matchedKeywords = 0

    // BM25-inspired scoring for each keyword
    for (const keyword of keywords) {
      const keywordLower = keyword.toLowerCase()
      const escapedKeyword = keywordLower.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const wordRegex = new RegExp(`\\b${escapedKeyword}\\b`, 'i')

      let keywordScore = 0

      // Term frequency (tf) - count occurrences
      const tf = (lineLower.match(new RegExp(keywordLower, 'g')) || []).length
      if (tf === 0) continue

      matchedKeywords++

      // BM25-like term frequency normalization
      // tf * (k1 + 1) / (tf + k1)
      const k1 = 1.5
      const normalizedTf = (tf * (k1 + 1)) / (tf + k1)

      // Word boundary bonus (like IDF boost)
      if (wordRegex.test(line)) {
        keywordScore = normalizedTf * 60 // Complete word match
      } else {
        keywordScore = normalizedTf * 20 // Partial match
      }

      // Code structure bonus
      if (
        line.includes('function') ||
        line.includes('class') ||
        line.includes('interface') ||
        line.includes('export') ||
        line.includes('import')
      ) {
        keywordScore *= 1.3
      }

      totalScore += keywordScore
    }

    // Multi-keyword bonus (like query coordination)
    if (matchedKeywords > 1) {
      totalScore *= 1 + (matchedKeywords - 1) * 0.2
    }

    // File type boost
    const fileExt = filePath.split('.').pop()?.toLowerCase()
    const importantExtensions = ['ts', 'js', 'tsx', 'jsx', 'py', 'java', 'cpp', 'c', 'h']
    if (fileExt && importantExtensions.includes(fileExt)) {
      totalScore *= 1.1
    }

    return Math.min(Math.round(totalScore), 100)
  }

  /**
   * 高亮匹配内容
   */
  private highlightMatch(line: string, keywords: string[]): string {
    let highlightedLine = line

    // 对每个关键词进行高亮
    for (const keyword of keywords) {
      const escapedKeyword = keyword.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const regex = new RegExp(`(${escapedKeyword})`, 'gi')
      highlightedLine = highlightedLine.replace(regex, '**$1**')
    }

    return highlightedLine
  }

  /**
   * 格式化搜索结果
   */
  private formatSearchResults(response: orbitSearchResponse): string {
    if (response.totalMatches === 0) {
      return `未找到关键词: ${response.query.join(', ')}`
    }

    let result = `搜索关键词: ${response.query.join(' OR ')}\n\n`

    // 显示符号匹配
    if (response.symbolMatches.length > 0) {
      result += `符号匹配 (${response.symbolMatches.length}):\n`
      for (const match of response.symbolMatches.slice(0, 5)) {
        result += `${match.filePath}:${match.symbol.line}\n`
        result += `${match.symbol.name} (${match.symbol.type}) - 评分: ${match.relevanceScore}\n\n`
      }
    }

    // 显示文本匹配
    if (response.textMatches.length > 0) {
      result += `文本匹配 (${response.textMatches.length}):\n`
      for (const match of response.textMatches.slice(0, 5)) {
        result += `${match.filePath}:${match.lineNumber}\n`
        result += `${match.line.trim()} - 评分: ${match.relevanceScore}\n\n`
      }
    }

    return result.trim()
  }
}

// 导出工具实例
export const orbitSearchTool = new OrbitSearchTool()
