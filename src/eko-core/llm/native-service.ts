import { llmApi } from '@/api'
import { Channel, invoke } from '@tauri-apps/api/core'
import { NativeLLMRequest, NativeLLMResponse, NativeLLMStreamChunk } from '../types/llm.types'
import { LLMError, ErrorHandler } from '../common/error'

/**
 * Native LLM Service - Direct interface to Tauri backend
 * Completely replaces ai-sdk dependencies
 */
export class NativeLLMService {
  /**
   * Make a non-streaming LLM call
   */
  async call(request: NativeLLMRequest): Promise<NativeLLMResponse> {
    try {
      this.validateRequest(request)
      const response = await llmApi.call(request)
      return response
    } catch (error) {
      throw this.handleError(error)
    }
  }

  /**
   * Make a streaming LLM call with optimized processing
   */
  async callStream(request: NativeLLMRequest): Promise<ReadableStream<NativeLLMStreamChunk>> {
    try {
      this.validateRequest(request)
      const channel = new Channel<NativeLLMStreamChunk>()

      // Handle abort signal if provided
      if (request.abortSignal) {
        request.abortSignal.addEventListener('abort', () => {
          this.cancelStream().catch(console.warn)
        })
      }

      // Start the streaming call - invoke returns immediately, stream data comes through channel
      invoke('llm_call_stream', {
        request: { ...request, stream: true },
        onChunk: channel,
      }).catch((error: unknown) => {
        // Handle invoke errors - the channel will receive error through normal message flow
        console.error('LLM stream invoke error:', error)
      })

      const rawStream = this.createStreamFromChannel(channel)

      return rawStream
    } catch (error) {
      throw this.handleError(error)
    }
  }

  /**
   * Get available models from backend
   */
  async getAvailableModels(): Promise<string[]> {
    try {
      return await llmApi.getAvailableModels()
    } catch (error) {
      throw this.handleError(error)
    }
  }

  /**
   * Test model connection
   */
  async testModelConnection(modelId: string): Promise<boolean> {
    try {
      return await llmApi.testModelConnection(modelId)
    } catch (error) {
      return false
    }
  }

  /**
   * Cancel a streaming request (if supported by backend)
   */
  async cancelStream(_requestId?: string): Promise<void> {
    try {
      await llmApi.cancelStream()
    } catch (error) {
      // Ignore cancellation errors as they may not be supported
      console.warn('Stream cancellation not supported:', error)
    }
  }

  /**
   * Validate request before sending to backend
   */
  private validateRequest(request: NativeLLMRequest): void {
    if (!request.model || request.model.trim() === '') {
      throw new LLMError('Model is required', 'model', false)
    }

    if (!request.messages || request.messages.length === 0) {
      throw new LLMError('Messages are required', 'unknown', false)
    }

    // Validate temperature range
    if (request.temperature !== undefined && (request.temperature < 0 || request.temperature > 2)) {
      throw new LLMError('Temperature must be between 0 and 2', 'unknown', false)
    }

    // Validate max tokens
    if (request.maxTokens !== undefined && request.maxTokens <= 0) {
      throw new LLMError('Max tokens must be positive', 'unknown', false)
    }
  }

  /**
   * Create a real-time ReadableStream from Tauri Channel for immediate streaming
   */
  private createStreamFromChannel(channel: Channel<NativeLLMStreamChunk>): ReadableStream<NativeLLMStreamChunk> {
    let isStreamClosed = false

    return new ReadableStream({
      start(controller) {
        channel.onmessage = (chunk: NativeLLMStreamChunk) => {
          if (isStreamClosed) return

          try {
            // Immediately enqueue each chunk for real-time streaming
            controller.enqueue(chunk)

            // Close stream on finish or error
            if (chunk.type === 'finish' || chunk.type === 'error') {
              isStreamClosed = true
              controller.close()
            }
          } catch (error) {
            if (!isStreamClosed) {
              isStreamClosed = true
              controller.error(error)
            }
          }
        }

        // Handle channel errors if the API supports it
        if ('onerror' in channel) {
          ;(channel as { onerror?: (error: unknown) => void }).onerror = (error: unknown) => {
            if (!isStreamClosed) {
              isStreamClosed = true
              controller.error(new Error(`Channel error: ${error}`))
            }
          }
        }
      },
      cancel() {
        isStreamClosed = true
        // Clean up channel resources if needed
        // Note: Tauri Channel API may not have explicit cleanup methods
      },
    })
  }

  /**
   * Handle and categorize errors from backend using enhanced error handler
   */
  private handleError(error: unknown): LLMError {
    return ErrorHandler.handleError(error)
  }
}
