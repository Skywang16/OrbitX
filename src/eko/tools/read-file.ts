/**
 * 文件读取工具 - Eko原生实现
 */

import { invoke } from '@tauri-apps/api/core'
import type { AgentContext } from '@eko-ai/eko'
import type { ToolResult, LanguageModelV2ToolCallPart } from '@eko-ai/eko/types'
import { EkoTool, createParameterSchema } from './base/eko-tool'

/**
 * 文件读取工具
 */
export class ReadFileTool extends EkoTool {
  constructor() {
    const parameters = createParameterSchema()
      .string('path', '文件路径', true)
      .number('startLine', '起始行号', false, 1)
      .number('endLine', '结束行号', false)
      .boolean('showLineNumbers', '显示行号', false, true)
      .build()

    super({
      name: 'read_file',
      description: '读取文件内容',
      parameters,
    })
  }

  protected async executeImpl(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: LanguageModelV2ToolCallPart
  ): Promise<ToolResult> {
    const { path, startLine, endLine, showLineNumbers } = args as {
      path: string
      startLine?: number
      endLine?: number
      showLineNumbers: boolean
    }

    // 验证行号范围
    if (startLine && endLine && startLine > endLine) {
      return this.error('开始行号不能大于结束行号')
    }

    await this.sendCallback(agentContext, toolCall, `读取文件: ${path}`, false)

    try {
      // 检查文件是否存在
      const exists = await this.checkFileExists(path)
      if (!exists) {
        return this.error(`文件不存在: ${path}`)
      }

      // 检查是否为目录
      const isDirectory = await this.checkIsDirectory(path)
      if (isDirectory) {
        return this.error(`路径是目录，无法读取: ${path}`)
      }

      // 读取文件内容
      await this.sendCallback(agentContext, toolCall, '读取内容...', false)
      const content = await this.readFileContent(path)

      // 处理行范围和行号
      const processedContent = this.processContent(content, {
        startLine,
        endLine,
        showLineNumbers,
      })

      // 构建结果
      const lineCount = content.split('\n').length
      const displayRange = startLine || endLine ? { start: startLine || 1, end: endLine || lineCount } : null

      let resultText = `文件读取成功: ${path}\n`

      if (displayRange) {
        resultText += `显示: 第${displayRange.start}-${displayRange.end}行\n`
      }

      resultText += `\n\`\`\`\n${processedContent}\n\`\`\``

      return this.text(resultText)
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return this.error(`读取文件失败: ${errorMessage}`)
    }
  }

  /**
   * 检查文件是否存在
   */
  private async checkFileExists(path: string): Promise<boolean> {
    try {
      return await invoke<boolean>('plugin:fs|exists', { path })
    } catch {
      return false
    }
  }

  /**
   * 检查是否为目录
   */
  private async checkIsDirectory(path: string): Promise<boolean> {
    try {
      const metadata = await invoke<{ isDir: boolean }>('plugin:fs|metadata', { path })
      return metadata.isDir
    } catch {
      return false
    }
  }

  /**
   * 读取文件内容
   */
  private async readFileContent(path: string): Promise<string> {
    const rawContent = await invoke<ArrayBuffer>('plugin:fs|read_text_file', { path })

    if (rawContent === null || rawContent === undefined) {
      throw new Error('文件内容为空或无法读取')
    }

    return new TextDecoder('utf-8').decode(rawContent)
  }

  /**
   * 处理文件内容
   */
  private processContent(
    content: string,
    options: {
      startLine?: number
      endLine?: number
      showLineNumbers: boolean
    }
  ): string {
    const lines = content.split('\n')
    let processedLines = lines

    // 应用行范围过滤
    if (options.startLine || options.endLine) {
      const start = options.startLine ? options.startLine - 1 : 0
      const end = options.endLine ? options.endLine : lines.length
      processedLines = lines.slice(start, end)
    }

    // 添加行号
    if (options.showLineNumbers) {
      const startNum = options.startLine || 1
      processedLines = processedLines.map((line, index) => `${(startNum + index).toString().padStart(4, ' ')}  ${line}`)
    }

    return processedLines.join('\n')
  }
}

// 导出工具实例
export const readFileTool = new ReadFileTool()
