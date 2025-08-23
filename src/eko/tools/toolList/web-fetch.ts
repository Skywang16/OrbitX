/**
 * 网络请求工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@eko-ai/eko/types'
import { NetworkError, ValidationError } from '../tool-error'
import { aiApi } from '@/api'

export interface WebFetchParams {
  url: string
}

// WebFetchResponse 类型已在 @/api/ai/tool 中定义

/**
 * 网络请求工具
 */
export class WebFetchTool extends ModifiableTool {
  constructor() {
    super(
      'web_fetch',
      `获取网页内容和API数据。使用GET方法获取网页的主要文本内容，自动提取并清理内容。适用于获取API数据、网页内容、文档、技术文章等场景。返回清理后的网页内容或API响应数据。`,
      {
        type: 'object',
        properties: {
          url: {
            type: 'string',
            description: 'URL地址。示例："https://api.github.com/users/octocat"、"https://docs.python.org/3/tutorial/"',
          },
        },
        required: ['url'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { url } = context.parameters as unknown as WebFetchParams

    // 验证URL
    this.validateUrl(url)

    // 优先尝试使用 Jina.ai Reader 进行智能内容提取
    try {
      const jinaResult = await this.tryJinaReader(url)
      if (jinaResult) {
        return jinaResult
      }
    } catch (error) {
      // Jina.ai Reader 失败，回退到本地算法
    }

    // 回退到本地 Tauri 后端进行请求
    try {
      const tauriResponse = await this.executeWithTauri({ url })

      if (tauriResponse.success) {
        return this.formatTauriResponse(tauriResponse)
      } else {
        throw new NetworkError(tauriResponse.error || '请求失败')
      }
    } catch (error) {
      if (error instanceof NetworkError) {
        throw error
      }
      throw new NetworkError(`请求失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 使用 Tauri 后端执行请求
   */
  private async executeWithTauri(params: { url: string }) {
    const response = await aiApi.webFetch({
      url: params.url,
      method: 'GET',
      headers: {},
      timeout: 10000,
    })

    return response
  }

  /**
   * 格式化响应结果
   */
  private formatTauriResponse(response: {
    status: number
    status_text: string
    data: string
    success: boolean
    error?: string
    page_title?: string
  }): ToolResult {
    if (response.page_title) {
      return {
        content: [
          {
            type: 'text',
            text: `${response.page_title}\n\n${response.data}`,
          },
        ],
      }
    }

    return {
      content: [
        {
          type: 'text',
          text: response.data,
        },
      ],
    }
  }

  private validateUrl(url: string): void {
    if (!url || url.trim() === '') {
      throw new ValidationError('URL不能为空')
    }

    try {
      const urlObj = new URL(url)

      // 检查协议
      if (!['http:', 'https:'].includes(urlObj.protocol)) {
        throw new ValidationError('只支持HTTP和HTTPS协议')
      }

      // 检查是否为本地地址（安全考虑）
      const hostname = urlObj.hostname.toLowerCase()

      if (
        hostname === 'localhost' ||
        hostname === '127.0.0.1' ||
        hostname.startsWith('192.168.') ||
        hostname.startsWith('10.') ||
        hostname.match(/^172\.(1[6-9]|2[0-9]|3[0-1])\./)
      ) {
        throw new ValidationError('不允许访问本地地址')
      }
    } catch (error) {
      if (error instanceof ValidationError) {
        throw error
      }
      throw new ValidationError(`无效的URL格式: ${url}`)
    }
  }

  /**
   * 尝试使用 Jina.ai Reader 进行智能内容提取
   */
  private async tryJinaReader(url: string): Promise<ToolResult | null> {
    const jinaUrl = `https://r.jina.ai/${url}`

    try {
      const controller = new AbortController()
      const timeoutId = setTimeout(() => controller.abort(), 10000)

      const response = await fetch(jinaUrl, {
        method: 'GET',
        headers: {
          'User-Agent': 'Eko-Agent/1.0',
          Accept: 'text/plain, text/markdown, */*',
        },
        signal: controller.signal,
      })

      clearTimeout(timeoutId)

      if (!response.ok) {
        return null
      }

      const content = await response.text()

      // 检查内容是否有效
      if (!content || content.trim().length < 50) {
        return null
      }

      return {
        content: [
          {
            type: 'text',
            text: content,
          },
        ],
      }
    } catch (error) {
      return null
    }
  }
}

// 导出工具实例
export const webFetchTool = new WebFetchTool()
