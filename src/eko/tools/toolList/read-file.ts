/**
 * 文件读取工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { FileNotFoundError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface ReadFileParams {
  path: string
  startLine?: number
  endLine?: number
}

/**
 * 文件读取工具
 */
export class ReadFileTool extends ModifiableTool {
  constructor() {
    super(
      'read_file',
      `读取文件内容工具。
输入示例: {"filePath": "src/main.ts", "startLine": 10, "endLine": 20}
输出示例: {
  "content": [{
    "type": "text",
    "text": "文件: src/main.ts (第10-20行)\\n\\n10: import { createApp } from 'vue'\\n11: import App from './App.vue'\\n12: \\n13: const app = createApp(App)\\n14: app.mount('#app')"
  }]
}`,
      {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description: '文件路径。示例："src/main.ts"、"package.json"',
          },
          startLine: {
            type: 'number',
            description: '开始行号。示例：10、1',
            minimum: 1,
          },
          endLine: {
            type: 'number',
            description: '结束行号。示例：20、50',
            minimum: 1,
          },
        },
        required: ['filePath'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { path, startLine, endLine } = context.parameters as unknown as ReadFileParams

    // 验证参数
    if (startLine && endLine && startLine > endLine) {
      throw new Error('开始行号不能大于结束行号')
    }

    try {
      // 首先检查文件是否存在
      const exists = await this.checkFileExists(path)
      if (!exists) {
        throw new FileNotFoundError(path)
      }

      // 检查是否为目录
      const isDirectory = await this.checkIsDirectory(path)
      if (isDirectory) {
        throw new Error(`路径 ${path} 是一个目录，请使用 read_directory 工具读取目录内容`)
      }

      // 使用Tauri API读取文件
      const rawContent = await invoke<ArrayBuffer>('plugin:fs|read_text_file', {
        path: path,
      })

      // 确保内容不为空
      if (rawContent === null || rawContent === undefined) {
        throw new Error('文件内容为空或无法读取')
      }

      // 将ArrayBuffer转换为字符串
      const content = new TextDecoder('utf-8').decode(rawContent)

      // 处理文件内容
      const lines = content.split('\n')
      let processedLines = lines

      // 应用行范围过滤
      if (startLine || endLine) {
        const start = startLine ? startLine - 1 : 0
        const end = endLine ? endLine : lines.length
        processedLines = lines.slice(start, end)
      }

      // 添加行号
      const startNum = startLine || 1
      processedLines = processedLines.map((line, index) => `${(startNum + index).toString().padStart(4, ' ')}  ${line}`)

      return {
        content: [
          {
            type: 'text',
            text: processedLines.join('\n'),
          },
        ],
      }
    } catch (error) {
      if (error instanceof FileNotFoundError) {
        throw error
      }
      throw new Error(`读取文件失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private async checkFileExists(path: string): Promise<boolean> {
    try {
      const exists = await invoke<boolean>('plugin:fs|exists', { path })
      return exists
    } catch (error) {
      return false
    }
  }

  private async checkIsDirectory(path: string): Promise<boolean> {
    try {
      const metadata = await invoke<{ isDir: boolean }>('plugin:fs|metadata', { path })
      return metadata.isDir
    } catch {
      return false
    }
  }
}

// 导出工具实例
export const readFileTool = new ReadFileTool()
