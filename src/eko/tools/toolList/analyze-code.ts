/**
 * 代码分析工具 - 使用AST分析代码结构
 */

import type { Tool } from '../../types'
import { aiApi, type AnalyzeCodeParams, type AnalysisResult, type CodeSymbol } from '@/api'

export const analyzeCodeTool: Tool<AnalyzeCodeParams> = {
  name: 'analyze-code',
  description: '分析代码结构：解析代码文件，提取函数、类、变量等符号信息',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '要分析的文件或目录路径',
      },
    },
    required: ['path'],
  },

  async execute(params, _context) {
    try {
      const { path } = params

      const result = await aiApi.analyzeCode({
        path,
        recursive: false,
        include: [],
        exclude: [],
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
    return '未找到代码文件'
  }

  let output = ''

  for (const analysis of result.analyses) {
    const fileName = analysis.file.split('/').pop() || analysis.file
    output += `${fileName}:\n`

    // 按类型分组符号
    const symbolsByType = groupSymbolsByType(analysis.symbols)

    // 输出各类型的符号
    for (const [type, symbols] of Object.entries(symbolsByType)) {
      if (symbols.length > 0) {
        output += `${getTypeDisplayName(type)}:\n`
        for (const symbol of symbols) {
          output += `  - ${symbol.name} (第${symbol.line}行)\n`
        }
      }
    }

    output += '\n'
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
      return '函数'
    case 'class':
      return '类'
    case 'variable':
      return '变量'
    case 'interface':
      return '接口'
    case 'type':
      return '类型'
    case 'enum':
      return '枚举'
    case 'method':
      return '方法'
    case 'property':
      return '属性'
    case 'constant':
      return '常量'
    default:
      return '其他'
  }
}
