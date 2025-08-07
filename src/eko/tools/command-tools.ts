/**
 * 命令执行相关工具
 */

import type { AgentContext } from '@eko-ai/eko'
import type { Tool, ToolResult } from '../types'
import { terminalAPI } from '@/api/terminal'
import { getAgentName, getOrCreateAgentTerminal, sleep } from './utils'
import type { ExecuteCommandParams, PreciseEditParams } from './types'

/**
 * 🔧 万能命令执行工具
 */
export const executeCommandTool: Tool = {
  name: 'execute_command',
  description:
    '🔧 万能命令执行工具：当其他专门工具无法满足需求时使用。执行任意终端命令（如npm install、git操作、系统命令等）',
  parameters: {
    type: 'object',
    properties: {
      command: {
        type: 'string',
        description: '要执行的命令',
      },
      terminalId: {
        type: 'number',
        description: '指定终端ID，可选。如果不指定则使用Agent专属终端',
      },
    },
    required: ['command'],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const { command, terminalId } = params as ExecuteCommandParams

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
              text: '❌ 无法创建或获取Agent专属终端来执行命令',
            },
          ],
        }
      }

      // 检查是否为危险命令
      const dangerousCommands = ['rm -rf /', 'sudo rm -rf', 'format', 'fdisk', 'mkfs']
      const isDangerous = dangerousCommands.some(dangerous => command.toLowerCase().includes(dangerous))

      if (isDangerous) {
        return {
          content: [
            {
              type: 'text',
              text: `⚠️ 检测到危险命令，已阻止执行: ${command}`,
            },
          ],
        }
      }

      // 执行命令
      await terminalAPI.writeToTerminal({
        paneId: targetTerminalId,
        data: `${command}\n`,
      })

      // 等待命令执行
      await sleep(2000)

      // 获取输出
      const output = await terminalAPI.getTerminalBuffer(targetTerminalId)

      return {
        content: [
          {
            type: 'text',
            text: `🔧 命令执行完成: ${command}\n\n📄 输出结果:\n${output.slice(-3000)}`,
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
 * ✏️ 精确编辑工具
 */
export const preciseEditTool: Tool = {
  name: 'precise_edit',
  description: '✏️ 精确编辑现有文件：类似IDE的查找替换，需要提供精确的上下文匹配。用于修改代码、更新配置等',
  parameters: {
    type: 'object',
    properties: {
      file_path: {
        type: 'string',
        description: '要编辑的文件的绝对路径',
      },
      old_string: {
        type: 'string',
        description:
          '要替换的精确文本内容，必须包含足够的上下文（建议前后各3行）以确保唯一匹配。必须完全匹配，包括空格、缩进、换行符',
      },
      new_string: {
        type: 'string',
        description: '替换后的新文本内容，保持正确的缩进和格式',
      },
      expected_replacements: {
        type: 'number',
        description: '期望的替换次数，默认为1。用于验证替换操作的准确性',
        default: 1,
      },
      create_backup: {
        type: 'boolean',
        description: '是否创建备份文件，默认true',
        default: true,
      },
    },
    required: ['file_path', 'old_string', 'new_string'],
  },
  execute: async (params: unknown, context: AgentContext): Promise<ToolResult> => {
    try {
      const {
        file_path,
        old_string,
        new_string,
        expected_replacements = 1,
        create_backup = true,
      } = params as PreciseEditParams

      const agentName = getAgentName(context)
      const terminalId = await getOrCreateAgentTerminal(agentName)

      if (!terminalId) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 无法创建或获取Agent专属终端来执行精确编辑操作',
            },
          ],
        }
      }

      // 1. 检查文件是否存在
      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: `if [ -f "${file_path}" ]; then echo "FILE_EXISTS"; else echo "FILE_NOT_FOUND"; fi\n`,
      })

      await sleep(500)
      let output = await terminalAPI.getTerminalBuffer(terminalId)

      const isNewFile = old_string === ''
      const fileExists = output.includes('FILE_EXISTS')

      if (!isNewFile && !fileExists) {
        return {
          content: [
            {
              type: 'text',
              text: `❌ 文件不存在: ${file_path}。如果要创建新文件，请将 old_string 设为空字符串`,
            },
          ],
        }
      }

      if (isNewFile && fileExists) {
        return {
          content: [
            {
              type: 'text',
              text: `❌ 文件已存在，无法创建: ${file_path}`,
            },
          ],
        }
      }

      // 2. 如果是新文件，直接创建
      if (isNewFile) {
        await terminalAPI.writeToTerminal({
          paneId: terminalId,
          data: `mkdir -p "$(dirname "${file_path}")"\n`,
        })
        await sleep(300)

        const tempFile = `/tmp/precise_edit_${Date.now()}.tmp`
        await terminalAPI.writeToTerminal({
          paneId: terminalId,
          data: `cat > "${tempFile}" << 'PRECISE_EDIT_EOF'\n${new_string}\nPRECISE_EDIT_EOF\n`,
        })
        await sleep(500)

        await terminalAPI.writeToTerminal({
          paneId: terminalId,
          data: `mv "${tempFile}" "${file_path}"\n`,
        })
        await sleep(300)

        return {
          content: [
            {
              type: 'text',
              text: `✅ 成功创建新文件: ${file_path}`,
            },
          ],
        }
      }

      // 3. 对于现有文件，进行精确替换验证
      const checkScript = `
python3 << 'PYTHON_EOF'
import sys
try:
    with open("${file_path}", 'r', encoding='utf-8') as f:
        content = f.read()
    
    old_text = """${old_string.replace(/"/g, '\\"').replace(/\$/g, '\\$')}"""
    
    # 计算匹配次数
    count = content.count(old_text)
    print(f"MATCH_COUNT:{count}")
    
    if count == 0:
        print("ERROR:未找到要替换的文本")
        sys.exit(1)
    elif count != ${expected_replacements}:
        print(f"ERROR:期望替换${expected_replacements}次，但找到{count}次匹配")
        sys.exit(1)
    else:
        print("VALIDATION_PASSED")
        
except Exception as e:
    print(f"ERROR:{str(e)}")
    sys.exit(1)
PYTHON_EOF
`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: checkScript,
      })

      await sleep(1000)
      output = await terminalAPI.getTerminalBuffer(terminalId)

      if (output.includes('ERROR:')) {
        const errorMatch = output.match(/ERROR:(.+)/)
        const errorMsg = errorMatch ? errorMatch[1].trim() : '验证失败'
        return {
          content: [
            {
              type: 'text',
              text: `❌ ${errorMsg}`,
            },
          ],
        }
      }

      if (!output.includes('VALIDATION_PASSED')) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 文本匹配验证失败',
            },
          ],
        }
      }

      // 4. 创建备份（如果需要）
      if (create_backup) {
        await terminalAPI.writeToTerminal({
          paneId: terminalId,
          data: `cp "${file_path}" "${file_path}.bak"\n`,
        })
        await sleep(300)
      }

      // 5. 执行精确替换
      const replaceScript = `
python3 << 'PYTHON_EOF'
try:
    with open("${file_path}", 'r', encoding='utf-8') as f:
        content = f.read()
    
    old_text = """${old_string.replace(/"/g, '\\"').replace(/\$/g, '\\$')}"""
    new_text = """${new_string.replace(/"/g, '\\"').replace(/\$/g, '\\$')}"""
    
    # 执行替换
    new_content = content.replace(old_text, new_text)
    
    with open("${file_path}", 'w', encoding='utf-8') as f:
        f.write(new_content)
    
    print("REPLACE_SUCCESS")
    
except Exception as e:
    print(f"REPLACE_ERROR:{str(e)}")
PYTHON_EOF
`

      await terminalAPI.writeToTerminal({
        paneId: terminalId,
        data: replaceScript,
      })

      await sleep(1000)
      output = await terminalAPI.getTerminalBuffer(terminalId)

      if (output.includes('REPLACE_ERROR:')) {
        const errorMatch = output.match(/REPLACE_ERROR:(.+)/)
        const errorMsg = errorMatch ? errorMatch[1].trim() : '替换失败'
        return {
          content: [
            {
              type: 'text',
              text: `❌ ${errorMsg}`,
            },
          ],
        }
      }

      if (!output.includes('REPLACE_SUCCESS')) {
        return {
          content: [
            {
              type: 'text',
              text: '❌ 文件替换操作失败',
            },
          ],
        }
      }

      // 6. 显示替换结果
      let resultMessage = `✅ 精确编辑完成: ${file_path}\n`
      resultMessage += `- 替换次数: ${expected_replacements}\n`
      if (create_backup) {
        resultMessage += `- 备份文件: ${file_path}.bak\n`
      }

      // 显示简短的变更预览
      const previewLines = Math.min(old_string.split('\n').length, 3)
      const oldPreview = old_string.split('\n').slice(0, previewLines).join('\n')
      const newPreview = new_string.split('\n').slice(0, previewLines).join('\n')

      if (oldPreview !== newPreview) {
        resultMessage += `\n📝 变更预览:\n`
        resultMessage += `- 原内容: ${oldPreview.substring(0, 50)}${oldPreview.length > 50 ? '...' : ''}\n`
        resultMessage += `+ 新内容: ${newPreview.substring(0, 50)}${newPreview.length > 50 ? '...' : ''}`
      }

      return {
        content: [
          {
            type: 'text',
            text: resultMessage,
          },
        ],
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `❌ 精确编辑失败: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      }
    }
  },
}
