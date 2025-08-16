/**
 * 文件编辑工具 - 支持精细化编辑功能
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { ValidationError } from '../tool-error'
import { writeTextFile, readTextFile, exists } from '@tauri-apps/plugin-fs'

export interface EditFileParams {
  filePath: string
  oldString: string
  newString: string
}

/**
 * 文件编辑工具 - 支持多种编辑模式
 */
export class EditFileTool extends ModifiableTool {
  constructor() {
    super('edit_file', '编辑文件：替换文件中的指定文本内容', {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: '要编辑的文件路径',
        },
        oldString: {
          type: 'string',
          description: '要替换的原始文本',
        },
        newString: {
          type: 'string',
          description: '替换后的新文本',
        },
      },
      required: ['filePath', 'oldString', 'newString'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { filePath, oldString, newString } = context.parameters as unknown as EditFileParams

    if (!filePath?.trim()) {
      throw new ValidationError('文件路径不能为空')
    }

    if (oldString === newString) {
      return {
        content: [{ type: 'text', text: `原始文本和新文本相同，无需修改` }],
      }
    }

    try {
      if (!(await exists(filePath))) {
        throw new ValidationError(`文件不存在: ${filePath}`)
      }

      const originalContent = await readTextFile(filePath)
      const modifiedContent = originalContent.replace(oldString, newString)

      if (modifiedContent === originalContent) {
        return {
          content: [{ type: 'text', text: `未找到匹配的内容，文件未修改` }],
        }
      }

      await writeTextFile(filePath, modifiedContent)

      return {
        content: [{ type: 'text', text: `文件已修改: ${filePath}` }],
      }
    } catch (error) {
      if (error instanceof ValidationError) throw error
      throw new Error(`编辑文件失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }
}

// 导出工具实例
export const editFileTool = new EditFileTool()
