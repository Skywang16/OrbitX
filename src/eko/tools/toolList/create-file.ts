/**
 * 文件创建工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { ValidationError, ToolError } from '../tool-error'
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
      `Create a new file and write content to it. Supports creating any type of file, including code files, configuration files, documents, etc. Automatically creates non-existent parent directories. Will directly overwrite if the file already exists. Must use absolute paths.`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description:
              'Absolute path to the file. Must be a complete path, for example: "/Users/user/project/src/utils.ts", "/home/user/workspace/config.json"',
          },
          content: {
            type: 'string',
            description: 'File content. Example: "export function hello() { return \'Hello\' }"',
          },
        },
        required: ['path', 'content'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path, content } = context.parameters as unknown as CreateFileParams

    if (!path?.trim()) {
      throw new ValidationError('File path cannot be empty')
    }

    try {
      await writeTextFile(path, content)

      return {
        content: [
          {
            type: 'text',
            text: `File created successfully: ${path}
Status: New file has been successfully created.
Suggestion: Use read_file tool to verify file content, or use edit_file tool for further modifications.`,
          },
        ],
      }
    } catch (error) {
      throw new ToolError(`Failed to create file: ${error instanceof Error ? error.message : String(error)}`)
    }
  }
}

// 导出工具实例
export const createFileTool = new CreateFileTool()
