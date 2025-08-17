/**
 * 文件创建工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { ValidationError } from '../tool-error'
import { writeTextFile } from '@tauri-apps/plugin-fs'

export interface CreateFileParams {
  path: string
  content: string
}

/**
 * 文件创建工具
 */
export class CreateFileTool extends ModifiableTool {
  constructor() {
    super(
      'create_file',
      `创建新文件并写入内容。支持创建任意类型的文件，包括代码文件、配置文件、文档等。会自动创建不存在的父目录。如果文件已存在会直接覆盖。适用于生成新的源码文件、配置文件、文档等场景。path参数指定文件路径（支持相对和绝对路径），content参数指定文件内容（支持多行文本，使用\\n表示换行）。创建成功后返回确认信息。

输入示例: {"path": "src/utils.ts", "content": "export function hello() {\\n  return 'Hello World'\\n}"}
输出示例: {
  "content": [{
    "type": "text",
    "text": "文件已创建: src/utils.ts"
  }]
}`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description: '文件路径。示例："src/utils.ts"、"config.json"',
          },
          content: {
            type: 'string',
            description: '文件内容。示例："export function hello() { return \'Hello\' }"',
          },
        },
        required: ['path', 'content'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path, content } = context.parameters as unknown as CreateFileParams

    if (!path?.trim()) {
      throw new ValidationError('文件路径不能为空')
    }

    try {
      await writeTextFile(path, content)

      return {
        content: [
          {
            type: 'text',
            text: `文件已创建: ${path}`,
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
