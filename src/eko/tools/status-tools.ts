/**
 * 状态查询相关工具
 */

import type { AgentContext } from '@eko-ai/eko'
import type { Tool, ToolResult } from '../types'
import { terminalAPI } from '@/api/terminal'
import { aiAPI } from '@/api/ai'
import { getAgentName, getOrCreateAgentTerminal, sleep, isDangerousPath } from './utils'
import type { GetTerminalStatusParams, RemoveFilesParams } from './types'

/**
 * 📊 获取终端状态工具
 */
export const getTerminalStatusTool: Tool = {
  name: 'get_terminal_status',
  description: '📊 获取终端状态：查看终端信息，包括当前目录、环境变量、活跃终端数等',
  parameters: {
    type: 'object',
    properties: {
      terminalId: {
        type: 'number',
        description: '指定终端ID，可选。如果不指定则使用Agent专属终端',
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
      const { terminalId, detailed = false } = params as GetTerminalStatusParams

      let targetTerminalId: number
      if (terminalId) {
        targetTerminalId = terminalId
      } else {
        const agentName = getAgentName(context)
        targetTerminalId = await getOrCreateAgentTerminal(agentName)
      }

      if (!targetTerminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端',
            },
          ],
        }
      }

      // 获取终端上下文和终端列表
      const terminalContext = await aiAPI.getTerminalContext()
      const allTerminals = await terminalAPI.listTerminals()

      let statusInfo = `📊 终端状态信息:\n`
      statusInfo += `- 终端ID: ${targetTerminalId}\n`
      statusInfo += `- 当前目录: ${terminalContext.workingDirectory || '未知'}\n`
      statusInfo += `- 活跃终端数: ${allTerminals.length}\n`

      if (detailed) {
        statusInfo += `- 所有终端ID: ${allTerminals.join(', ')}\n`
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
 * 🗑️ 安全文件删除工具
 */
export const removeFilesTool: Tool = {
  name: 'remove_files',
  description: '🗑️ 安全删除文件/目录：支持预览、备份、安全检查。用于删除不需要的文件或清理项目',
  parameters: {
    type: 'object',
    properties: {
      paths: {
        type: 'array',
        items: {
          type: 'string',
        },
        description: '要删除的文件或目录路径列表',
      },
      recursive: {
        type: 'boolean',
        description: '是否递归删除目录，默认false',
        default: false,
      },
      force: {
        type: 'boolean',
        description: '是否强制删除（跳过确认），默认false。危险操作！',
        default: false,
      },
      create_backup: {
        type: 'boolean',
        description: '删除前是否创建备份，默认true',
        default: true,
      },
      dry_run: {
        type: 'boolean',
        description: '是否只是预览删除操作而不实际执行，默认false',
        default: false,
      },
    },
    required: ['paths'],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const {
        paths,
        recursive = false,
        force = false,
        create_backup = true,
        dry_run = false,
      } = params as RemoveFilesParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行文件删除操作',
            },
          ],
        }
      }

      let resultText = dry_run ? '🔍 删除预览 (不会实际删除):\n\n' : '🗑️ 文件删除操作:\n\n'
      const deletedItems: string[] = []
      const failedItems: string[] = []
      const skippedItems: string[] = []

      // 危险路径检查
      const hasDangerousPath = paths.some(path => isDangerousPath(path))

      if (hasDangerousPath && !force) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 检测到危险路径！包含系统关键目录或根目录。如果确实需要删除，请设置 force: true',
            },
          ],
        }
      }

      for (const filePath of paths) {
        try {
          // 1. 检查文件/目录是否存在
          await terminalAPI.writeToTerminal({
            paneId: terminalId,
            data: `if [ -e "${filePath}" ]; then echo "EXISTS:${filePath}"; if [ -f "${filePath}" ]; then echo "TYPE:FILE"; elif [ -d "${filePath}" ]; then echo "TYPE:DIR"; fi; else echo "NOT_EXISTS:${filePath}"; fi\n`,
          })

          await sleep(300)
          let output = await terminalAPI.getTerminalBuffer(terminalId)

          if (output.includes(`NOT_EXISTS:${filePath}`)) {
            skippedItems.push(`${filePath} (不存在)`)
            continue
          }

          const isDirectory = output.includes('TYPE:DIR')
          const isFile = output.includes('TYPE:FILE')

          // 2. 目录删除需要recursive参数
          if (isDirectory && !recursive) {
            skippedItems.push(`${filePath} (目录需要 recursive: true)`)
            continue
          }

          if (dry_run) {
            // 预览模式：只显示将要删除的内容
            resultText += `${isDirectory ? '📁' : '📄'} ${filePath}\n`
            if (isDirectory) {
              // 显示目录内容预览
              await terminalAPI.writeToTerminal({
                paneId: terminalId,
                data: `find "${filePath}" -type f | head -5\n`,
              })
              await sleep(500)
              const dirContent = await terminalAPI.getTerminalBuffer(terminalId)
              const files = dirContent.split('\n').filter(line => line.includes(filePath) && line.trim() !== filePath)
              if (files.length > 0) {
                resultText += `  包含文件: ${files.slice(0, 3).join(', ')}${files.length > 3 ? '...' : ''}\n`
              }
            }
            deletedItems.push(filePath)
            continue
          }

          // 4. 创建备份（如果需要且是文件）
          if (create_backup && isFile) {
            const backupPath = `${filePath}.deleted.${Date.now()}.bak`
            await terminalAPI.writeToTerminal({
              paneId: terminalId,
              data: `cp "${filePath}" "${backupPath}"\n`,
            })
            await sleep(300)
            resultText += `💾 已创建备份: ${backupPath}\n`
          }

          // 5. 执行删除操作
          let deleteCommand = ''
          if (isDirectory) {
            deleteCommand = `rm -rf "${filePath}"`
          } else {
            deleteCommand = `rm "${filePath}"`
          }

          await terminalAPI.writeToTerminal({
            paneId: terminalId,
            data: `${deleteCommand} && echo "DELETE_SUCCESS:${filePath}" || echo "DELETE_FAILED:${filePath}"\n`,
          })

          await sleep(500)
          output = await terminalAPI.getTerminalBuffer(terminalId)

          if (output.includes(`DELETE_SUCCESS:${filePath}`)) {
            deletedItems.push(filePath)
            resultText += `✅ 已删除: ${filePath}\n`
          } else {
            failedItems.push(filePath)
            resultText += `❌ 删除失败: ${filePath}\n`
          }
        } catch (error) {
          failedItems.push(`${filePath} (${error})`)
          resultText += `❌ 处理失败: ${filePath} - ${error}\n`
        }
      }

      // 6. 生成总结报告
      resultText += '\n📊 操作总结:\n'
      if (dry_run) {
        resultText += `- 预览项目: ${deletedItems.length}\n`
        resultText += `- 跳过项目: ${skippedItems.length}\n`
        resultText += '\n💡 要实际执行删除，请设置 dry_run: false'
      } else {
        resultText += `- 成功删除: ${deletedItems.length}\n`
        resultText += `- 删除失败: ${failedItems.length}\n`
        resultText += `- 跳过项目: ${skippedItems.length}\n`

        if (failedItems.length > 0) {
          resultText += `\n❌ 失败项目: ${failedItems.join(', ')}`
        }
        if (skippedItems.length > 0) {
          resultText += `\n⏭️ 跳过项目: ${skippedItems.join(', ')}`
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
            text: `❌ 文件删除操作失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}
