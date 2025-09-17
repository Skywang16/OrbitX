import { channelApi } from './index'
import type { NativeLLMStreamChunk } from '@/eko-core/types/llm.types'
import { invoke } from '@/utils/request'

/**
 * LLM 专用 Channel API
 */
class LLMChannelApi {
  /**
   * 创建 LLM 流式调用
   */
  createStream(request: Record<string, unknown>): ReadableStream<NativeLLMStreamChunk> {
    return channelApi.createStream<NativeLLMStreamChunk>(
      'llm_call_stream',
      { request },
      {
        cancelCommand: 'llm_cancel_stream',
        shouldClose: (chunk: NativeLLMStreamChunk) => {
          return chunk.type === 'finish' || chunk.type === 'error'
        },
      }
    )
  }

  /**
   * 取消流式调用
   */
  async cancelStream(requestId = 'current'): Promise<void> {
    await invoke('llm_cancel_stream', { requestId })
  }
}

export const llmChannelApi = new LLMChannelApi()
