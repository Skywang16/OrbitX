/**
 * 网络请求工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { NetworkError, ValidationError } from './tool-error'

export interface WebFetchParams {
  url: string
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS'
  headers?: Record<string, string>
  body?: string
  timeout?: number
  followRedirects?: boolean
  maxRedirects?: number
  responseFormat?: 'text' | 'json' | 'blob' | 'arrayBuffer'
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
    } = context.parameters as unknown as WebFetchParams

    // 验证URL
    this.validateUrl(url)

    // 验证方法和请求体
    if (body && !['POST', 'PUT', 'PATCH'].includes(method)) {
      throw new ValidationError(`HTTP方法 ${method} 不支持请求体`)
    }

    const startTime = Date.now()

    try {
      // 构建请求选项
      const requestOptions: RequestInit = {
        method,
        headers: {
          'User-Agent': 'Eko-Agent/1.0',
          ...headers,
        },
        redirect: followRedirects ? 'follow' : 'manual',
        signal: AbortSignal.timeout(timeout),
      }

      // 添加请求体
      if (body) {
        requestOptions.body = body

        // 自动设置Content-Type如果没有指定
        if (!headers['Content-Type'] && !headers['content-type']) {
          try {
            JSON.parse(body)
            requestOptions.headers = {
              ...requestOptions.headers,
              'Content-Type': 'application/json',
            }
          } catch {
            requestOptions.headers = {
              ...requestOptions.headers,
              'Content-Type': 'text/plain',
            }
          }
        }
      }

      // 发送请求
      const response = await fetch(url, requestOptions)
      const responseTime = Date.now() - startTime

      // 获取响应头
      const responseHeaders: Record<string, string> = {}
      response.headers.forEach((value, key) => {
        responseHeaders[key] = value
      })

      // 获取响应数据
      let data: unknown
      try {
        switch (responseFormat) {
          case 'json':
            data = await response.json()
            break
          case 'blob':
            data = `[Blob: ${(await response.blob()).size} bytes]`
            break
          case 'arrayBuffer':
            data = `[ArrayBuffer: ${(await response.arrayBuffer()).byteLength} bytes]`
            break
          default:
            data = await response.text()
        }
      } catch (error) {
        throw new NetworkError(`解析响应数据失败: ${error instanceof Error ? error.message : String(error)}`)
      }

      const result: WebFetchResponse = {
        status: response.status,
        statusText: response.statusText,
        headers: responseHeaders,
        data,
        responseTime,
        finalUrl: response.url,
      }

      // 格式化输出
      let resultText = `🌐 网络请求结果:\n\n`
      resultText += `📡 ${method} ${url}\n`
      resultText += `📊 状态: ${result.status} ${result.statusText}\n`
      resultText += `⏱️ 响应时间: ${result.responseTime}ms\n`

      if (result.finalUrl !== url) {
        resultText += `🔗 最终URL: ${result.finalUrl}\n`
      }

      resultText += `\n📋 响应头:\n`
      for (const [key, value] of Object.entries(result.headers)) {
        resultText += `  ${key}: ${value}\n`
      }

      resultText += `\n📄 响应内容:\n`
      if (typeof result.data === 'string') {
        const content = result.data
        if (content.length > 2000) {
          resultText += content.substring(0, 2000) + '\n\n... (内容被截断，总长度: ' + content.length + ' 字符)'
        } else {
          resultText += content
        }
      } else {
        resultText += JSON.stringify(result.data, null, 2)
      }

      return {
        content: [
          {
            type: 'text',
            text: resultText,
          },
        ],
      }
    } catch (error) {
      if (error instanceof DOMException && error.name === 'TimeoutError') {
        throw new NetworkError(`请求超时 (${timeout}ms)`)
      }

      if (error instanceof DOMException && error.name === 'AbortError') {
        throw new NetworkError('请求被中止')
      }

      if (error instanceof TypeError) {
        throw new NetworkError(`网络连接失败: ${error.message}`)
      }

      if (error instanceof NetworkError || error instanceof ValidationError) {
        throw error
      }

      throw new NetworkError(`请求失败: ${error instanceof Error ? error.message : String(error)}`)
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
}

// 导出工具实例
export const webFetchTool = new WebFetchTool()
