/**
 * LLM提供商接入层
 *
 * 封装现有的AI API，提供统一的LLM调用接口
 */

import type { LLMProvider, LLMResponse, LLMStreamChunk, LLMCallOptions } from '../types/llm'
import { aiAPI } from '../../api/ai'
import type { AIResponse, StreamCallback } from '../../types'

/**
 * 默认LLM提供商实现
 */
export class DefaultLLMProvider implements LLMProvider {
  public readonly name = 'default'

  /**
   * 调用LLM（非流式）
   */
  async call(prompt: string, options?: LLMCallOptions): Promise<LLMResponse> {
    try {
      // 构建完整的提示词
      const fullPrompt = this.buildPrompt(prompt, options)

      // 调用AI API
      const response: AIResponse = await aiAPI.sendChatMessage(fullPrompt, options?.model)

      // 直接返回响应，无转换
      return {
        content: response.content,
        finishReason: 'stop',
        usage: response.metadata?.tokensUsed
          ? {
              promptTokens: 0,
              completionTokens: response.metadata.tokensUsed as number,
              totalTokens: response.metadata.tokensUsed as number,
            }
          : undefined,
        metadata: response.metadata,
      }
    } catch (error) {
      throw new Error(`LLM call failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 流式调用LLM
   */
  async *stream(prompt: string, options?: LLMCallOptions): AsyncIterable<LLMStreamChunk> {
    const fullPrompt = this.buildPrompt(prompt, options)

    let buffer = ''
    let isComplete = false
    let error: Error | null = null

    // 创建流式回调
    const callback: StreamCallback = (chunk: any) => {
      try {
        if (chunk?.type === 'content') {
          buffer += chunk.content || ''
        } else if (chunk?.type === 'done') {
          isComplete = true
        } else if (chunk?.type === 'error') {
          error = new Error(String(chunk.error || '流式处理错误'))
          isComplete = true
        }
      } catch (e) {
        error = new Error('流式处理异常')
        isComplete = true
      }
    }

    // 启动流式调用
    const streamPromise = aiAPI.streamChatMessageWithChannel(fullPrompt, callback, options?.model)

    // 生成流式响应
    while (!isComplete) {
      if (error) {
        throw error
      }

      if (buffer.length > 0) {
        const content = buffer
        buffer = ''
        yield {
          content,
          metadata: {
            model: options?.model,
            timestamp: Date.now(),
          },
        }
      }

      // 短暂等待，避免忙等待
      await new Promise(resolve => setTimeout(resolve, 10))
    }

    // 等待流式调用完成
    try {
      await streamPromise
    } catch (streamError) {
      throw new Error(`Stream failed: ${streamError instanceof Error ? streamError.message : String(streamError)}`)
    }

    // 发送最后的内容（如果有）
    if (buffer.length > 0) {
      yield {
        content: buffer,
        finishReason: 'stop',
        metadata: {
          model: options?.model,
          timestamp: Date.now(),
        },
      }
    }
  }

  /**
   * 检查LLM是否可用
   */
  async isAvailable(): Promise<boolean> {
    try {
      // 尝试获取AI模型列表来检查可用性
      const models = await aiAPI.getModels()
      return models.length > 0
    } catch {
      return false
    }
  }

  /**
   * 构建完整的提示词
   */
  private buildPrompt(prompt: string, options?: LLMCallOptions): string {
    let fullPrompt = ''

    // 添加系统提示词
    if (options?.systemPrompt) {
      fullPrompt += `System: ${options.systemPrompt}\n\n`
    }

    // 直接添加工具信息，无格式化
    if (options?.tools && options.tools.length > 0) {
      fullPrompt += '可用工具:\n'
      options.tools.forEach(tool => {
        fullPrompt += `- ${tool.name}: ${tool.description}\n`
      })
      fullPrompt += '\n'
    }

    // 添加主要提示词
    fullPrompt += prompt

    // 如果有工具，添加工具调用指令
    if (options?.tools && options.tools.length > 0) {
      fullPrompt += '\n\n请根据需要调用合适的工具，并以JSON格式返回工具调用信息。'
    }

    return fullPrompt
  }
}

/**
 * LLM管理器
 */
export class LLMManager {
  private _providers: Map<string, LLMProvider> = new Map()
  private _defaultProvider: string = 'default'

  constructor() {
    // 注册默认提供商
    this.registerProvider(new DefaultLLMProvider())
  }

  /**
   * 注册LLM提供商
   */
  registerProvider(provider: LLMProvider): void {
    this._providers.set(provider.name, provider)
  }

  /**
   * 获取LLM提供商
   */
  getProvider(name?: string): LLMProvider | undefined {
    return this._providers.get(name || this._defaultProvider)
  }

  /**
   * 设置默认提供商
   */
  setDefaultProvider(name: string): void {
    if (!this._providers.has(name)) {
      throw new Error(`Provider '${name}' not found`)
    }
    this._defaultProvider = name
  }

  /**
   * 获取所有提供商名称
   */
  getProviderNames(): string[] {
    return Array.from(this._providers.keys())
  }

  /**
   * 调用LLM
   */
  async call(prompt: string, options?: LLMCallOptions & { provider?: string }): Promise<LLMResponse> {
    const provider = this.getProvider(options?.provider)
    if (!provider) {
      throw new Error(`Provider '${options?.provider || this._defaultProvider}' not found`)
    }

    return provider.call(prompt, options)
  }

  /**
   * 流式调用LLM
   */
  async *stream(prompt: string, options?: LLMCallOptions & { provider?: string }): AsyncIterable<LLMStreamChunk> {
    const provider = this.getProvider(options?.provider)
    if (!provider) {
      throw new Error(`Provider '${options?.provider || this._defaultProvider}' not found`)
    }

    yield* provider.stream(prompt, options)
  }

  /**
   * 检查提供商是否可用
   */
  async isProviderAvailable(name?: string): Promise<boolean> {
    const provider = this.getProvider(name)
    if (!provider) {
      return false
    }

    return provider.isAvailable()
  }

  /**
   * 获取可用的提供商
   */
  async getAvailableProviders(): Promise<string[]> {
    const available: string[] = []

    for (const [name, provider] of this._providers.entries()) {
      try {
        if (await provider.isAvailable()) {
          available.push(name)
        }
      } catch {
        // 忽略检查失败的提供商
      }
    }

    return available
  }
}

// 导出单例实例
export const llmManager = new LLMManager()
