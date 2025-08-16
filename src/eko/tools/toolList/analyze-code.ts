/**
 * 代码分析工具 - 使用AST分析代码结构
 */

import type { Tool } from '../../types'
import { analyzeCode, type AnalyzeCodeParams, type AnalysisResult, type CodeSymbol } from '@/api/ai/tool'

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

  async execute(params, _context) {
    try {
      const { path, recursive = false, include = [], exclude = [] } = params

      // 调用 Tauri 后端的 AST 分析命令
      const result = await analyzeCode({
        path,
        recursive,
        include,
        exclude,
      })

      return {
        content: [
          {
            type: 'text',
            text: formatAnalysisResults(result),
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
 * 格式化分析结果 - LLM 偏好格式
 */
function formatAnalysisResults(result: AnalysisResult): string {
  if (result.analyses.length === 0) {
    return 'No code files found for analysis.'
  }

  let output = `# Code Analysis Results\n\n`
  output += `**Summary:** ${result.total_files} files processed, ${result.success_count} successful, ${result.error_count} failed\n\n`

  for (const analysis of result.analyses) {
    const fileName = analysis.file.split('/').pop() || analysis.file
    output += `## File: \`${fileName}\` (${analysis.language})\n\n`

    // 按类型分组符号
    const symbolsByType = groupSymbolsByType(analysis.symbols)

    // 输出各类型的符号
    for (const [type, symbols] of Object.entries(symbolsByType)) {
      if (symbols.length > 0) {
        output += `### ${getTypeDisplayName(type)} (${symbols.length})\n\n`
        for (const symbol of symbols) {
          output += `- **${symbol.name}** (line ${symbol.line})\n`
        }
        output += '\n'
      }
    }

    // 导入信息
    if (analysis.imports.length > 0) {
      output += `### Imports (${analysis.imports.length})\n\n`
      output += '```\n'
      analysis.imports.forEach(imp => {
        output += `${imp}\n`
      })
      output += '```\n\n'
    }

    // 导出信息
    if (analysis.exports.length > 0) {
      output += `### Exports (${analysis.exports.length})\n\n`
      output += '```\n'
      analysis.exports.forEach(exp => {
        output += `${exp}\n`
      })
      output += '```\n\n'
    }

    output += '---\n\n'
  }

  return output
}

/**
 * 按类型分组符号
 */
function groupSymbolsByType(symbols: CodeSymbol[]): Record<string, CodeSymbol[]> {
  const groups: Record<string, CodeSymbol[]> = {}

  for (const symbol of symbols) {
    const type = symbol.type
    if (!groups[type]) {
      groups[type] = []
    }
    groups[type].push(symbol)
  }

  return groups
}

/**
 * 获取类型的显示名称
 */
function getTypeDisplayName(type: string): string {
  switch (type) {
    case 'function':
      return 'Functions'
    case 'class':
      return 'Classes'
    case 'variable':
      return 'Variables'
    case 'interface':
      return 'Interfaces'
    case 'type':
      return 'Type Aliases'
    case 'struct':
      return 'Structs'
    case 'enum':
      return 'Enums'
    case 'trait':
      return 'Traits'
    case 'method':
      return 'Methods'
    case 'property':
      return 'Properties'
    case 'constant':
      return 'Constants'
    case 'module':
      return 'Modules'
    case 'namespace':
      return 'Namespaces'
    case 'macro':
      return 'Macros'
    default:
      return 'Other Symbols'
  }
}
