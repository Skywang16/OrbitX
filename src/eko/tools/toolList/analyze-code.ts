/**
 * 代码分析工具 - 使用AST分析代码结构
 */

import type { Tool, AnalyzeCodeParams, CodeAnalysis, CodeSymbol } from '../../types'
import { readFile, readDir } from '@tauri-apps/plugin-fs'
import { parse } from '@typescript-eslint/typescript-estree'
import * as acorn from 'acorn'
import { join, extname, basename } from 'path'

export const analyzeCodeTool: Tool<AnalyzeCodeParams> = {
  name: 'analyze-code',
  description:
    '🔬 AST代码结构分析：基于语法树精确分析代码结构，提取所有符号定义（函数、类、变量、接口等）和依赖关系。适用于：了解代码整体架构、获取完整符号列表、分析模块依赖。与orbit_context的区别：这个工具解析语法结构，orbit_context做文本搜索',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '要分析的文件或目录路径',
      },
      recursive: {
        type: 'boolean',
        description: '是否递归分析子目录',
        default: false,
      },
      include: {
        type: 'array',
        items: { type: 'string' },
        description: '包含的文件模式，如 ["*.ts", "*.js"]',
      },
      exclude: {
        type: 'array',
        items: { type: 'string' },
        description: '排除的文件模式，如 ["node_modules", "*.test.ts"]',
      },
    },
    required: ['path'],
  },

  async execute(params, context) {
    try {
      const { path, recursive = false, include = [], exclude = [] } = params

      // 检查路径是文件还是目录
      const isFile = await checkIsFile(path)

      let filesToAnalyze: string[] = []

      if (isFile) {
        filesToAnalyze = [path]
      } else {
        filesToAnalyze = await getFilesToAnalyze(path, recursive, include, exclude)
      }

      const results: CodeAnalysis[] = []

      for (const filePath of filesToAnalyze) {
        try {
          const analysis = await analyzeFile(filePath)
          if (analysis) {
            results.push(analysis)
          }
        } catch (error) {
          console.warn(`分析文件 ${filePath} 失败:`, error)
        }
      }

      return {
        content: [
          {
            type: 'text',
            text: formatAnalysisResults(results),
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `代码分析失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 检查路径是否为文件
 */
async function checkIsFile(path: string): Promise<boolean> {
  try {
    // 简单的文件检查 - 如果有扩展名就认为是文件
    return extname(path) !== ''
  } catch {
    return false
  }
}

/**
 * 获取要分析的文件列表
 */
async function getFilesToAnalyze(
  dirPath: string,
  recursive: boolean,
  include: string[],
  exclude: string[]
): Promise<string[]> {
  const files: string[] = []

  try {
    const entries = await readDir(dirPath)

    for (const entry of entries) {
      const fullPath = join(dirPath, entry.name)

      if (entry.isDirectory) {
        if (recursive && !shouldExclude(entry.name, exclude)) {
          const subFiles = await getFilesToAnalyze(fullPath, recursive, include, exclude)
          files.push(...subFiles)
        }
      } else {
        if (shouldInclude(entry.name, include) && !shouldExclude(entry.name, exclude)) {
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
 * 检查文件是否应该包含
 */
function shouldInclude(fileName: string, include: string[]): boolean {
  if (include.length === 0) {
    // 默认包含常见的代码文件
    const ext = extname(fileName)
    return ['.ts', '.tsx', '.js', '.jsx', '.py', '.rs', '.go'].includes(ext)
  }

  return include.some(pattern => {
    if (pattern.includes('*')) {
      const regex = new RegExp(pattern.replace(/\*/g, '.*'))
      return regex.test(fileName)
    }
    return fileName.includes(pattern)
  })
}

/**
 * 检查文件是否应该排除
 */
function shouldExclude(fileName: string, exclude: string[]): boolean {
  return exclude.some(pattern => {
    if (pattern.includes('*')) {
      const regex = new RegExp(pattern.replace(/\*/g, '.*'))
      return regex.test(fileName)
    }
    return fileName.includes(pattern)
  })
}

/**
 * 分析单个文件
 */
async function analyzeFile(filePath: string): Promise<CodeAnalysis | null> {
  try {
    const content = await readFile(filePath)
    const text = new TextDecoder().decode(content)
    const ext = extname(filePath)

    let language = 'unknown'
    let symbols: CodeSymbol[] = []
    let imports: string[] = []
    let exports: string[] = []

    if (['.ts', '.tsx'].includes(ext)) {
      language = 'typescript'
      const result = analyzeTypeScript(text, filePath)
      symbols = result.symbols
      imports = result.imports
      exports = result.exports
    } else if (['.js', '.jsx'].includes(ext)) {
      language = 'javascript'
      const result = analyzeJavaScript(text, filePath)
      symbols = result.symbols
      imports = result.imports
      exports = result.exports
    }

    return {
      file: filePath,
      language,
      symbols,
      imports,
      exports,
    }
  } catch (error) {
    console.warn(`分析文件 ${filePath} 失败:`, error)
    return null
  }
}

/**
 * 分析TypeScript代码
 */
function analyzeTypeScript(content: string, filePath: string) {
  const symbols: CodeSymbol[] = []
  const imports: string[] = []
  const exports: string[] = []

  try {
    const ast = parse(content, {
      loc: true,
      range: true,
      jsx: filePath.endsWith('.tsx'),
    })

    // 遍历AST节点
    function visit(node: any) {
      if (!node) return

      switch (node.type) {
        case 'FunctionDeclaration':
          if (node.id?.name) {
            symbols.push({
              name: node.id.name,
              type: 'function',
              line: node.loc?.start?.line || 0,
              column: node.loc?.start?.column || 0,
              file: filePath,
            })
          }
          break

        case 'ClassDeclaration':
          if (node.id?.name) {
            symbols.push({
              name: node.id.name,
              type: 'class',
              line: node.loc?.start?.line || 0,
              column: node.loc?.start?.column || 0,
              file: filePath,
            })
          }
          break

        case 'VariableDeclarator':
          if (node.id?.name) {
            symbols.push({
              name: node.id.name,
              type: 'variable',
              line: node.loc?.start?.line || 0,
              column: node.loc?.start?.column || 0,
              file: filePath,
            })
          }
          break

        case 'TSInterfaceDeclaration':
          if (node.id?.name) {
            symbols.push({
              name: node.id.name,
              type: 'interface',
              line: node.loc?.start?.line || 0,
              column: node.loc?.start?.column || 0,
              file: filePath,
            })
          }
          break

        case 'TSTypeAliasDeclaration':
          if (node.id?.name) {
            symbols.push({
              name: node.id.name,
              type: 'type',
              line: node.loc?.start?.line || 0,
              column: node.loc?.start?.column || 0,
              file: filePath,
            })
          }
          break

        case 'ImportDeclaration':
          if (node.source?.value) {
            imports.push(node.source.value)
          }
          break

        case 'ExportNamedDeclaration':
        case 'ExportDefaultDeclaration':
          if (node.declaration?.id?.name) {
            exports.push(node.declaration.id.name)
          }
          break
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

  return { symbols, imports, exports }
}

/**
 * 分析JavaScript代码
 */
function analyzeJavaScript(content: string, filePath: string) {
  const symbols: CodeSymbol[] = []
  const imports: string[] = []
  const exports: string[] = []

  try {
    const ast = acorn.parse(content, {
      ecmaVersion: 'latest',
      sourceType: 'module',
      locations: true,
    })

    // 简化的AST遍历
    function visit(node: any) {
      if (!node) return

      switch (node.type) {
        case 'FunctionDeclaration':
          if (node.id?.name) {
            symbols.push({
              name: node.id.name,
              type: 'function',
              line: node.loc?.start?.line || 0,
              column: node.loc?.start?.column || 0,
              file: filePath,
            })
          }
          break

        case 'ClassDeclaration':
          if (node.id?.name) {
            symbols.push({
              name: node.id.name,
              type: 'class',
              line: node.loc?.start?.line || 0,
              column: node.loc?.start?.column || 0,
              file: filePath,
            })
          }
          break

        case 'VariableDeclarator':
          if (node.id?.name) {
            symbols.push({
              name: node.id.name,
              type: 'variable',
              line: node.loc?.start?.line || 0,
              column: node.loc?.start?.column || 0,
              file: filePath,
            })
          }
          break

        case 'ImportDeclaration':
          if (node.source?.value) {
            imports.push(node.source.value)
          }
          break
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

  return { symbols, imports, exports }
}

/**
 * 格式化分析结果
 */
function formatAnalysisResults(results: CodeAnalysis[]): string {
  if (results.length === 0) {
    return '没有找到可分析的代码文件'
  }

  let output = `代码分析结果 (共 ${results.length} 个文件):\n\n`

  for (const result of results) {
    output += `📁 ${basename(result.file)} (${result.language})\n`

    if (result.symbols.length > 0) {
      output += `  符号 (${result.symbols.length}):\n`
      for (const symbol of result.symbols) {
        const icon = getSymbolIcon(symbol.type)
        output += `    ${icon} ${symbol.name} (${symbol.type}) - 第${symbol.line}行\n`
      }
    }

    if (result.imports.length > 0) {
      output += `  导入 (${result.imports.length}): ${result.imports.join(', ')}\n`
    }

    if (result.exports.length > 0) {
      output += `  导出 (${result.exports.length}): ${result.exports.join(', ')}\n`
    }

    output += '\n'
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
    default:
      return '📄'
  }
}
