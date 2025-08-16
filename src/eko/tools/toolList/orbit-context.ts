/**
 * Orbit Context 文本搜索工具
 *
 * 基于Cline的设计理念：不进行静态索引，而是通过动态搜索来探索代码库
 * 提供强大的代码搜索和上下文理解能力
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { FileNotFoundError, ValidationError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface OrbitContextParams {
  query: string
  targetPath?: string
  fileExtensions?: string[]
}

export interface SearchMatch {
  filePath: string
  lineNumber: number
  line: string
  context: {
    before: string[]
    after: string[]
  }
  highlightedLine: string
}

export interface OrbitContextResponse {
  query: string
  searchMode: string
  targetPath: string
  totalMatches: number
  filesSearched: number
  searchTime: number
  matches: SearchMatch[]
  summary: string
}

/**
 * Orbit Context 文本搜索工具
 *
 * 动态探索代码库，提供智能的文本搜索和上下文理解
 */
export class OrbitContextTool extends ModifiableTool {
  constructor() {
    super('orbit_context', '代码库智能搜索：全面扫描代码库，智能检索相关代码片段和上下文信息', {
      type: 'object',
      properties: {
        query: {
          type: 'string',
          description: '搜索查询内容',
        },
        targetPath: {
          type: 'string',
          description: '搜索路径（可选，默认当前工作目录）',
        },
        fileExtensions: {
          type: 'array',
          items: { type: 'string' },
          description: '文件类型过滤（可选）',
        },
      },
      required: ['query'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { query, targetPath, fileExtensions } = context.parameters as unknown as OrbitContextParams

    if (!query || query.trim() === '') {
      throw new ValidationError('搜索查询不能为空')
    }

    try {
      // 确定搜索路径
      const searchPath = await this.resolveSearchPath(targetPath)

      // 执行全面搜索 - 参考ACE引擎的智能检索理念
      const matches = await this.performSearch({
        query,
        searchMode: 'text',
        searchPath,
        fileExtensions,
        // 智能排除常见的依赖和构建目录
        ignorePatterns: ['node_modules', '.git', 'dist', 'build', '.next', 'coverage', '.nyc_output'],
        contextLines: 3,
        maxResults: 100, // 增加结果数量以获得更全面的搜索
        caseSensitive: false,
        wholeWords: false,
        includeHidden: false,
      })

      // 智能格式化输出 - 突出重要信息，减少干扰
      const resultText = this.formatSearchResults({
        query,
        searchMode: 'text',
        targetPath: searchPath,
        totalMatches: matches.length,
        filesSearched: new Set(matches.map((m: SearchMatch) => m.filePath)).size,
        searchTime: 0,
        matches,
        summary: this.generateSearchSummary(matches, 'text', query),
      })

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
      throw new Error(`搜索失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async resolveSearchPath(targetPath?: string): Promise<string> {
    if (targetPath) {
      // 验证路径存在
      const exists = await this.checkPathExists(targetPath)
      if (!exists) {
        throw new FileNotFoundError(targetPath)
      }
      return targetPath
    }

    // 使用当前工作目录
    try {
      return (await invoke<string>('get_current_working_directory')) || process.cwd()
    } catch (error) {
      // 降级到进程工作目录
      return process.cwd()
    }
  }

  private async performSearch(options: {
    query: string
    searchMode: string
    searchPath: string
    fileExtensions?: string[]
    ignorePatterns?: string[]
    contextLines: number
    maxResults: number
    caseSensitive: boolean
    wholeWords: boolean
    includeHidden: boolean
  }): Promise<SearchMatch[]> {
    const matches: SearchMatch[] = []

    try {
      // 获取所有要搜索的文件
      const files = await this.getSearchableFiles(
        options.searchPath,
        options.fileExtensions,
        options.ignorePatterns,
        options.includeHidden
      )

      // 根据搜索模式准备搜索逻辑
      const searchFunction = this.getSearchFunction(
        options.searchMode,
        options.query,
        options.caseSensitive,
        options.wholeWords
      )

      // 搜索每个文件
      for (const filePath of files) {
        if (matches.length >= options.maxResults) {
          break
        }

        try {
          const fileMatches = await this.searchInFile(filePath, searchFunction, options.contextLines)
          matches.push(...fileMatches.slice(0, options.maxResults - matches.length))
        } catch (error) {
          // 忽略单个文件的错误，继续搜索其他文件
          // 在生产环境中，可以考虑使用日志系统记录错误
        }
      }

      return matches
    } catch (error) {
      throw new Error(`执行搜索时出错: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async getSearchableFiles(
    searchPath: string,
    fileExtensions?: string[],
    ignorePatterns?: string[],
    includeHidden?: boolean
  ): Promise<string[]> {
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

    const allIgnorePatterns = [...defaultIgnorePatterns, ...(ignorePatterns || [])]

    try {
      // 递归获取所有文件
      await this.walkDirectory(searchPath, files, fileExtensions, allIgnorePatterns, includeHidden || false)
      return files
    } catch (error) {
      throw new Error(`获取文件列表失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

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

        // 跳过隐藏文件/目录（除非明确包含）
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
      // 在生产环境中，可以考虑使用日志系统记录错误
    }
  }

  private shouldIgnore(path: string, name: string, ignorePatterns: string[]): boolean {
    for (const pattern of ignorePatterns) {
      // 简单的模式匹配
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

  private shouldIncludeFile(fileName: string, allowedExtensions?: string[]): boolean {
    if (!allowedExtensions || allowedExtensions.length === 0) {
      // 默认只搜索文本文件
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

  private getSearchFunction(
    searchMode: string,
    query: string,
    caseSensitive: boolean,
    wholeWords: boolean
  ): (line: string, lineNumber: number) => boolean {
    switch (searchMode) {
      case 'regex': {
        const regexFlags = caseSensitive ? 'g' : 'gi'
        const regex = new RegExp(query, regexFlags)
        return (line: string) => regex.test(line)
      }

      case 'semantic':
        // 语义搜索：查找相关的概念和模式
        return this.createSemanticSearchFunction(query, caseSensitive)

      case 'code':
        // 代码结构搜索：查找函数、类、变量等
        return this.createCodeSearchFunction(query, caseSensitive)

      case 'text':
      default: {
        // 文本搜索
        const searchQuery = caseSensitive ? query : query.toLowerCase()
        if (wholeWords) {
          const wordRegex = new RegExp(`\\b${this.escapeRegex(searchQuery)}\\b`, caseSensitive ? 'g' : 'gi')
          return (line: string) => wordRegex.test(line)
        } else {
          return (line: string) => {
            const searchLine = caseSensitive ? line : line.toLowerCase()
            return searchLine.includes(searchQuery)
          }
        }
      }
    }
  }

  private createSemanticSearchFunction(query: string, caseSensitive: boolean): (line: string) => boolean {
    // 语义搜索：查找相关概念的模式
    const semanticPatterns = this.generateSemanticPatterns(query)
    const flags = caseSensitive ? 'g' : 'gi'

    return (line: string) => {
      return semanticPatterns.some(pattern => {
        const regex = new RegExp(pattern, flags)
        return regex.test(line)
      })
    }
  }

  private createCodeSearchFunction(query: string, caseSensitive: boolean): (line: string) => boolean {
    // 代码结构搜索：查找函数定义、类定义、变量声明等
    const codePatterns = this.generateCodePatterns(query)
    const flags = caseSensitive ? 'g' : 'gi'

    return (line: string) => {
      return codePatterns.some(pattern => {
        const regex = new RegExp(pattern, flags)
        return regex.test(line)
      })
    }
  }

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

  private escapeRegex(text: string): string {
    return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  private async searchInFile(
    filePath: string,
    searchFunction: (line: string, lineNumber: number) => boolean,
    contextLines: number
  ): Promise<SearchMatch[]> {
    try {
      // 读取文件内容
      const content = await this.readFileContent(filePath)
      const lines = content.split('\n')
      const matches: SearchMatch[] = []

      for (let i = 0; i < lines.length; i++) {
        const line = lines[i]
        if (searchFunction(line, i + 1)) {
          // 获取上下文
          const contextBefore = lines.slice(Math.max(0, i - contextLines), i)
          const contextAfter = lines.slice(i + 1, Math.min(lines.length, i + 1 + contextLines))

          matches.push({
            filePath,
            lineNumber: i + 1,
            line: line,
            context: {
              before: contextBefore,
              after: contextAfter,
            },
            highlightedLine: line, // 在实际实现中可以添加高亮
          })
        }
      }

      return matches
    } catch (error) {
      // 如果无法读取文件，返回空结果
      return []
    }
  }

  private async readFileContent(filePath: string): Promise<string> {
    try {
      const rawContent = await invoke<ArrayBuffer>('plugin:fs|read_text_file', {
        path: filePath,
      })
      return new TextDecoder('utf-8').decode(rawContent)
    } catch (error) {
      throw new Error(`无法读取文件 ${filePath}: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private generateSearchSummary(matches: SearchMatch[], searchMode: string, query: string): string {
    if (matches.length === 0) {
      return `未找到包含 "${query}" 的结果`
    }

    const fileCount = new Set(matches.map(m => m.filePath)).size
    const modeDescription =
      {
        text: '文本搜索',
        regex: '正则表达式搜索',
        semantic: '语义搜索',
        code: '代码结构搜索',
      }[searchMode] || '搜索'

    return `${modeDescription}找到 ${matches.length} 个匹配项，分布在 ${fileCount} 个文件中`
  }

  private formatSearchResults(response: OrbitContextResponse): string {
    if (response.matches.length === 0) {
      return `未找到包含 "${response.query}" 的结果`
    }

    let result = `搜索结果：找到 ${response.totalMatches} 个匹配项\n\n`

    // 按文件分组显示结果
    const groupedResults = new Map<string, SearchMatch[]>()
    for (const match of response.matches) {
      if (!groupedResults.has(match.filePath)) {
        groupedResults.set(match.filePath, [])
      }
      groupedResults.get(match.filePath)!.push(match)
    }

    for (const [filePath, fileMatches] of groupedResults) {
      result += `${filePath}:\n`

      for (const match of fileMatches.slice(0, 3)) {
        result += `  第${match.lineNumber}行: ${match.line.trim()}\n`

        // 显示简单上下文
        if (match.context.before.length > 0) {
          const beforeLine = match.context.before[match.context.before.length - 1]
          if (beforeLine.trim()) {
            result += `    前: ${beforeLine.trim()}\n`
          }
        }
        if (match.context.after.length > 0) {
          const afterLine = match.context.after[0]
          if (afterLine.trim()) {
            result += `    后: ${afterLine.trim()}\n`
          }
        }
      }

      if (fileMatches.length > 3) {
        result += `  ... 还有 ${fileMatches.length - 3} 个匹配项\n`
      }
      result += '\n'
    }

    return result
  }

  private async checkPathExists(path: string): Promise<boolean> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      return exists
    } catch (error) {
      return false
    }
  }
}

// 导出工具实例
export const orbitContextTool = new OrbitContextTool()
