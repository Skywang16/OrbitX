/**
 * 文件创建工具 - Eko原生实现
 */

import { invoke } from '@tauri-apps/api/core'
import type { AgentContext } from '@eko-ai/eko'
import type { ToolResult, LanguageModelV2ToolCallPart } from '@eko-ai/eko/types'
import { EkoTool, createParameterSchema } from './base/eko-tool'

/**
 * 文件创建工具
 */
export class CreateFileTool extends EkoTool {
  constructor() {
    const parameters = createParameterSchema()
      .string('path', '文件路径', true)
      .string('content', '文件内容', false, '')
      .boolean('createDirectories', '创建父目录', false, true)
      .boolean('overwrite', '覆盖已存在文件', false, false)
      .build()

    super({
      name: 'create_file',
      description: '创建文件',
      parameters,
    })
  }

  protected async executeImpl(
    args: Record<string, unknown>,
    agentContext: AgentContext,
    toolCall: LanguageModelV2ToolCallPart
  ): Promise<ToolResult> {
    const { path, content, createDirectories, overwrite } = args as {
      path: string
      content: string
      createDirectories: boolean
      overwrite: boolean
    }

    await this.sendCallback(agentContext, toolCall, `创建文件: ${path}`, false)

    try {
      // 检查文件是否已存在
      const exists = await this.checkFileExists(path)
      if (exists && !overwrite) {
        return this.error(`文件已存在且不允许覆盖: ${path}`)
      }

      // 创建父目录
      if (createDirectories) {
        await this.sendCallback(agentContext, toolCall, '创建目录...', false)
        await this.createParentDirectories(path)
      }

      // 写入文件
      await this.sendCallback(agentContext, toolCall, '写入内容...', false)
      await this.writeFile(path, content)

      // 验证文件创建成功
      const fileInfo = await this.getFileInfo(path)
      if (!fileInfo.exists) {
        throw new Error('文件创建失败，无法验证文件存在')
      }

      const resultText = `文件${exists ? '覆盖' : '创建'}成功: ${path}`

      return this.success(resultText)
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      return this.error(`创建文件失败: ${errorMessage}`)
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
   * 创建父目录
   */
  private async createParentDirectories(filePath: string): Promise<void> {
    const parts = filePath.split('/')
    const dirPath = parts.slice(0, -1).join('/')

    if (dirPath) {
      try {
        await invoke('plugin:fs|create_dir_all', { path: dirPath })
      } catch (error) {
        throw new Error(`无法创建父目录 ${dirPath}: ${error}`)
      }
    }
  }

  /**
   * 写入文件
   */
  private async writeFile(path: string, content: string): Promise<void> {
    try {
      const encoder = new TextEncoder()
      const data = encoder.encode(content)
      await invoke('plugin:fs|write_file', { path, contents: Array.from(data) })
    } catch (error) {
      throw new Error(`写入文件失败: ${error}`)
    }
  }

  /**
   * 检查文件是否存在
   */
  private async getFileInfo(path: string): Promise<{ exists: boolean }> {
    try {
      await invoke('plugin:fs|metadata', { path })
      return { exists: true }
    } catch {
      return { exists: false }
    }
  }
}

// 导出工具实例
export const createFileTool = new CreateFileTool()
