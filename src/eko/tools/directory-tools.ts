/**
 * 目录操作相关工具
 */

import type { AgentContext } from '@eko-ai/eko'
import type { Tool, ToolResult } from '../types'
import { terminalAPI } from '@/api/terminal'
import { getAgentName, getOrCreateAgentTerminal, sleep } from './utils'
import type { ListDirectoryParams, CreateDirectoryParams, ChangeDirectoryParams } from './types'

/**
 * 📂 列出目录内容工具
 */
export const listDirectoryTool: Tool = {
  name: 'list_directory',
  description: '📂 列出目录内容：查看目录中的文件和子目录，支持显示隐藏文件和详细信息',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '目录路径，默认为当前目录',
        default: '.',
      },
      showHidden: {
        type: 'boolean',
        description: '是否显示隐藏文件，默认false',
        default: false,
      },
      detailed: {
        type: 'boolean',
        description: '是否显示详细信息，默认false',
        default: false,
      },
    },
    required: [],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const { path = '.', showHidden = false, detailed = false } = params as ListDirectoryParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行目录列表操作',
            },
          ],
        }
      }

      // 构建ls命令
      let lsCommand = 'ls'
      if (detailed) lsCommand += ' -l'
      if (showHidden) lsCommand += ' -a'
      lsCommand += ` "${path}"\n`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: lsCommand,
      })

      await sleep(500)

      const output = await terminalAPI.getTerminalBuffer(terminalId)

      return {
        content: [
          {
            type: 'text',
            text: `📂 目录内容 (${path}):\n\n${output.slice(-2000)}`,
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 列出目录失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 📁 创建目录工具
 */
export const createDirectoryTool: Tool = {
  name: 'create_directory',
  description: '📁 创建目录：创建新的文件夹，支持递归创建多级目录',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '要创建的目录路径',
      },
      recursive: {
        type: 'boolean',
        description: '是否递归创建父目录，默认true',
        default: true,
      },
    },
    required: ['path'],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const { path, recursive = true } = params as CreateDirectoryParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行目录创建操作',
            },
          ],
        }
      }

      // 构建mkdir命令
      const mkdirCommand = recursive ? `mkdir -p "${path}"` : `mkdir "${path}"`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: `${mkdirCommand}\n`,
      })

      await sleep(300)

      return {
        content: [
          {
            type: 'text',
            text: `✅ 目录创建成功: ${path}`,
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 创建目录失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 🚶 切换工作目录工具
 */
export const changeDirectoryTool: Tool = {
  name: 'change_directory',
  description: '🚶 切换工作目录：改变当前所在的目录位置',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '要切换到的目录路径',
      },
    },
    required: ['path'],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const { path } = params as ChangeDirectoryParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行目录切换操作',
            },
          ],
        }
      }

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: `cd "${path}"\n`,
      })

      await sleep(300)

      // 验证切换是否成功
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: 'pwd\n',
      })

      await sleep(300)

      const output = await terminalAPI.getTerminalBuffer(terminalId)

      return {
        content: [
          {
            type: 'text',
            text: `✅ 已切换到目录: ${path}\n当前目录: ${output.slice(-200).trim()}`,
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 切换目录失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 📍 获取当前目录工具
 */
export const getCurrentDirectoryTool: Tool = {
  name: 'get_current_directory',
  description: '📍 获取当前目录：显示当前所在的目录路径。这是查询当前目录的首选工具',
  parameters: {
    type: 'object',
    properties: {},
    required: [],
  },
  execute: async (_params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来获取当前目录',
            },
          ],
        }
      }

      // 使用更可靠的命令和标记来获取当前目录
      const marker = `PWD_RESULT_${Date.now()}`
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: `echo "${marker}_START" && pwd && echo "${marker}_END"\n`,
      })

      // 等待更长时间确保命令执行完成
      await sleep(1000)

      const output = await terminalAPI.getTerminalBuffer(terminalId)

      // 提取标记之间的内容
      const startMarker = `${marker}_START`
      const endMarker = `${marker}_END`
      const startIndex = output.indexOf(startMarker)
      const endIndex = output.indexOf(endMarker)

      let currentDir = ''

      if (startIndex !== -1 && endIndex !== -1) {
        // 提取标记之间的内容
        const content = output.substring(startIndex + startMarker.length, endIndex)
        // 清理ANSI转义序列和控制字符
        currentDir =
          content
            .replace(/\x1b\[[0-9;]*m/g, '') // 移除ANSI颜色代码
            .replace(/\r/g, '') // 移除回车符
            .replace(/\n+/g, '\n') // 合并多个换行符
            .trim()
            .split('\n')
            .filter(line => line.trim() && !line.includes(marker))
            .pop() || ''
      }

      // 如果标记方法失败，尝试简单解析
      if (!currentDir) {
        // 清理输出并尝试找到路径
        const cleanOutput = output
          .replace(/\x1b\[[0-9;]*m/g, '') // 移除ANSI颜色代码
          .replace(/\r/g, '') // 移除回车符
          .split('\n')
          .map(line => line.trim())
          .filter(line => line && line.startsWith('/')) // 查找以/开头的路径
          .pop()

        currentDir = cleanOutput || '未知'
      }

      return {
        content: [
          {
            type: 'text',
            text: `📍 当前目录: ${currentDir}`,
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 获取当前目录失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}
