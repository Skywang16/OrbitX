/**
 * 文件写入工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { FileAlreadyExistsError, ValidationError } from './tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface WriteFileParams {
  filePath: string
  content: string
  overwrite?: boolean
}

/**
 * 文件写入工具
 */
export class WriteFileTool extends ModifiableTool {
  constructor() {
    super('write_file', '💾 写入文件：创建新文件或覆盖现有文件', {
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
    const { filePath, content, overwrite = false } = context.parameters as unknown as WriteFileParams

    // 验证文件路径
    if (!filePath || filePath.trim() === '') {
      throw new ValidationError('文件路径不能为空')
    }

    try {
      // 检查文件是否已存在
      try {
        await invoke('plugin:fs|metadata', { path: filePath })
        if (!overwrite) {
          throw new FileAlreadyExistsError(filePath)
        }
      } catch {
        // 文件不存在，可以写入
      }

      // 写入文件
      await invoke('plugin:fs|write_text_file', {
        path: filePath,
        contents: content,
      })

      return {
        content: [
          {
            type: 'text',
            text: `✅ 文件已写入: ${filePath}`,
          },
        ],
      }
    } catch (error) {
      if (error instanceof FileAlreadyExistsError) {
        throw error
      }
      throw new Error(`写入文件失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }
}

// 导出工具实例
export const writeFileTool = new WriteFileTool()
