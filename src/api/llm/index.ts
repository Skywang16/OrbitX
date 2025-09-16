/**
 * LLM 核心 API
 *
 * 提供大语言模型的统一接口，包括：
 * - 模型调用
 * - 流式处理
 * - 模型管理
 */

import { invoke } from '@/utils/request'
import { Channel } from '@tauri-apps/api/core'
import type { NativeLLMRequest, NativeLLMResponse, NativeLLMStreamChunk } from '@/eko-core/types/llm.types'

/**
 * LLM API 接口类
 */
export class LLMApi {
  /**
   * 普通LLM调用
   */
  async call(request: NativeLLMRequest): Promise<NativeLLMResponse> {
    return await invoke<NativeLLMResponse>('llm_call', { request })
  }

  /**
   * 流式LLM调用
   */
  async callStream(request: NativeLLMRequest): Promise<ReadableStream<NativeLLMStreamChunk>> {
    const channel = new Channel<NativeLLMStreamChunk>()

    // Handle abort signal if provided
    if (request.abortSignal) {
      request.abortSignal.addEventListener('abort', () => {
        this.cancelStream().catch(console.warn)
      })
    }

    // Start the streaming call
    invoke('llm_call_stream', {
      request,
      onEvent: channel.onmessage,
    })

    return new ReadableStream({
      start(controller) {
        channel.onmessage = (chunk: NativeLLMStreamChunk) => {
          controller.enqueue(chunk)
          // Close the stream when receiving a finish or error chunk
          if (chunk.type === 'finish' || chunk.type === 'error') {
            controller.close()
          }
        }
      },
    })
  }

  /**
   * 获取可用模型列表
   */
  async getAvailableModels(): Promise<string[]> {
    return await invoke<string[]>('llm_get_available_models')
  }

  /**
   * 测试模型连接
   */
  async testModelConnection(modelId: string): Promise<boolean> {
    return await invoke<boolean>('llm_test_model_connection', { modelId })
  }

  /**
   * 取消流式调用
   */
  async cancelStream(): Promise<void> {
    await invoke('llm_cancel_stream', { requestId: 'current' })
  }
}

export const llmApi = new LLMApi()

// 默认导出
export default llmApi
