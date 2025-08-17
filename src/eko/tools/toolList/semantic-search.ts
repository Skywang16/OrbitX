/**
 * 语义搜索工具 - 整合文本搜索、AST分析和语义理解的强大搜索引擎
 *
 * 融合了 orbit_context 的动态搜索能力和 analyze-code 的结构分析能力
 * 提供类似 AugmentCode ACE 引擎的智能代码搜索体验
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { FileNotFoundError, ValidationError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'
import { aiApi, type AnalyzeCodeParams, type AnalysisResult, type CodeSymbol } from '@/api'

// ===== 类型定义 =====

export interface SemanticSearchParams {
  /** 搜索查询内容 */
  query: string
  /** 搜索路径（可选，默认当前工作目录） */
  path?: string
}

export interface SearchMatch {
  filePath: string
  lineNumber: number
  line: string
  matchType: 'text' | 'symbol' | 'semantic' | 'structure'
  relevanceScore: number
  context: {
    before: string[]
    after: string[]
  }
  highlightedLine: string
  symbolInfo?: {
    name: string
    type: string
    definition: boolean
    usage: boolean
  }
}

export interface SymbolMatch {
  symbol: CodeSymbol
  filePath: string
  matchType: 'definition' | 'usage' | 'reference'
  relevanceScore: number
  context: string[]
}

export interface SemanticSearchResponse {
  query: string
  searchMode: string
  targetPath: string
  totalMatches: number
  filesSearched: number
  searchTime: number
  textMatches: SearchMatch[]
  symbolMatches: SymbolMatch[]
  summary: string
  suggestions: string[]
  codeStructure?: {
    totalFiles: number
    totalSymbols: number
    languages: string[]
    symbolTypes: Record<string, number>
  }
}

/**
 * 语义搜索工具 - ACE引擎风格的智能代码搜索
 *
 * 核心特性：
 * 1. 多模式搜索：文本、语义、代码结构、符号定义
 * 2. 智能相关性评分：结合文本匹配度、符号重要性、上下文相关性
 * 3. 结构化结果：按类型、重要性、文件组织搜索结果
 * 4. 上下文理解：提供丰富的代码上下文和建议
 */
export class SemanticSearchTool extends ModifiableTool {
  constructor() {
    super(
      'semantic_search',
      `智能语义搜索工具。
输入示例: {"query": "createUser", "path": "./src"}
输出示例: {
  "content": [{
    "type": "text",
    "text": "语义搜索结果 (3个匹配项，2个文件)\\n查询: \\"createUser\\" | 模式: hybrid | 耗时: 85ms\\n\\n符号匹配 (1个):\\n- createUser (function) - user.ts:15 [评分:100]\\n  export async function createUser(userData: UserData) {\\n\\n文本匹配 (2个):\\n- auth.ts:45 [评分:75]\\n  // TODO: 调用 **createUser** 创建新用户\\n- service.ts:23 [评分:60]\\n  const result = await **createUser**(data)"
  }]
}`,
      {
        type: 'object',
        properties: {
          query: {
            type: 'string',
            description: '搜索查询内容。示例："createUser"、"UserService"、"TODO"、"error"',
          },
          path: {
            type: 'string',
            description: '搜索路径。示例："./src"、"./components"。不填默认当前目录',
          },
        },
        required: ['query'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const params = context.parameters as unknown as SemanticSearchParams
    const { query, path } = params

    if (!query || query.trim() === '') {
      throw new ValidationError('搜索查询不能为空')
    }

    try {
      const startTime = Date.now()

      // 确定搜索路径
      const searchPath = await this.resolveSearchPath(path)

      // 执行语义搜索（使用默认配置）
      const searchResult = await this.performSemanticSearch({
        query,
        searchPath,
        mode: 'hybrid',
        fileExtensions: undefined,
        maxResults: 30,
        includeContext: true,
        contextLines: 2,
      })

      const searchTime = Date.now() - startTime
      searchResult.searchTime = searchTime

      // 格式化输出结果
      const resultText = this.formatSearchResults(searchResult)

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
      throw new Error(`语义搜索失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 解析搜索路径
   */
  private async resolveSearchPath(targetPath?: string): Promise<string> {
    if (targetPath) {
      const exists = await this.checkPathExists(targetPath)
      if (!exists) {
        throw new FileNotFoundError(targetPath)
      }
      return targetPath
    }

    try {
      return (await invoke<string>('get_current_working_directory')) || process.cwd()
    } catch (error) {
      return process.cwd()
    }
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
   * 执行语义搜索 - 核心搜索引擎
   */
  private async performSemanticSearch(options: {
    query: string
    searchPath: string
    mode: string
    fileExtensions?: string[]
    maxResults: number
    includeContext: boolean
    contextLines: number
  }): Promise<SemanticSearchResponse> {
    const { query, searchPath, mode, fileExtensions, maxResults, includeContext, contextLines } = options

    // 初始化搜索结果
    const response: SemanticSearchResponse = {
      query,
      searchMode: mode,
      targetPath: searchPath,
      totalMatches: 0,
      filesSearched: 0,
      searchTime: 0,
      textMatches: [],
      symbolMatches: [],
      summary: '',
      suggestions: [],
    }

    try {
      // 1. 获取代码结构分析（AST分析）
      let codeAnalysis: AnalysisResult | null = null
      if (mode === 'symbol' || mode === 'code' || mode === 'hybrid') {
        try {
          codeAnalysis = await this.performCodeAnalysis(searchPath, fileExtensions)
          response.codeStructure = this.extractCodeStructureInfo(codeAnalysis)
        } catch (error) {
          // AST分析失败时继续进行文本搜索
        }
      }

      // 2. 执行符号搜索
      if (codeAnalysis && (mode === 'symbol' || mode === 'hybrid')) {
        const symbolMatches = await this.searchSymbols(query, codeAnalysis, maxResults)
        response.symbolMatches = symbolMatches
      }

      // 3. 执行文本搜索
      if (mode === 'text' || mode === 'semantic' || mode === 'hybrid') {
        const textMatches = await this.searchText({
          query,
          searchPath,
          mode,
          fileExtensions,
          maxResults: maxResults - response.symbolMatches.length,
          contextLines: includeContext ? contextLines : 0,
        })
        response.textMatches = textMatches
      }

      // 4. 计算总匹配数和文件数
      response.totalMatches = response.textMatches.length + response.symbolMatches.length
      const allFiles = new Set([
        ...response.textMatches.map(m => m.filePath),
        ...response.symbolMatches.map(m => m.filePath),
      ])
      response.filesSearched = allFiles.size

      // 5. 生成搜索摘要和建议
      response.summary = this.generateSearchSummary(response)
      response.suggestions = this.generateSearchSuggestions(response, query)

      return response
    } catch (error) {
      throw new Error(`搜索执行失败: ${error instanceof Error ? error.message : String(error)}`)
    } finally {
      // 搜索完成后清理缓存，释放内存
      this.clearCache()
    }
  }

  /**
   * 清理文件内容缓存
   */
  private clearCache(): void {
    this.fileContentCache.clear()
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
   * 提取代码结构信息
   */
  private extractCodeStructureInfo(analysis: AnalysisResult): {
    totalFiles: number
    totalSymbols: number
    languages: string[]
    symbolTypes: Record<string, number>
  } {
    const languages = new Set<string>()
    const symbolTypes: Record<string, number> = {}
    let totalSymbols = 0

    for (const fileAnalysis of analysis.analyses) {
      languages.add(fileAnalysis.language)

      for (const symbol of fileAnalysis.symbols) {
        totalSymbols++
        const type = symbol.type
        symbolTypes[type] = (symbolTypes[type] || 0) + 1
      }
    }

    return {
      totalFiles: analysis.analyses.length,
      totalSymbols,
      languages: Array.from(languages),
      symbolTypes,
    }
  }

  /**
   * 搜索符号定义和引用
   */
  private async searchSymbols(query: string, analysis: AnalysisResult, maxResults: number): Promise<SymbolMatch[]> {
    const matches: SymbolMatch[] = []
    const queryLower = query.toLowerCase()

    for (const fileAnalysis of analysis.analyses) {
      for (const symbol of fileAnalysis.symbols) {
        const symbolName = symbol.name.toLowerCase()

        // 计算相关性评分
        let relevanceScore = 0

        // 精确匹配
        if (symbolName === queryLower) {
          relevanceScore = 100
        }
        // 开头匹配
        else if (symbolName.startsWith(queryLower)) {
          relevanceScore = 80
        }
        // 包含匹配
        else if (symbolName.includes(queryLower)) {
          relevanceScore = 60
        }
        // 驼峰匹配
        else if (this.matchesCamelCase(symbol.name, query)) {
          relevanceScore = 70
        }
        // 模糊匹配
        else if (this.fuzzyMatch(symbolName, queryLower)) {
          relevanceScore = 40
        }

        if (relevanceScore > 0) {
          // 根据符号类型调整评分
          relevanceScore = this.adjustScoreBySymbolType(relevanceScore, symbol.type)

          matches.push({
            symbol,
            filePath: fileAnalysis.file,
            matchType: 'definition',
            relevanceScore,
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
   * 获取符号上下文（优化版本，使用缓存）
   */
  private fileContentCache = new Map<string, string>()

  private async getSymbolContext(filePath: string, symbol: CodeSymbol): Promise<string[]> {
    try {
      // 使用缓存避免重复读取同一文件
      let content = this.fileContentCache.get(filePath)
      if (!content) {
        content = await this.readFileContent(filePath)
        this.fileContentCache.set(filePath, content)

        // 限制缓存大小，避免内存泄漏
        if (this.fileContentCache.size > 100) {
          const firstKey = this.fileContentCache.keys().next().value
          if (firstKey) {
            this.fileContentCache.delete(firstKey)
          }
        }
      }

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
   * 读取文件内容（优化版本，增加文件大小限制和错误处理）
   */
  private async readFileContent(filePath: string): Promise<string> {
    try {
      // 检查文件大小，避免读取过大的文件
      const stats = await invoke<{ size: number }>('plugin:fs|metadata', { path: filePath })
      const maxFileSize = 10 * 1024 * 1024 // 10MB 限制

      if (stats.size > maxFileSize) {
        throw new Error(`文件过大 (${Math.round(stats.size / 1024 / 1024)}MB)，跳过读取`)
      }

      const rawContent = await invoke<ArrayBuffer>('plugin:fs|read_text_file', {
        path: filePath,
      })
      return new TextDecoder('utf-8').decode(rawContent)
    } catch (error) {
      if (error instanceof Error && error.message.includes('文件过大')) {
        throw error
      }
      throw new Error(`无法读取文件 ${filePath}: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 执行文本搜索
   */
  private async searchText(options: {
    query: string
    searchPath: string
    mode: string
    fileExtensions?: string[]
    maxResults: number
    contextLines: number
  }): Promise<SearchMatch[]> {
    const { query, searchPath, mode, fileExtensions, maxResults, contextLines } = options
    const matches: SearchMatch[] = []

    try {
      // 获取要搜索的文件列表
      const files = await this.getSearchableFiles(searchPath, fileExtensions)

      // 根据搜索模式创建搜索函数
      const searchFunction = this.createSearchFunction(mode, query)

      // 搜索每个文件
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

      // 按相关性评分排序
      return matches.sort((a, b) => b.relevanceScore - a.relevanceScore)
    } catch (error) {
      throw new Error(`文本搜索失败: ${error instanceof Error ? error.message : String(error)}`)
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
      throw new Error(`获取文件列表失败: ${error instanceof Error ? error.message : String(error)}`)
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
      const entries = await invoke<{ name: string; path: string; isDir: boolean }[]>('plugin:fs|read_dir', {
        dir: dirPath,
      })

      for (const entry of entries) {
        const { name, path, isDir } = entry

        // 跳过隐藏文件/目录
        if (!includeHidden && name.startsWith('.')) {
          continue
        }

        // 检查忽略模式
        if (this.shouldIgnore(path, name, ignorePatterns)) {
          continue
        }

        if (isDir) {
          // 递归搜索子目录
          await this.walkDirectory(path, files, allowedExtensions, ignorePatterns, includeHidden)
        } else {
          // 检查文件扩展名
          if (this.shouldIncludeFile(name, allowedExtensions)) {
            files.push(path)
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

  /**
   * 创建搜索函数
   */
  private createSearchFunction(mode: string, query: string): (line: string, lineNumber: number) => boolean {
    const queryLower = query.toLowerCase()

    switch (mode) {
      case 'semantic':
        return this.createSemanticSearchFunction(query)

      case 'code':
        return this.createCodeSearchFunction(query)

      case 'text':
      case 'hybrid':
      default:
        return (line: string) => {
          const lineLower = line.toLowerCase()
          return lineLower.includes(queryLower)
        }
    }
  }

  /**
   * 创建语义搜索函数
   */
  private createSemanticSearchFunction(query: string): (line: string) => boolean {
    const semanticPatterns = this.generateSemanticPatterns(query)

    return (line: string) => {
      return semanticPatterns.some(pattern => {
        const regex = new RegExp(pattern, 'gi')
        return regex.test(line)
      })
    }
  }

  /**
   * 创建代码搜索函数
   */
  private createCodeSearchFunction(query: string): (line: string) => boolean {
    const codePatterns = this.generateCodePatterns(query)

    return (line: string) => {
      return codePatterns.some(pattern => {
        const regex = new RegExp(pattern, 'gi')
        return regex.test(line)
      })
    }
  }

  /**
   * 生成语义搜索模式
   */
  private generateSemanticPatterns(query: string): string[] {
    const patterns: string[] = []
    const escapedQuery = this.escapeRegex(query)

    // 基本匹配
    patterns.push(escapedQuery)

    // 常见变形
    patterns.push(`${escapedQuery}[s]?`) // 复数形式
    patterns.push(`${escapedQuery}[ed]?`) // 过去式
    patterns.push(`${escapedQuery}[ing]?`) // 进行时

    // 驼峰命名法变形
    patterns.push(`[a-z]*${escapedQuery}[A-Z]?[a-z]*`)
    patterns.push(`[A-Z]*${escapedQuery}[a-z]*`)

    // 下划线连接
    patterns.push(`[a-z_]*${escapedQuery}[a-z_]*`)

    return patterns
  }

  /**
   * 生成代码搜索模式
   */
  private generateCodePatterns(query: string): string[] {
    const patterns: string[] = []
    const escapedQuery = this.escapeRegex(query)

    // 函数定义
    patterns.push(`function\\s+${escapedQuery}\\s*\\(`)
    patterns.push(`const\\s+${escapedQuery}\\s*=`)
    patterns.push(`let\\s+${escapedQuery}\\s*=`)
    patterns.push(`var\\s+${escapedQuery}\\s*=`)
    patterns.push(`${escapedQuery}\\s*:\\s*function`)
    patterns.push(`${escapedQuery}\\s*\\(.*\\)\\s*{`) // 箭头函数或方法

    // 类定义
    patterns.push(`class\\s+${escapedQuery}\\b`)
    patterns.push(`interface\\s+${escapedQuery}\\b`)
    patterns.push(`type\\s+${escapedQuery}\\s*=`)
    patterns.push(`enum\\s+${escapedQuery}\\b`)

    // 属性和方法
    patterns.push(`\\.${escapedQuery}\\b`)
    patterns.push(`${escapedQuery}\\s*:`)
    patterns.push(`${escapedQuery}\\s*\\(`)

    // 导入导出
    patterns.push(`import.*${escapedQuery}`)
    patterns.push(`export.*${escapedQuery}`)
    patterns.push(`from.*${escapedQuery}`)

    return patterns
  }

  /**
   * 转义正则表达式特殊字符
   */
  private escapeRegex(text: string): string {
    return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  /**
   * 在文件中搜索（优化版本，增加性能优化和错误处理）
   */
  private async searchInFile(
    filePath: string,
    searchFunction: (line: string, lineNumber: number) => boolean,
    contextLines: number,
    query: string
  ): Promise<SearchMatch[]> {
    try {
      // 使用缓存的文件内容
      let content = this.fileContentCache.get(filePath)
      if (!content) {
        content = await this.readFileContent(filePath)
        this.fileContentCache.set(filePath, content)

        // 限制缓存大小
        if (this.fileContentCache.size > 100) {
          const firstKey = this.fileContentCache.keys().next().value
          if (firstKey) {
            this.fileContentCache.delete(firstKey)
          }
        }
      }

      const lines = content.split('\n')
      const matches: SearchMatch[] = []

      // 限制单个文件的最大匹配数，避免性能问题
      const maxMatchesPerFile = 20

      for (let i = 0; i < lines.length && matches.length < maxMatchesPerFile; i++) {
        const line = lines[i]

        // 跳过空行和过长的行
        if (!line.trim() || line.length > 1000) {
          continue
        }

        if (searchFunction(line, i + 1)) {
          // 计算相关性评分
          const relevanceScore = this.calculateTextRelevanceScore(line, query, filePath)

          // 获取上下文
          const contextBefore = lines.slice(Math.max(0, i - contextLines), i)
          const contextAfter = lines.slice(i + 1, Math.min(lines.length, i + 1 + contextLines))

          matches.push({
            filePath,
            lineNumber: i + 1,
            line: line,
            matchType: 'text',
            relevanceScore,
            context: {
              before: contextBefore,
              after: contextAfter,
            },
            highlightedLine: this.highlightMatch(line, query),
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
  private calculateTextRelevanceScore(line: string, query: string, filePath: string): number {
    let score = 50 // 基础分数

    const lineLower = line.toLowerCase()
    const queryLower = query.toLowerCase()

    // 精确匹配
    if (lineLower.includes(queryLower)) {
      score += 30
    }

    // 单词边界匹配
    const wordRegex = new RegExp(`\\b${this.escapeRegex(queryLower)}\\b`, 'i')
    if (wordRegex.test(line)) {
      score += 20
    }

    // 文件类型权重
    const fileExt = filePath.split('.').pop()?.toLowerCase()
    const importantExtensions = ['ts', 'js', 'tsx', 'jsx', 'py', 'java', 'cpp', 'c', 'h']
    if (fileExt && importantExtensions.includes(fileExt)) {
      score += 10
    }

    // 代码结构权重
    if (line.includes('function') || line.includes('class') || line.includes('interface')) {
      score += 15
    }

    return Math.min(score, 100)
  }

  /**
   * 高亮匹配内容
   */
  private highlightMatch(line: string, query: string): string {
    const regex = new RegExp(`(${this.escapeRegex(query)})`, 'gi')
    return line.replace(regex, '**$1**')
  }

  /**
   * 生成搜索摘要
   */
  private generateSearchSummary(response: SemanticSearchResponse): string {
    if (response.totalMatches === 0) {
      return `未找到包含 "${response.query}" 的结果`
    }

    const { totalMatches, filesSearched, textMatches, symbolMatches } = response
    const modeDescription =
      {
        text: '文本搜索',
        semantic: '语义搜索',
        code: '代码结构搜索',
        symbol: '符号搜索',
        hybrid: '混合搜索',
      }[response.searchMode] || '搜索'

    let summary = `${modeDescription}找到 ${totalMatches} 个匹配项，分布在 ${filesSearched} 个文件中`

    if (symbolMatches.length > 0) {
      summary += `\n符号匹配: ${symbolMatches.length} 个`
    }

    if (textMatches.length > 0) {
      summary += `\n文本匹配: ${textMatches.length} 个`
    }

    if (response.codeStructure) {
      const { totalFiles, totalSymbols, languages } = response.codeStructure
      summary += `\n代码结构: ${totalFiles} 个文件，${totalSymbols} 个符号，语言: ${languages.join(', ')}`
    }

    return summary
  }

  /**
   * 生成搜索建议
   */
  private generateSearchSuggestions(response: SemanticSearchResponse, query: string): string[] {
    const suggestions: string[] = []

    // 基于结果数量的建议
    if (response.totalMatches === 0) {
      suggestions.push('尝试使用更通用的搜索词')
      suggestions.push('检查拼写是否正确')
      suggestions.push('尝试使用不同的搜索模式')
    } else if (response.totalMatches > 50) {
      suggestions.push('结果较多，建议使用更具体的搜索词')
      suggestions.push('考虑添加文件类型过滤')
    }

    // 基于搜索模式的建议
    if (response.searchMode === 'text' && response.symbolMatches.length === 0) {
      suggestions.push('尝试使用 "symbol" 模式搜索函数和类定义')
    }

    if (response.searchMode === 'symbol' && response.textMatches.length === 0) {
      suggestions.push('尝试使用 "text" 模式搜索文本内容')
    }

    // 基于文件类型的建议
    if (response.codeStructure) {
      const { languages } = response.codeStructure
      if (languages.length > 1) {
        suggestions.push(`涉及多种编程语言: ${languages.join(', ')}，考虑按语言过滤`)
      }
    }

    // 基于查询内容的建议
    if (query.length < 3) {
      suggestions.push('搜索词较短，建议使用更长的关键词')
    }

    return suggestions
  }

  /**
   * 格式化搜索结果
   */
  private formatSearchResults(response: SemanticSearchResponse): string {
    if (response.totalMatches === 0) {
      return `未找到包含 "${response.query}" 的结果\n\n建议:\n${response.suggestions.map(s => `- ${s}`).join('\n')}`
    }

    let result = `语义搜索结果 (${response.totalMatches}个匹配项，${response.filesSearched}个文件)\n`
    result += `查询: "${response.query}" | 模式: ${response.searchMode} | 耗时: ${response.searchTime}ms\n\n`

    // 显示代码结构概览
    if (response.codeStructure) {
      const { totalFiles, totalSymbols, languages, symbolTypes } = response.codeStructure
      result += `代码结构概览:\n`
      result += `- 文件数: ${totalFiles}\n`
      result += `- 符号数: ${totalSymbols}\n`
      result += `- 语言: ${languages.join(', ')}\n`

      const topSymbolTypes = Object.entries(symbolTypes)
        .sort(([, a], [, b]) => b - a)
        .slice(0, 3)
        .map(([type, count]) => `${type}(${count})`)
        .join(', ')

      if (topSymbolTypes) {
        result += `- 主要符号类型: ${topSymbolTypes}\n`
      }
      result += '\n'
    }

    // 显示符号匹配
    if (response.symbolMatches.length > 0) {
      result += `符号匹配 (${response.symbolMatches.length}个):\n`

      const topSymbolMatches = response.symbolMatches.slice(0, 5)
      for (const match of topSymbolMatches) {
        const { symbol, filePath, relevanceScore } = match
        const fileName = filePath.split('/').pop() || filePath
        result += `- ${symbol.name} (${symbol.type}) - ${fileName}:${symbol.line} [评分:${relevanceScore}]\n`

        if (match.context.length > 0) {
          const contextPreview = match.context[0].trim()
          if (contextPreview) {
            result += `  ${contextPreview}\n`
          }
        }
      }

      if (response.symbolMatches.length > 5) {
        result += `  ... 还有 ${response.symbolMatches.length - 5} 个符号匹配\n`
      }
      result += '\n'
    }

    // 显示文本匹配
    if (response.textMatches.length > 0) {
      result += `文本匹配 (${response.textMatches.length}个):\n`

      const topTextMatches = response.textMatches.slice(0, 8)
      for (const match of topTextMatches) {
        const fileName = match.filePath.split('/').pop() || match.filePath
        result += `- ${fileName}:${match.lineNumber} [评分:${match.relevanceScore}]\n`
        result += `  ${match.highlightedLine.trim()}\n`

        if (match.context.before.length > 0 || match.context.after.length > 0) {
          const contextLines = [...match.context.before.slice(-1), ...match.context.after.slice(0, 1)].filter(line =>
            line.trim()
          )

          if (contextLines.length > 0) {
            result += `  上下文: ${contextLines.join(' | ')}\n`
          }
        }
      }

      if (response.textMatches.length > 8) {
        result += `  ... 还有 ${response.textMatches.length - 8} 个文本匹配\n`
      }
      result += '\n'
    }

    // 显示建议
    if (response.suggestions.length > 0) {
      result += `建议:\n`
      for (const suggestion of response.suggestions) {
        result += `- ${suggestion}\n`
      }
    }

    return result
  }
}

// 导出工具实例
export const semanticSearchTool = new SemanticSearchTool()
