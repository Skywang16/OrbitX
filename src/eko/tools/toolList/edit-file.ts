/**
 * 文件编辑工具 - 支持精细化编辑功能
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { ValidationError } from '../tool-error'
import { writeTextFile, readTextFile, exists } from '@tauri-apps/plugin-fs'

export interface EditFileParams {
  path: string
  oldString: string
  newString: string
}

/**
 * 文件编辑工具 - 支持多种编辑模式
 */
export class EditFileTool extends ModifiableTool {
  constructor() {
    super(
      'edit_file',
      `编辑文件工具。
输入示例: {"filePath": "src/main.ts", "oldString": "const app = createApp()", "newString": "const app = createApp(App)"}
输出示例: {
  "content": [{
    "type": "text",
    "text": "文件已修改: src/main.ts\\n替换了 1 处匹配项"
  }]
}`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description: '文件路径。示例："src/main.ts"',
          },
          oldString: {
            type: 'string',
            description: '要替换的原始文本。示例："const app = createApp()"',
          },
          newString: {
            type: 'string',
            description: '替换后的新文本。示例："const app = createApp(App)"',
          },
        },
        required: ['filePath', 'oldString', 'newString'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path, oldString, newString } = context.parameters as unknown as EditFileParams

    if (!path?.trim()) {
      throw new ValidationError('文件路径不能为空')
    }

    if (oldString === newString) {
      return {
        content: [{ type: 'text', text: `原始文本和新文本相同，无需修改` }],
      }
    }

    try {
      if (!(await exists(path))) {
        throw new ValidationError(`文件不存在: ${path}`)
      }

      const originalContent = await readTextFile(path)
      const modifiedContent = originalContent.replace(oldString, newString)

      if (modifiedContent === originalContent) {
        return {
          content: [{ type: 'text', text: `未找到匹配的内容，文件未修改` }],
        }
      }

      await writeTextFile(path, modifiedContent)

      return {
        content: [{ type: 'text', text: `文件已修改: ${path}` }],
      }
    } catch (error) {
      if (error instanceof ValidationError) throw error
      throw new Error(`编辑文件失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }
}

// 导出工具实例
export const editFileTool = new EditFileTool()
