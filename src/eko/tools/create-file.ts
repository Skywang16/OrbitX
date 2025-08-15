/**
 * 文件创建工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { ValidationError } from './tool-error'
import { writeTextFile, exists } from '@tauri-apps/plugin-fs'

export interface CreateFileParams {
  filePath: string
  content: string
  overwrite?: boolean
}

/**
 * 文件创建工具
 */
export class CreateFileTool extends ModifiableTool {
  constructor() {
    super('create_file', '📄 创建文件：创建新文件或覆盖现有文件', {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: '要写入的文件路径',
        },
        content: {
          type: 'string',
          description: '文件内容',
        },
        overwrite: {
          type: 'boolean',
          description: '如果文件已存在是否覆盖，默认false',
          default: false,
        },
      },
      required: ['filePath', 'content'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { filePath, content, overwrite = false } = context.parameters as unknown as CreateFileParams

    if (!filePath?.trim()) {
      throw new ValidationError('文件路径不能为空')
    }

    try {
      const fileExists = await exists(filePath)

      if (fileExists && !overwrite) {
        return {
          content: [
            {
              type: 'text',
              text: `❌ 文件已存在: ${filePath}\n💡 如需覆盖现有文件，请设置 overwrite: true 参数`,
            },
          ],
        }
      }

      await writeTextFile(filePath, content)

      return {
        content: [
          {
            type: 'text',
            text: fileExists ? `✅ 文件已更新: ${filePath}` : `✅ 文件已创建: ${filePath}`,
          },
        ],
      }
    } catch (error) {
      throw new Error(`创建文件失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }
}

// 导出工具实例
export const createFileTool = new CreateFileTool()
