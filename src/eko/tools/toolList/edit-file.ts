/**
 * 文件编辑工具 - 支持精细化编辑功能
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@eko-ai/eko/types'
import { writeTextFile, readTextFile } from '@tauri-apps/plugin-fs'

// 扩展的内容项类型，支持 data 字段
interface ExtendedContentItem {
  type: 'text'
  text: string
  data?: SimpleEditResult
}

export interface EditFileParams {
  path: string
  oldString: string
  newString: string
}

export interface SimpleEditResult {
  file: string // 文件路径
  success: boolean // 是否成功
  old: string // 原始内容片段
  new: string // 新内容片段
}

/**
 * 文件编辑工具 - 支持多种编辑模式
 */
export class EditFileTool extends ModifiableTool {
  constructor() {
    super(
      'edit_file',
      `编辑文件内容，通过精确的字符串替换修改文件。会查找文件中所有匹配oldString的内容并替换为newString。替换是全局的，必须完全匹配，区分大小写。建议先使用read_file工具检查当前文件内容以确保精确匹配。必须使用绝对路径。`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description:
              '文件的绝对路径。必须是完整路径，例如："/Users/user/project/src/main.ts"、"/home/user/workspace/config.json"',
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
        required: ['path', 'oldString', 'newString'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path, oldString, newString } = context.parameters as unknown as EditFileParams

    const originalContent = await readTextFile(path)
    const modifiedContent = originalContent.replace(oldString, newString)

    if (modifiedContent === originalContent) {
      const editResult: SimpleEditResult = {
        file: path,
        success: false,
        old: oldString,
        new: newString,
      }
      return {
        content: [
          {
            type: 'text',
            text: `编辑失败：在文件 ${path} 中未找到匹配的内容。
状态：指定的 oldString 在文件中不存在。
建议：使用 read_file 工具检查当前文件内容，确保 oldString 与要替换的文本完全匹配。`,
            data: editResult,
          } as ExtendedContentItem,
        ],
      }
    }

    await writeTextFile(path, modifiedContent)

    const editResult: SimpleEditResult = {
      file: path,
      success: true,
      old: oldString,
      new: newString,
    }

    return {
      content: [
        {
          type: 'text',
          text: `文件编辑成功: ${path}
状态：内容已成功替换。
建议：使用 read_file 工具验证更改结果。`,
          data: editResult,
        } as ExtendedContentItem,
      ],
    }
  }
}

// 导出工具实例
export const editFileTool = new EditFileTool()
