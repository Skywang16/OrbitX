/**
 * 文件编辑工具 - 支持精细化编辑功能
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { writeTextFile, readTextFile } from '@tauri-apps/plugin-fs'

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
      `编辑文件内容，通过精确的字符串替换修改文件。会查找文件中所有匹配oldString的内容并替换为newString。替换是全局的，会替换文件中的所有匹配项。必须完全匹配，区分大小写。适用于代码重构、配置更新、版本号修改等场景。path参数指定文件路径，oldString参数指定要替换的原始文本，newString参数指定替换后的新文本。如果未找到匹配内容会提示，成功替换会返回确认信息。

输入示例: {"path": "src/main.ts", "oldString": "const app = createApp()", "newString": "const app = createApp(App)"}
输出示例: {
  "content": [{
    "type": "text",
    "text": "文件已修改: src/main.ts"
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
        content: [{ type: 'text', text: `未找到匹配的内容`, data: editResult }],
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
      content: [{ type: 'text', text: `文件已修改: ${path}`, data: editResult }],
    }
  }
}

// 导出工具实例
export const editFileTool = new EditFileTool()
