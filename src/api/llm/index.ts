/**
 * LLM 核心 API
 *
 * 提供大语言模型的统一接口，包括：
 * - 模型调用
 * - 流式处理
 * - 模型管理
 */

import { invoke } from '@/utils/request'
import { llmChannelApi } from '@/api/channel/llm'

export interface NativeLLMRequest extends Record<string, unknown> {
  abortSignal?: AbortSignal
}
export type NativeLLMResponse = unknown
export type NativeLLMStreamChunk = { type: string; [key: string]: unknown }

/**
 * LLM API 接口类
 */
export class LLMApi {
  /**
   * 普通LLM调用
   */
  call = async (request: NativeLLMRequest): Promise<NativeLLMResponse> => {
    return await invoke<NativeLLMResponse>('llm_call', { request })
  }

  /**
   * 流式LLM调用
   */
  callStream = async (request: NativeLLMRequest): Promise<ReadableStream<NativeLLMStreamChunk>> => {
    // Handle abort signal if provided
    if (request.abortSignal) {
      request.abortSignal.addEventListener('abort', () => {
        this.cancelStream().catch(console.warn)
      })
    }

    // 使用统一的 Channel API
    return llmChannelApi.createStream({ request })
  }

  /**
   * 获取可用模型列表
   */
  getAvailableModels = async (): Promise<string[]> => {
    return await invoke<string[]>('llm_get_available_models')
  }

  /**
   * 测试模型连接
   */
  testModelConnection = async (modelId: string): Promise<boolean> => {
    return await invoke<boolean>('llm_test_model_connection', { modelId })
  }

  /**
   * 取消流式调用
   */
  cancelStream = async (): Promise<void> => {
    return llmChannelApi.cancelStream()
  }
}

export const llmApi = new LLMApi()

// 默认导出
export default llmApi
