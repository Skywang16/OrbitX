/**
 * 终端专用工具集
 * 为终端Agent提供各种终端操作工具
 */

import type { AgentContext } from '@eko-ai/eko'
import type {
  Tool,
  ToolResult,
  ExecuteCommandParams,
  ReadFileParams,
  WriteFileParams,
  ListDirectoryParams,
  GetTerminalStatusParams,
} from '../types'
import { terminalAPI } from '@/api/terminal'
import { aiAPI } from '@/api/ai'

/**
 * 执行命令工具
 */
export const executeCommandTool: Tool = {
  name: 'execute_command',
  description: '在终端中执行命令并返回结果',
  parameters: {
    type: 'object',
    properties: {
      command: {
        type: 'string',
        description: '要执行的命令',
      },
      terminalId: {
        type: 'number',
        description: '终端ID，可选，不指定则使用默认终端',
      },
      workingDirectory: {
        type: 'string',
        description: '工作目录，可选',
      },
      timeout: {
        type: 'number',
        description: '超时时间(毫秒)，默认30秒',
      },
    },
    required: ['command'],
  },
  execute: async (params: unknown, _context: AgentContext): Promise<ToolResult> => {
    try {
      const { command, terminalId } = params as ExecuteCommandParams

      // 安全检查 - 检查危险命令
      const dangerousCommands = ['rm -rf', 'sudo rm', 'format', 'del /f', 'shutdown', 'reboot']
      const isDangerous = dangerousCommands.some(dangerous => command.toLowerCase().includes(dangerous))

      if (isDangerous) {
        return {
          content: [
            {
              type: 'text',
              text: `⚠️ 检测到潜在危险命令: ${command}\n为了安全起见，此命令被阻止执行。如果确实需要执行，请手动在终端中运行。`,
            },
          ],
        }
      }

      // 获取终端列表，确保终端存在
      const terminals = await terminalAPI.listTerminals()
      const targetTerminalId = terminalId || (terminals.length > 0 ? terminals[0] : null)

      if (!targetTerminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 没有可用的终端，请先创建一个终端',
            },
          ],
        }
      }

      // 执行命令
      await terminalAPI.writeToTerminal({
        paneId: targetTerminalId,
        data: command + '\n',
      })

      // 等待一段时间让命令执行
      await new Promise(resolve => setTimeout(resolve, 1000))

      // 获取终端缓冲区内容作为结果
      const output = await terminalAPI.getTerminalBuffer(targetTerminalId)

      return {
        content: [
          {
            type: 'text',
            text: `✅ 命令执行完成: ${command}\n\n输出:\n${output.slice(-1000)}`, // 只显示最后1000字符
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 命令执行失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 获取终端状态工具
 */
export const getTerminalStatusTool: Tool = {
  name: 'get_terminal_status',
  description: '获取终端状态信息，包括当前目录、环境变量等',
  parameters: {
    type: 'object',
    properties: {
      terminalId: {
        type: 'number',
        description: '终端ID，可选',
      },
      detailed: {
        type: 'boolean',
        description: '是否返回详细信息',
      },
    },
  },
  execute: async (params: unknown, _context: AgentContext): Promise<ToolResult> => {
    try {
      const { terminalId, detailed = false } = params as GetTerminalStatusParams

      // 获取终端列表
      const terminals = await terminalAPI.listTerminals()
      const targetTerminalId = terminalId || (terminals.length > 0 ? terminals[0] : null)

      if (!targetTerminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 没有可用的终端',
            },
          ],
        }
      }

      // 获取终端上下文
      const terminalContext = await aiAPI.getTerminalContext()

      let statusInfo = `📊 终端状态信息:\n`
      statusInfo += `- 终端ID: ${targetTerminalId}\n`
      statusInfo += `- 当前目录: ${terminalContext.workingDirectory || '未知'}\n`
      statusInfo += `- 活跃终端数: ${terminals.length}\n`

      if (detailed) {
        statusInfo += `- 所有终端ID: ${terminals.join(', ')}\n`
        statusInfo += `- 系统信息: ${JSON.stringify(terminalContext.systemInfo || {}, null, 2)}\n`
      }

      return {
        content: [
          {
            type: 'text',
            text: statusInfo,
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 获取终端状态失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 读取文件工具
 */
export const readFileTool: Tool = {
  name: 'read_file',
  description: '读取文件内容',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '文件路径',
      },
      encoding: {
        type: 'string',
        description: '编码格式，默认utf-8',
      },
    },
    required: ['path'],
  },
  execute: async (params: unknown, _context: AgentContext): Promise<ToolResult> => {
    try {
      const { path } = params as ReadFileParams

      // 使用cat命令读取文件
      const terminals = await terminalAPI.listTerminals()
      const terminalId = terminals.length > 0 ? terminals[0] : null

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 没有可用的终端来执行文件读取操作',
            },
          ],
        }
      }

      // 执行cat命令
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: `cat "${path}"\n`,
      })

      // 等待命令执行
      await new Promise(resolve => setTimeout(resolve, 1000))

      // 获取输出
      const output = await terminalAPI.getTerminalBuffer(terminalId)

      return {
        content: [
          {
            type: 'text',
            text: `📄 文件内容 (${path}):\n\n${output.slice(-2000)}`, // 显示最后2000字符
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 读取文件失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 写入文件工具
 */
export const writeFileTool: Tool = {
  name: 'write_file',
  description: '写入内容到文件',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '文件路径',
      },
      content: {
        type: 'string',
        description: '要写入的内容',
      },
      append: {
        type: 'boolean',
        description: '是否追加到文件末尾，默认false（覆盖）',
      },
    },
    required: ['path', 'content'],
  },
  execute: async (params: unknown, _context: AgentContext): Promise<ToolResult> => {
    try {
      const { path, content, append = false } = params as WriteFileParams

      const terminals = await terminalAPI.listTerminals()
      const terminalId = terminals.length > 0 ? terminals[0] : null

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 没有可用的终端来执行文件写入操作',
            },
          ],
        }
      }

      // 使用echo命令写入文件
      const operator = append ? '>>' : '>'
      const command = `echo "${content.replace(/"/g, '\\"')}" ${operator} "${path}"\n`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: command,
      })

      // 等待命令执行
      await new Promise(resolve => setTimeout(resolve, 500))

      return {
        content: [
          {
            type: 'text',
            text: `✅ 文件${append ? '追加' : '写入'}成功: ${path}`,
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 写入文件失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 列出目录工具
 */
export const listDirectoryTool: Tool = {
  name: 'list_directory',
  description: '列出目录中的文件和子目录',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '目录路径，默认为当前目录',
      },
      showHidden: {
        type: 'boolean',
        description: '是否显示隐藏文件',
      },
      detailed: {
        type: 'boolean',
        description: '是否显示详细信息（文件大小、权限等）',
      },
    },
  },
  execute: async (params: unknown, _context: AgentContext): Promise<ToolResult> => {
    try {
      const { path = '.', showHidden = false, detailed = false } = params as ListDirectoryParams

      const terminals = await terminalAPI.listTerminals()
      const terminalId = terminals.length > 0 ? terminals[0] : null

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 没有可用的终端来执行目录列表操作',
            },
          ],
        }
      }

      // 构建ls命令
      let command = 'ls'
      if (detailed) command += ' -l'
      if (showHidden) command += ' -a'
      command += ` "${path}"\n`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: command,
      })

      // 等待命令执行
      await new Promise(resolve => setTimeout(resolve, 1000))

      // 获取输出
      const output = await terminalAPI.getTerminalBuffer(terminalId)

      return {
        content: [
          {
            type: 'text',
            text: `📁 目录列表 (${path}):\n\n${output.slice(-1500)}`,
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
 * 创建目录工具
 */
export const createDirectoryTool: Tool = {
  name: 'create_directory',
  description: '创建新目录',
  parameters: {
    type: 'object',
    properties: {
      path: {
        type: 'string',
        description: '要创建的目录路径',
      },
      recursive: {
        type: 'boolean',
        description: '是否递归创建父目录',
      },
    },
    required: ['path'],
  },
  execute: async (params: unknown, _context: AgentContext): Promise<ToolResult> => {
    try {
      const { path, recursive = false } = params as { path: string; recursive?: boolean }

      const terminals = await terminalAPI.listTerminals()
      const terminalId = terminals.length > 0 ? terminals[0] : null

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 没有可用的终端来执行目录创建操作',
            },
          ],
        }
      }

      // 构建mkdir命令
      const command = `mkdir ${recursive ? '-p ' : ''}"${path}"\n`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: command,
      })

      // 等待命令执行
      await new Promise(resolve => setTimeout(resolve, 500))

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
 * 切换目录工具
 */
export const changeDirectoryTool: Tool = {
  name: 'change_directory',
  description: '切换当前工作目录',
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
  execute: async (params: unknown, _context: AgentContext): Promise<ToolResult> => {
    try {
      const { path } = params as { path: string }

      const terminals = await terminalAPI.listTerminals()
      const terminalId = terminals.length > 0 ? terminals[0] : null

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 没有可用的终端来执行目录切换操作',
            },
          ],
        }
      }

      // 执行cd命令
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: `cd "${path}"\n`,
      })

      // 等待命令执行
      await new Promise(resolve => setTimeout(resolve, 500))

      // 执行pwd确认当前目录
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: 'pwd\n',
      })

      await new Promise(resolve => setTimeout(resolve, 500))

      // 获取当前目录
      const output = await terminalAPI.getTerminalBuffer(terminalId)

      return {
        content: [
          {
            type: 'text',
            text: `✅ 目录切换完成\n当前目录: ${output.slice(-200).trim()}`,
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
 * 获取当前工作目录工具
 */
export const getCurrentDirectoryTool: Tool = {
  name: 'get_current_directory',
  description: '获取当前工作目录',
  parameters: {
    type: 'object',
    properties: {},
  },
  execute: async (_params: unknown, _context: AgentContext): Promise<ToolResult> => {
    try {
      const terminals = await terminalAPI.listTerminals()
      const terminalId = terminals.length > 0 ? terminals[0] : null

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 没有可用的终端',
            },
          ],
        }
      }

      // 执行pwd命令
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: 'pwd\n',
      })

      // 等待命令执行
      await new Promise(resolve => setTimeout(resolve, 500))

      // 获取输出
      const output = await terminalAPI.getTerminalBuffer(terminalId)
      const lines = output.split('\n')
      const currentDir = lines[lines.length - 2] || lines[lines.length - 1]

      return {
        content: [
          {
            type: 'text',
            text: `📍 当前工作目录: ${currentDir.trim()}`,
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

/**
 * 所有终端工具的集合
 */
export const terminalTools: Tool[] = [
  executeCommandTool,
  getTerminalStatusTool,
  readFileTool,
  writeFileTool,
  listDirectoryTool,
  createDirectoryTool,
  changeDirectoryTool,
  getCurrentDirectoryTool,
]
