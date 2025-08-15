/**
 * 文件编辑工具 - 支持精细化编辑功能
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { ValidationError } from './tool-error'
import { writeTextFile, readTextFile, exists } from '@tauri-apps/plugin-fs'

export interface EditFileParams {
  filePath: string
  oldString: string
  newString: string
  replaceAll?: boolean
  lineNumber?: number
  insertMode?: 'replace' | 'before' | 'after'
}

/**
 * 文件编辑工具 - 支持多种编辑模式
 */
export class EditFileTool extends ModifiableTool {
  constructor() {
    super('edit_file', '📝 编辑文件：对现有文件进行精确修改', {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: '要编辑的文件路径',
        },
        oldString: {
          type: 'string',
          description: '要替换的原始文本内容',
        },
        newString: {
          type: 'string',
          description: '替换后的新文本内容',
        },
        replaceAll: {
          type: 'boolean',
          description: '是否替换所有匹配项，默认false（只替换第一个）',
          default: false,
        },
        lineNumber: {
          type: 'number',
          description: '指定操作的行号（从1开始），可选',
        },
        insertMode: {
          type: 'string',
          enum: ['replace', 'before', 'after'],
          description: '插入模式：replace替换行，before在行前插入，after在行后插入',
          default: 'replace',
        },
      },
      required: ['filePath', 'oldString', 'newString'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const {
      filePath,
      oldString,
      newString,
      replaceAll = false,
      lineNumber,
      insertMode = 'replace',
    } = context.parameters as unknown as EditFileParams

    if (!filePath?.trim()) {
      throw new ValidationError('文件路径不能为空')
    }

    if (oldString === newString) {
      return {
        content: [{ type: 'text', text: `⚠️ 原始文本和新文本相同，无需修改: ${filePath}` }],
      }
    }

    try {
      if (!(await exists(filePath))) {
        throw new ValidationError(`文件不存在: ${filePath}`)
      }

      const originalContent = await readTextFile(filePath)
      const modifiedContent =
        lineNumber !== undefined
          ? this.editByLineNumber(originalContent, oldString, newString, lineNumber, insertMode)
          : this.editByTextMatch(originalContent, oldString, newString, replaceAll)

      if (modifiedContent === originalContent) {
        return {
          content: [{ type: 'text', text: `⚠️ 未找到匹配的内容，文件未修改: ${filePath}` }],
        }
      }

      await writeTextFile(filePath, modifiedContent)

      const stats = this.calculateEditStats(originalContent, modifiedContent, oldString)

      return {
        content: [{ type: 'text', text: `✅ 文件已修改: ${filePath}\n📊 ${stats}` }],
      }
    } catch (error) {
      if (error instanceof ValidationError) throw error
      throw new Error(`编辑文件失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 基于行号进行编辑
   */
  private editByLineNumber(
    content: string,
    oldString: string,
    newString: string,
    lineNumber: number,
    insertMode: string
  ): string {
    const lines = content.split('\n')
    const index = lineNumber - 1

    if (lineNumber < 1 || lineNumber > lines.length) {
      throw new ValidationError(`行号超出范围: ${lineNumber}（文件共${lines.length}行）`)
    }

    switch (insertMode) {
      case 'replace':
        lines[index] = lines[index].includes(oldString) ? lines[index].replace(oldString, newString) : lines[index]
        break
      case 'before':
        lines.splice(index, 0, newString)
        break
      case 'after':
        lines.splice(index + 1, 0, newString)
        break
    }

    return lines.join('\n')
  }

  private editByTextMatch(content: string, oldString: string, newString: string, replaceAll: boolean): string {
    if (replaceAll) {
      // 使用全局正则表达式替换所有匹配项
      const escapedOldString = oldString.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      return content.replace(new RegExp(escapedOldString, 'g'), newString)
    } else {
      return content.replace(oldString, newString)
    }
  }

  private calculateEditStats(originalContent: string, modifiedContent: string, oldString: string): string {
    const linesDiff = modifiedContent.split('\n').length - originalContent.split('\n').length
    const replacementCount = (
      originalContent.match(new RegExp(oldString.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g')) || []
    ).length

    let stats = `替换了 ${replacementCount} 处内容`
    if (linesDiff !== 0) {
      stats += `，行数变化: ${linesDiff > 0 ? '+' : ''}${linesDiff}`
    }
    return stats
  }
}

// 导出工具实例
export const editFileTool = new EditFileTool()
