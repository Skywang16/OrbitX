/**
 * 文件编辑工具 - 支持精细化编辑功能
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
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
      `Edit file contents by precise string replacement. Searches for all content in the file that matches oldString and replaces it with newString. Replacement is global, must match exactly, and is case-sensitive. It's recommended to use the read_file tool first to check current file content to ensure precise matching. Must use absolute paths.`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description:
              'Absolute path to the file. Must be a complete path, for example: "/Users/user/project/src/main.ts", "/home/user/workspace/config.json"',
          },
          oldString: {
            type: 'string',
            description: 'Original text to be replaced. Example: "const app = createApp()"',
          },
          newString: {
            type: 'string',
            description: 'New text after replacement. Example: "const app = createApp(App)"',
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
            text: `Edit failed: No matching content found in file ${path}.
Status: The specified oldString does not exist in the file.
Suggestion: Use read_file tool to check current file content, ensure oldString exactly matches the text to be replaced.`,
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
          text: `File edited successfully: ${path}
Status: Content has been successfully replaced.
Suggestion: Use read_file tool to verify the changes.`,
          data: editResult,
        } as ExtendedContentItem,
      ],
    }
  }
}

// 导出工具实例
export const editFileTool = new EditFileTool()
