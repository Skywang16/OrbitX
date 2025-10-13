import { channelApi } from './index'
import { invoke } from '@/utils/request'

type LLMStreamChunk = { type: string; [key: string]: unknown }

/**
 * LLM 专用 Channel API
 */
class LLMChannelApi {
  /**
   * 创建 LLM 流式调用
   */
  createStream = (request: Record<string, unknown>): ReadableStream<LLMStreamChunk> => {
    return channelApi.createStream<LLMStreamChunk>(
      'llm_call_stream',
      { request },
      {
        cancelCommand: 'llm_cancel_stream',
        shouldClose: (chunk: LLMStreamChunk) => {
          return chunk.type === 'finish' || chunk.type === 'error'
        },
      }
    )
  }

  /**
   * 取消流式调用
   */
  cancelStream = async (requestId = 'current'): Promise<void> => {
    await invoke('llm_cancel_stream', { requestId })
  }
}

export const llmChannelApi = new LLMChannelApi()
