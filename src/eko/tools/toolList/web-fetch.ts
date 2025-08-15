/**
 * 网络请求工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { NetworkError, ValidationError } from '../tool-error'
import { invoke } from '@tauri-apps/api/core'

export interface WebFetchParams {
  url: string
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS'
  headers?: Record<string, string>
  body?: string
  timeout?: number
  followRedirects?: boolean
  maxRedirects?: number
  responseFormat?: 'text' | 'json' | 'blob' | 'arrayBuffer'
  corsMode?: 'cors' | 'no-cors' | 'same-origin'
  useProxy?: boolean
  extractContent?: boolean
  maxContentLength?: number
  // 智能内容提取参数（简化版）
  useJinaReader?: boolean // 是否优先使用Jina.ai Reader，默认true
}

export interface WebFetchResponse {
  status: number
  statusText: string
  headers: Record<string, string>
  data: unknown
  responseTime: number
  finalUrl: string
}

/**
 * 网络请求工具
 */
export class WebFetchTool extends ModifiableTool {
  constructor() {
    super(
      'web_fetch',
      '🌐 网络请求：发送HTTP请求获取网络资源，支持各种HTTP方法、自定义头部、请求体。用于API调用、数据获取等',
      {
        type: 'object',
        properties: {
          url: {
            type: 'string',
            description: '请求的URL地址',
            format: 'uri',
          },
          method: {
            type: 'string',
            enum: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'],
            description: 'HTTP方法，默认GET',
            default: 'GET',
          },
          headers: {
            type: 'object',
            description: '请求头部',
            additionalProperties: { type: 'string' },
          },
          body: {
            type: 'string',
            description: '请求体（用于POST、PUT等方法）',
          },
          timeout: {
            type: 'number',
            description: '超时时间（毫秒），默认10秒',
            default: 10000,
            minimum: 1000,
            maximum: 60000,
          },
          followRedirects: {
            type: 'boolean',
            description: '是否跟随重定向，默认true',
            default: true,
          },
          maxRedirects: {
            type: 'number',
            description: '最大重定向次数，默认5',
            default: 5,
            minimum: 0,
            maximum: 20,
          },
          responseFormat: {
            type: 'string',
            enum: ['text', 'json', 'blob', 'arrayBuffer'],
            description: '响应数据格式，默认text',
            default: 'text',
          },
          corsMode: {
            type: 'string',
            enum: ['cors', 'no-cors', 'same-origin'],
            description: 'CORS模式，默认cors',
            default: 'cors',
          },
          useProxy: {
            type: 'boolean',
            description: '是否使用代理服务器绕过CORS限制，默认false',
            default: false,
          },
          useJinaReader: {
            type: 'boolean',
            description: '是否优先使用Jina.ai Reader进行智能内容提取，默认true',
            default: true,
          },
        },
        required: ['url'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const {
      url,
      method = 'GET',
      headers = {},
      body,
      timeout = 10000,
      followRedirects = true,
      responseFormat = 'text',
      corsMode = 'cors',
      useProxy = false,
      extractContent = true,
      maxContentLength = 2000,
      useJinaReader = true,
    } = context.parameters as unknown as WebFetchParams

    console.log('🔍 [WebFetch] 开始执行网络请求')
    console.log('📋 [WebFetch] 请求参数:', {
      url,
      method,
      headers,
      body: body ? `${body.substring(0, 100)}${body.length > 100 ? '...' : ''}` : undefined,
      timeout,
      followRedirects,
      responseFormat,
      corsMode,
      useProxy,
    })

    // 验证URL
    console.log('✅ [WebFetch] 开始验证URL:', url)
    this.validateUrl(url)
    console.log('✅ [WebFetch] URL验证通过')

    // 验证方法和请求体
    if (body && !['POST', 'PUT', 'PATCH'].includes(method)) {
      throw new ValidationError(`HTTP方法 ${method} 不支持请求体`)
    }

    // 优先尝试使用 Jina.ai Reader 进行智能内容提取
    if (useJinaReader && method === 'GET' && extractContent) {
      console.log('🤖 [WebFetch] 尝试使用 Jina.ai Reader 进行智能内容提取')
      try {
        const jinaResult = await this.tryJinaReader(url, timeout)
        if (jinaResult) {
          console.log('✅ [WebFetch] Jina.ai Reader 提取成功')
          return jinaResult
        }
      } catch (error) {
        console.warn('⚠️ [WebFetch] Jina.ai Reader 失败，回退到本地算法:', error)
      }
    }

    // 回退到本地 Tauri 后端进行无头请求
    console.log('🚀 [WebFetch] 使用本地 Tauri 后端进行无头请求')
    try {
      const tauriResponse = await this.executeWithTauri({
        url,
        method,
        headers,
        body,
        timeout,
        followRedirects,
        responseFormat,
        extractContent,
        maxContentLength,
      })

      if (tauriResponse.success) {
        console.log('✅ [WebFetch] 请求成功')
        return this.formatTauriResponse(tauriResponse, url, method)
      } else {
        console.error('❌ [WebFetch] 请求失败:', tauriResponse.error)
        throw new NetworkError(tauriResponse.error || '请求失败')
      }
    } catch (error) {
      console.error('❌ [WebFetch] 请求执行失败:', error)
      if (error instanceof NetworkError) {
        throw error
      }
      throw new NetworkError(`请求失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 使用 Tauri 后端执行无头请求
   */
  private async executeWithTauri(params: {
    url: string
    method: string
    headers: Record<string, string>
    body?: string
    timeout: number
    followRedirects: boolean
    responseFormat: string
    extractContent: boolean
    maxContentLength: number
  }) {
    interface TauriWebFetchResponse {
      status: number
      status_text: string
      headers: Record<string, string>
      data: string
      response_time: number
      final_url: string
      success: boolean
      error?: string
    }

    const response = await invoke<TauriWebFetchResponse>('web_fetch_headless', {
      request: {
        url: params.url,
        method: params.method,
        headers: params.headers,
        body: params.body,
        timeout: params.timeout,
        follow_redirects: params.followRedirects,
        response_format: params.responseFormat,
        extract_content: params.extractContent,
        max_content_length: params.maxContentLength,
      },
    })

    return response
  }

  /**
   * 格式化 Tauri 响应
   */
  private formatTauriResponse(
    response: {
      status: number
      status_text: string
      headers: Record<string, string>
      data: string
      response_time: number
      final_url: string
      success: boolean
      error?: string
      content_type?: string
      content_length?: number
      extracted_text?: string
      page_title?: string
    },
    originalUrl: string,
    method: string
  ): ToolResult {
    let resultText = `🌐 网络请求结果 (智能提取):\n\n`
    resultText += `📡 ${method} ${originalUrl}\n`
    resultText += `📊 状态: ${response.status} ${response.status_text}\n`
    resultText += `⏱️ 响应时间: ${response.response_time}ms\n`

    if (response.final_url !== originalUrl) {
      resultText += `🔗 最终URL: ${response.final_url}\n`
    }

    if (response.content_type) {
      resultText += `📄 内容类型: ${response.content_type}\n`
    }

    if (response.content_length) {
      resultText += `📏 内容大小: ${response.content_length} 字符\n`
    }

    if (response.page_title) {
      resultText += `📰 页面标题: ${response.page_title}\n`
    }

    // 只显示关键的响应头
    const importantHeaders = ['content-type', 'content-length', 'server', 'date']
    const filteredHeaders = Object.entries(response.headers).filter(([key]) =>
      importantHeaders.includes(key.toLowerCase())
    )

    if (filteredHeaders.length > 0) {
      resultText += `\n📋 关键响应头:\n`
      for (const [key, value] of filteredHeaders) {
        resultText += `  ${key}: ${value}\n`
      }
    }

    resultText += `\n📄 提取的内容:\n`
    resultText += response.data

    return {
      content: [
        {
          type: 'text',
          text: resultText,
        },
      ],
    }
  }

  private validateUrl(url: string): void {
    console.log('🔍 [WebFetch] 开始URL验证:', url)

    if (!url || url.trim() === '') {
      console.error('❌ [WebFetch] URL为空')
      throw new ValidationError('URL不能为空')
    }

    try {
      const urlObj = new URL(url)
      console.log('📋 [WebFetch] URL解析结果:', {
        protocol: urlObj.protocol,
        hostname: urlObj.hostname,
        port: urlObj.port,
        pathname: urlObj.pathname,
      })

      // 检查协议
      if (!['http:', 'https:'].includes(urlObj.protocol)) {
        console.error('❌ [WebFetch] 不支持的协议:', urlObj.protocol)
        throw new ValidationError('只支持HTTP和HTTPS协议')
      }

      // 检查是否为本地地址（安全考虑）
      const hostname = urlObj.hostname.toLowerCase()
      console.log('🔍 [WebFetch] 检查主机名:', hostname)

      if (
        hostname === 'localhost' ||
        hostname === '127.0.0.1' ||
        hostname.startsWith('192.168.') ||
        hostname.startsWith('10.') ||
        hostname.match(/^172\.(1[6-9]|2[0-9]|3[0-1])\./)
      ) {
        console.error('❌ [WebFetch] 检测到本地地址:', hostname)
        throw new ValidationError('不允许访问本地地址')
      }

      console.log('✅ [WebFetch] URL验证通过')
    } catch (error) {
      if (error instanceof ValidationError) {
        console.error('❌ [WebFetch] 验证错误:', error.message)
        throw error
      }
      console.error('❌ [WebFetch] URL格式错误:', error)
      throw new ValidationError(`无效的URL格式: ${url}`)
    }
  }

  /**
   * 尝试使用 Jina.ai Reader 进行智能内容提取
   */
  private async tryJinaReader(url: string, timeout: number): Promise<ToolResult | null> {
    const startTime = Date.now()

    // 构建 Jina.ai Reader URL
    const jinaUrl = `https://r.jina.ai/${url}`

    try {
      // 使用 fetch 调用 Jina.ai Reader
      const controller = new AbortController()
      const timeoutId = setTimeout(() => controller.abort(), timeout)

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
        console.warn(`Jina.ai Reader 返回错误状态: ${response.status}`)
        return null
      }

      const content = await response.text()
      const responseTime = Date.now() - startTime

      // 检查内容是否有效
      if (!content || content.trim().length < 50) {
        console.warn('Jina.ai Reader 返回的内容太短，可能提取失败')
        return null
      }

      // 格式化返回结果
      let resultText = `🌐 网络请求结果 (Jina.ai Reader 智能提取):\n\n`
      resultText += `📡 GET ${url}\n`
      resultText += `📊 状态: 200 OK\n`
      resultText += `⏱️ 响应时间: ${responseTime}ms\n`
      resultText += `🤖 提取方式: Jina.ai Reader\n`
      resultText += `\n📄 智能提取的内容:\n`
      resultText += content

      return {
        content: [
          {
            type: 'text',
            text: resultText,
          },
        ],
      }
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') {
        console.warn('Jina.ai Reader 请求超时')
      } else {
        console.warn('Jina.ai Reader 请求失败:', error)
      }
      return null
    }
  }
}

// 导出工具实例
export const webFetchTool = new WebFetchTool()
