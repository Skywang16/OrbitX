/**
 * 文件创建工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { ValidationError } from '../tool-error'
import { writeTextFile } from '@tauri-apps/plugin-fs'

export interface CreateFileParams {
  filePath: string
  content: string
}

/**
 * 文件创建工具
 */
export class CreateFileTool extends ModifiableTool {
  constructor() {
    super('create_file', '创建文件：创建新文件并写入内容', {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: '文件路径',
        },
        content: {
          type: 'string',
          description: '文件内容',
        },
      },
      required: ['filePath', 'content'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { filePath, content } = context.parameters as unknown as CreateFileParams

    if (!filePath?.trim()) {
      throw new ValidationError('文件路径不能为空')
    }

    try {
      await writeTextFile(filePath, content)

      return {
        content: [
          {
            type: 'text',
            text: `文件已创建: ${filePath}`,
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
