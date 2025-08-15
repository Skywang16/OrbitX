/**
 * 符号查找工具 - 在代码库中查找函数、类、变量等符号
 */

import type { Tool, TerminalToolParams } from '../../types'
import { readFile, readDir } from '@tauri-apps/plugin-fs'
import { parse } from '@typescript-eslint/typescript-estree'
import * as acorn from 'acorn'
import { join, extname, basename } from 'path'

interface FindSymbolParams extends TerminalToolParams {
  /** 要查找的符号名称 */
  symbol: string
  /** 搜索路径 */
  path?: string
  /** 符号类型过滤 */
  type?: 'function' | 'class' | 'variable' | 'interface' | 'type' | 'all'
  /** 是否递归搜索 */
  recursive?: boolean
}

interface SymbolMatch {
  name: string
  type: string
  file: string
  line: number
  column: number
  context: string
}

export const findSymbolTool: Tool<FindSymbolParams> = {
  name: 'find-symbol',
  description:
    '🎯 精确符号查找：基于AST语法分析精确查找特定符号的定义位置（函数、类、变量、接口等）。适用于：查找符号定义、跳转到声明、理解符号用法。与orbit_context的区别：这个工具只查找语法符号定义，orbit_context可搜索任意文本内容',
  parameters: {
    type: 'object',
    properties: {
      symbol: {
        type: 'string',
        description: '要查找的符号名称',
      },
      path: {
        type: 'string',
        description: '搜索路径，默认为当前目录',
        default: '.',
      },
      type: {
        type: 'string',
        enum: ['function', 'class', 'variable', 'interface', 'type', 'all'],
        description: '符号类型过滤',
        default: 'all',
      },
      recursive: {
        type: 'boolean',
        description: '是否递归搜索子目录',
        default: true,
      },
    },
    required: ['symbol'],
  },

  async execute(params, context) {
    try {
      const { symbol, path = '.', type = 'all', recursive = true } = params

      const matches = await findSymbolInPath(symbol, path, type, recursive)

      return {
        content: [
          {
            type: 'text',
            text: formatSearchResults(symbol, matches),
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `符号查找失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 在指定路径中查找符号
 */
async function findSymbolInPath(
  symbolName: string,
  searchPath: string,
  symbolType: string,
  recursive: boolean
): Promise<SymbolMatch[]> {
  const matches: SymbolMatch[] = []

  try {
    // 检查是否为文件
    const isFile = extname(searchPath) !== ''

    if (isFile) {
      const fileMatches = await findSymbolInFile(symbolName, searchPath, symbolType)
      matches.push(...fileMatches)
    } else {
      const files = await getCodeFiles(searchPath, recursive)

      for (const file of files) {
        try {
          const fileMatches = await findSymbolInFile(symbolName, file, symbolType)
          matches.push(...fileMatches)
        } catch (error) {
          console.warn(`搜索文件 ${file} 失败:`, error)
        }
      }
    }
  } catch (error) {
    console.warn(`搜索路径 ${searchPath} 失败:`, error)
  }

  return matches
}

/**
 * 获取代码文件列表
 */
async function getCodeFiles(dirPath: string, recursive: boolean): Promise<string[]> {
  const files: string[] = []

  try {
    const entries = await readDir(dirPath)

    for (const entry of entries) {
      const fullPath = join(dirPath, entry.name)

      if (entry.isDirectory) {
        if (recursive && !shouldSkipDirectory(entry.name)) {
          const subFiles = await getCodeFiles(fullPath, recursive)
          files.push(...subFiles)
        }
      } else {
        if (isCodeFile(entry.name)) {
          files.push(fullPath)
        }
      }
    }
  } catch (error) {
    console.warn(`读取目录 ${dirPath} 失败:`, error)
  }

  return files
}

/**
 * 检查是否为代码文件
 */
function isCodeFile(fileName: string): boolean {
  const ext = extname(fileName)
  return ['.ts', '.tsx', '.js', '.jsx', '.py', '.rs', '.go', '.java', '.cpp', '.c', '.h'].includes(ext)
}

/**
 * 检查是否应该跳过目录
 */
function shouldSkipDirectory(dirName: string): boolean {
  const skipDirs = ['node_modules', '.git', 'dist', 'build', 'target', '__pycache__', '.vscode', '.idea']
  return skipDirs.includes(dirName) || dirName.startsWith('.')
}

/**
 * 在单个文件中查找符号
 */
async function findSymbolInFile(symbolName: string, filePath: string, symbolType: string): Promise<SymbolMatch[]> {
  const matches: SymbolMatch[] = []

  try {
    const content = await readFile(filePath)
    const text = new TextDecoder().decode(content)
    const ext = extname(filePath)

    if (['.ts', '.tsx'].includes(ext)) {
      const tsMatches = findSymbolInTypeScript(symbolName, text, filePath, symbolType)
      matches.push(...tsMatches)
    } else if (['.js', '.jsx'].includes(ext)) {
      const jsMatches = findSymbolInJavaScript(symbolName, text, filePath, symbolType)
      matches.push(...jsMatches)
    } else {
      // 对于其他文件类型，使用简单的文本搜索
      const textMatches = findSymbolInText(symbolName, text, filePath)
      matches.push(...textMatches)
    }
  } catch (error) {
    console.warn(`搜索文件 ${filePath} 失败:`, error)
  }

  return matches
}

/**
 * 在TypeScript代码中查找符号
 */
function findSymbolInTypeScript(
  symbolName: string,
  content: string,
  filePath: string,
  symbolType: string
): SymbolMatch[] {
  const matches: SymbolMatch[] = []

  try {
    const ast = parse(content, {
      loc: true,
      range: true,
      jsx: filePath.endsWith('.tsx'),
    })

    const lines = content.split('\n')

    function visit(node: any) {
      if (!node) return

      let nodeType = ''
      let nodeName = ''

      switch (node.type) {
        case 'FunctionDeclaration':
          nodeType = 'function'
          nodeName = node.id?.name
          break
        case 'ClassDeclaration':
          nodeType = 'class'
          nodeName = node.id?.name
          break
        case 'VariableDeclarator':
          nodeType = 'variable'
          nodeName = node.id?.name
          break
        case 'TSInterfaceDeclaration':
          nodeType = 'interface'
          nodeName = node.id?.name
          break
        case 'TSTypeAliasDeclaration':
          nodeType = 'type'
          nodeName = node.id?.name
          break
      }

      if (nodeName && nodeName.includes(symbolName) && (symbolType === 'all' || symbolType === nodeType)) {
        const line = node.loc?.start?.line || 1
        const column = node.loc?.start?.column || 0
        const context = getLineContext(lines, line - 1)

        matches.push({
          name: nodeName,
          type: nodeType,
          file: filePath,
          line,
          column,
          context,
        })
      }

      // 递归遍历子节点
      for (const key in node) {
        const child = node[key]
        if (Array.isArray(child)) {
          child.forEach(visit)
        } else if (child && typeof child === 'object') {
          visit(child)
        }
      }
    }

    visit(ast)
  } catch (error) {
    console.warn('TypeScript解析失败:', error)
  }

  return matches
}

/**
 * 在JavaScript代码中查找符号
 */
function findSymbolInJavaScript(
  symbolName: string,
  content: string,
  filePath: string,
  symbolType: string
): SymbolMatch[] {
  const matches: SymbolMatch[] = []

  try {
    const ast = acorn.parse(content, {
      ecmaVersion: 'latest',
      sourceType: 'module',
      locations: true,
    })

    const lines = content.split('\n')

    function visit(node: any) {
      if (!node) return

      let nodeType = ''
      let nodeName = ''

      switch (node.type) {
        case 'FunctionDeclaration':
          nodeType = 'function'
          nodeName = node.id?.name
          break
        case 'ClassDeclaration':
          nodeType = 'class'
          nodeName = node.id?.name
          break
        case 'VariableDeclarator':
          nodeType = 'variable'
          nodeName = node.id?.name
          break
      }

      if (nodeName && nodeName.includes(symbolName) && (symbolType === 'all' || symbolType === nodeType)) {
        const line = node.loc?.start?.line || 1
        const column = node.loc?.start?.column || 0
        const context = getLineContext(lines, line - 1)

        matches.push({
          name: nodeName,
          type: nodeType,
          file: filePath,
          line,
          column,
          context,
        })
      }

      // 递归遍历
      for (const key in node) {
        const child = node[key]
        if (Array.isArray(child)) {
          child.forEach(visit)
        } else if (child && typeof child === 'object') {
          visit(child)
        }
      }
    }

    visit(ast)
  } catch (error) {
    console.warn('JavaScript解析失败:', error)
  }

  return matches
}

/**
 * 在文本中查找符号（简单文本搜索）
 */
function findSymbolInText(symbolName: string, content: string, filePath: string): SymbolMatch[] {
  const matches: SymbolMatch[] = []
  const lines = content.split('\n')

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    if (line.includes(symbolName)) {
      const column = line.indexOf(symbolName)
      const context = getLineContext(lines, i)

      matches.push({
        name: symbolName,
        type: 'text',
        file: filePath,
        line: i + 1,
        column,
        context,
      })
    }
  }

  return matches
}

/**
 * 获取行上下文
 */
function getLineContext(lines: string[], lineIndex: number): string {
  const start = Math.max(0, lineIndex - 1)
  const end = Math.min(lines.length, lineIndex + 2)
  return lines.slice(start, end).join('\n')
}

/**
 * 格式化搜索结果
 */
function formatSearchResults(symbolName: string, matches: SymbolMatch[]): string {
  if (matches.length === 0) {
    return `未找到符号 "${symbolName}"`
  }

  let output = `找到 ${matches.length} 个匹配的符号 "${symbolName}":\n\n`

  // 按文件分组
  const groupedMatches = new Map<string, SymbolMatch[]>()
  for (const match of matches) {
    if (!groupedMatches.has(match.file)) {
      groupedMatches.set(match.file, [])
    }
    groupedMatches.get(match.file)!.push(match)
  }

  for (const [file, fileMatches] of groupedMatches) {
    output += `📁 ${basename(file)}\n`

    for (const match of fileMatches) {
      const icon = getSymbolIcon(match.type)
      output += `  ${icon} ${match.name} (${match.type}) - 第${match.line}行:${match.column}列\n`

      // 显示代码上下文
      const contextLines = match.context.split('\n')
      for (const contextLine of contextLines) {
        if (contextLine.trim()) {
          output += `    ${contextLine.trim()}\n`
        }
      }
      output += '\n'
    }
  }

  return output
}

/**
 * 获取符号图标
 */
function getSymbolIcon(type: string): string {
  switch (type) {
    case 'function':
      return '🔧'
    case 'class':
      return '🏗️'
    case 'variable':
      return '📦'
    case 'interface':
      return '📋'
    case 'type':
      return '🏷️'
    case 'text':
      return '📄'
    default:
      return '❓'
  }
}
