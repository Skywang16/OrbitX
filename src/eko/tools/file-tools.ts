/**
 * 文件操作相关工具
 */

import type { AgentContext } from '@eko-ai/eko'
import type { Tool, ToolResult } from '../types'
import { terminalAPI } from '@/api/terminal'
import { getAgentName, getOrCreateAgentTerminal, sleep, escapeShellArg } from './utils'
import type { EnhancedReadFileParams, SaveFileParams, WriteFileParams } from './types'

/**
 * 📖 增强版文件读取工具
 */
export const enhancedReadFileTool: Tool = {
  name: 'read_file_enhanced',
  description: '📖 读取文件内容：查看任何文件的内容，支持行号显示、指定行范围、文件信息。用于查看代码、配置文件等',
  parameters: {
    type: 'object',
    properties: {
      file_path: {
        type: 'string',
        description: '要读取的文件路径',
      },
      show_line_numbers: {
        type: 'boolean',
        description: '是否显示行号，默认true',
        default: true,
      },
      start_line: {
        type: 'number',
        description: '开始读取的行号（从1开始），可选',
      },
      end_line: {
        type: 'number',
        description: '结束读取的行号，可选',
      },
      show_file_info: {
        type: 'boolean',
        description: '是否显示文件信息（大小、修改时间等），默认true',
        default: true,
      },
    },
    required: ['file_path'],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const {
        file_path,
        show_line_numbers = true,
        start_line,
        end_line,
        show_file_info = true,
      } = params as EnhancedReadFileParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行文件读取操作',
            },
          ],
        }
      }

      // 1. 检查文件是否存在并获取文件信息
      if (show_file_info) {
        await terminalAPI.writeToTerminal({
          paneId: terminalId,
          data: `if [ -f "${file_path}" ]; then ls -la "${file_path}"; wc -l "${file_path}"; file "${file_path}"; else echo "FILE_NOT_FOUND"; fi\n`,
        })
        await sleep(500)
      } else {
        await terminalAPI.writeToTerminal({
          paneId: terminalId,
          data: `if [ -f "${file_path}" ]; then echo "FILE_EXISTS"; else echo "FILE_NOT_FOUND"; fi\n`,
        })
        await sleep(300)
      }

      let output = await terminalAPI.getTerminalBuffer(terminalId)

      if (output.includes('FILE_NOT_FOUND')) {
        return {
          content: [
            {
              type: 'text',
              text: `❌ 文件不存在: ${file_path}`,
            },
          ],
        }
      }

      // 2. 构建读取命令
      let readCommand = ''

      if (start_line && end_line) {
        // 读取指定行范围
        if (show_line_numbers) {
          readCommand = `sed -n '${start_line},${end_line}p' "${file_path}" | nl -ba -s': ' -w4 -v${start_line}\n`
        } else {
          readCommand = `sed -n '${start_line},${end_line}p' "${file_path}"\n`
        }
      } else if (start_line) {
        // 从指定行开始读取
        if (show_line_numbers) {
          readCommand = `tail -n +${start_line} "${file_path}" | nl -ba -s': ' -w4 -v${start_line}\n`
        } else {
          readCommand = `tail -n +${start_line} "${file_path}"\n`
        }
      } else {
        // 读取整个文件
        if (show_line_numbers) {
          readCommand = `nl -ba -s': ' -w4 "${file_path}"\n`
        } else {
          readCommand = `cat "${file_path}"\n`
        }
      }

      // 3. 执行读取命令
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: readCommand,
      })

      await sleep(1000)
      output = await terminalAPI.getTerminalBuffer(terminalId)

      // 4. 格式化输出结果
      let resultText = ''

      if (show_file_info) {
        resultText += `📁 文件信息:\n`
        const lines = output.split('\n')
        for (const line of lines) {
          if (line.includes(file_path) && (line.includes('-rw') || line.includes('-r-'))) {
            resultText += `${line}\n`
            break
          }
        }
        resultText += `\n📄 文件内容:\n`
      }

      // 提取实际的文件内容
      const contentLines = output.split('\n')
      let contentStartIndex = -1

      for (let i = 0; i < contentLines.length; i++) {
        const line = contentLines[i]
        if (show_line_numbers && /^\s*\d+:\s/.test(line)) {
          contentStartIndex = i
          break
        } else if (!show_line_numbers && !line.includes(file_path) && !line.includes('FILE_') && line.trim() !== '') {
          contentStartIndex = i
          break
        }
      }

      if (contentStartIndex >= 0) {
        const fileContent = contentLines.slice(contentStartIndex).join('\n').trim()
        resultText += fileContent
      } else {
        resultText += '(文件为空或无法读取内容)'
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
            text: `❌ 读取文件失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 💾 专业文件创建工具
 */
export const saveFileTool: Tool = {
  name: 'save_file',
  description: '💾 创建新文件：从零开始创建文件，支持自动创建目录、设置编码和权限。用于创建新的代码文件、配置文件等',
  parameters: {
    type: 'object',
    properties: {
      file_path: {
        type: 'string',
        description: '要创建的文件路径',
      },
      content: {
        type: 'string',
        description: '文件内容',
      },
      encoding: {
        type: 'string',
        description: '文件编码，默认utf-8',
        default: 'utf-8',
      },
      create_directories: {
        type: 'boolean',
        description: '是否自动创建不存在的目录，默认true',
        default: true,
      },
      overwrite: {
        type: 'boolean',
        description: '如果文件已存在是否覆盖，默认false',
        default: false,
      },
      file_permissions: {
        type: 'string',
        description: '文件权限（如644, 755），可选',
      },
      add_newline: {
        type: 'boolean',
        description: '是否在文件末尾添加换行符，默认true',
        default: true,
      },
    },
    required: ['file_path', 'content'],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const {
        file_path,
        content,
        encoding = 'utf-8',
        create_directories = true,
        overwrite = false,
        file_permissions,
        add_newline = true,
      } = params as SaveFileParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行文件创建操作',
            },
          ],
        }
      }

      let resultText = `📝 创建文件: ${file_path}\n\n`

      // 1. 检查文件是否已存在
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: `if [ -f "${file_path}" ]; then echo "FILE_EXISTS"; else echo "FILE_NOT_EXISTS"; fi\n`,
      })

      await sleep(300)
      let output = await terminalAPI.getTerminalBuffer(terminalId)

      const fileExists = output.includes('FILE_EXISTS')

      if (fileExists && !overwrite) {
        return {
          content: [
            {
              type: 'text',
              text: `❌ 文件已存在: ${file_path}。如需覆盖，请设置 overwrite: true`,
            },
          ],
        }
      }

      // 2. 创建目录（如果需要）
      if (create_directories) {
        await terminalAPI.writeToTerminal({
          paneId: terminalId,
          data: `mkdir -p "$(dirname "${file_path}")"\n`,
        })
        await sleep(300)
        resultText += `📁 已确保目录存在: $(dirname "${file_path}")\n`
      }

      // 3. 准备文件内容
      let finalContent = content
      if (add_newline && !content.endsWith('\n')) {
        finalContent += '\n'
      }

      // 4. 使用Python创建文件（更好的编码支持）
      const createScript = `
python3 << 'PYTHON_EOF'
import sys
try:
    content = """${finalContent.replace(/"/g, '\\"').replace(/\$/g, '\\$')}"""
    
    with open("${file_path}", 'w', encoding='${encoding}') as f:
        f.write(content)
    
    print("CREATE_SUCCESS")
    
    # 获取文件信息
    import os
    stat = os.stat("${file_path}")
    print(f"FILE_SIZE:{stat.st_size}")
    
except Exception as e:
    print(f"CREATE_ERROR:{str(e)}")
    sys.exit(1)
PYTHON_EOF
`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: createScript,
      })

      await sleep(1000)
      output = await terminalAPI.getTerminalBuffer(terminalId)

      if (output.includes('CREATE_ERROR:')) {
        const errorMatch = output.match(/CREATE_ERROR:(.+)/)
        const errorMsg = errorMatch ? errorMatch[1].trim() : '创建失败'
        return {
          content: [
            {
              type: 'text',
              text: `❌ ${errorMsg}`,
            },
          ],
        }
      }

      if (!output.includes('CREATE_SUCCESS')) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 文件创建操作失败',
            },
          ],
        }
      }

      // 5. 设置文件权限（如果指定）
      if (file_permissions) {
        await terminalAPI.writeToTerminal({
          paneId: terminalId,
          data: `chmod ${file_permissions} "${file_path}"\n`,
        })
        await sleep(300)
        resultText += `🔒 已设置文件权限: ${file_permissions}\n`
      }

      // 提取文件大小
      const sizeMatch = output.match(/FILE_SIZE:(\d+)/)
      const fileSize = sizeMatch ? parseInt(sizeMatch[1]) : 0

      resultText += `✅ 文件创建成功!\n`
      resultText += `- 路径: ${file_path}\n`
      resultText += `- 大小: ${fileSize} 字节\n`
      resultText += `- 编码: ${encoding}\n`
      resultText += `- 行数: ${content.split('\n').length}\n`

      if (fileExists) {
        resultText += `- 操作: 覆盖现有文件\n`
      } else {
        resultText += `- 操作: 创建新文件\n`
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
            text: `❌ 文件创建失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}

/**
 * 📝 快速写入/追加工具
 */
export const writeFileTool: Tool = {
  name: 'write_file',
  description: '📝 快速写入/追加内容：简单的文本写入，支持追加模式。用于快速添加日志、注释、简单内容等',
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
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const { path, content, append = false } = params as WriteFileParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行文件写入操作',
            },
          ],
        }
      }

      // 使用echo命令写入文件
      const operator = append ? '>>' : '>'
      const escapedContent = escapeShellArg(content)
      const command = `echo "${escapedContent}" ${operator} "${path}"\n`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: command,
      })

      await sleep(500)

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
