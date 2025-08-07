/**
 * 搜索相关工具
 */

import type { AgentContext } from '@eko-ai/eko'
import type { Tool, ToolResult } from '../types'
import { terminalAPI } from '@/api/terminal'
import { getAgentName, getOrCreateAgentTerminal, sleep } from './utils'
import type { CodeSearchParams } from './types'

/**
 * 🔍 代码搜索工具
 */
export const codeSearchTool: Tool = {
  name: 'search_code',
  description: '🔍 搜索代码/文本：在文件中查找特定内容，支持正则表达式、文件类型过滤。用于查找函数、变量、配置项等',
  parameters: {
    type: 'object',
    properties: {
      pattern: {
        type: 'string',
        description: '要搜索的模式或文本',
      },
      file_path: {
        type: 'string',
        description: '要搜索的文件路径，可选。如果不提供则在当前目录递归搜索',
      },
      directory: {
        type: 'string',
        description: '要搜索的目录路径，默认为当前目录',
        default: '.',
      },
      case_sensitive: {
        type: 'boolean',
        description: '是否区分大小写，默认false',
        default: false,
      },
      regex: {
        type: 'boolean',
        description: '是否使用正则表达式，默认false',
        default: false,
      },
      show_line_numbers: {
        type: 'boolean',
        description: '是否显示行号，默认true',
        default: true,
      },
      context_lines: {
        type: 'number',
        description: '显示匹配行前后的上下文行数，默认2',
        default: 2,
      },
      file_extensions: {
        type: 'string',
        description: '限制搜索的文件扩展名，用逗号分隔，如 "js,ts,vue"',
      },
    },
    required: ['pattern'],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const {
        pattern,
        file_path,
        directory = '.',
        case_sensitive = false,
        regex = false,
        show_line_numbers = true,
        context_lines = 2,
        file_extensions,
      } = params as CodeSearchParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行代码搜索操作',
            },
          ],
        }
      }

      // 构建grep命令
      let grepCommand = 'grep'

      // 添加选项
      if (!case_sensitive) grepCommand += ' -i'
      if (regex) grepCommand += ' -E'
      if (show_line_numbers) grepCommand += ' -n'
      if (context_lines > 0) grepCommand += ` -C ${context_lines}`
      grepCommand += ' --color=never' // 禁用颜色输出

      // 转义搜索模式中的特殊字符（如果不是正则表达式）
      let searchPattern = pattern
      if (!regex) {
        searchPattern = pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      }

      if (file_path) {
        // 搜索单个文件
        grepCommand += ` "${searchPattern}" "${file_path}"`
      } else {
        // 递归搜索目录
        grepCommand += ' -r'

        // 添加文件扩展名过滤
        if (file_extensions) {
          const extensions = file_extensions.split(',').map(ext => ext.trim())
          const includePattern = extensions.map(ext => `--include="*.${ext}"`).join(' ')
          grepCommand += ` ${includePattern}`
        }

        grepCommand += ` "${searchPattern}" "${directory}"`
      }

      grepCommand += '\n'

      // 执行搜索命令
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: grepCommand,
      })

      await sleep(1500)

      const output = await terminalAPI.getTerminalBuffer(terminalId)

      // 解析搜索结果
      const lines = output.split('\n')
      let resultText = `🔍 搜索结果 (模式: "${pattern}"):\n\n`

      // 过滤出实际的搜索结果
      const searchResults = lines.filter(line => {
        return line.includes(':') && !line.includes(grepCommand.trim()) && !line.includes('grep') && line.trim() !== ''
      })

      if (searchResults.length === 0) {
        resultText += '❌ 未找到匹配的结果'
      } else {
        resultText += `✅ 找到 ${searchResults.length} 个匹配项:\n\n`

        // 按文件分组显示结果
        const fileGroups: { [key: string]: string[] } = {}

        for (const result of searchResults) {
          const colonIndex = result.indexOf(':')
          if (colonIndex > 0) {
            const filePath = result.substring(0, colonIndex)
            if (!fileGroups[filePath]) {
              fileGroups[filePath] = []
            }
            fileGroups[filePath].push(result)
          }
        }

        for (const [filePath, matches] of Object.entries(fileGroups)) {
          resultText += `📁 ${filePath}:\n`
          for (const match of matches.slice(0, 10)) {
            // 限制每个文件最多显示10个匹配
            resultText += `  ${match}\n`
          }
          if (matches.length > 10) {
            resultText += `  ... 还有 ${matches.length - 10} 个匹配项\n`
          }
          resultText += '\n'
        }
      }

      return {
        content: [
          {
            type: 'text',
            text: resultText,
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 代码搜索失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}
