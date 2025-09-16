import { Channel, invoke } from '@tauri-apps/api/core'
import type { ChannelCallbacks, ChannelSubscription, ChannelOptions } from './types'

/**
 * 统一的 Channel API 封装
 * 提供类似 invoke 封装的统一接口，支持不同类型的流式数据订阅
 */
class ChannelApi {
  /**
   * 通用订阅方法
   */
  subscribe<T>(
    command: string,
    payload: Record<string, unknown>,
    callbacks: ChannelCallbacks<T>,
    options?: ChannelOptions<T>
  ): ChannelSubscription {
    const channel = new Channel<T>()
    channel.onmessage = callbacks.onMessage

    // 处理错误回调
    if (callbacks.onError && 'onerror' in channel) {
      ;(channel as Channel<T> & { onerror?: (error: unknown) => void }).onerror = callbacks.onError
    }

    // 触发后端订阅命令
    invoke(command, { ...payload, channel }).catch(err => {
      if (callbacks.onError) callbacks.onError(err)
      else console.warn(`[channelApi] invoke ${command} error:`, err)
    })

    return {
      unsubscribe: async () => {
        try {
          const cancelCommand = options?.cancelCommand || `${command}_cancel`
          await invoke(cancelCommand, payload)
        } catch (err) {
          console.warn(`[channelApi] cancel ${command} error:`, err)
        }
      },
    }
  }

  /**
   * 创建流式 ReadableStream（用于 LLM 等需要 ReadableStream 的场景）
   */
  createStream<T>(command: string, payload: Record<string, unknown>, options?: ChannelOptions<T>): ReadableStream<T> {
    const channel = new Channel<T>()
    let isStreamClosed = false

    // 启动后端命令
    invoke(command, { ...payload, channel }).catch(error => {
      console.error(`[channelApi] stream invoke ${command} error:`, error)
    })

    return new ReadableStream({
      start(controller) {
        channel.onmessage = (chunk: T) => {
          if (isStreamClosed) return

          try {
            controller.enqueue(chunk)

            // 检查是否应该关闭流
            if (options?.shouldClose?.(chunk)) {
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

        // 处理 Channel 错误
        if ('onerror' in channel) {
          ;(channel as Channel<T> & { onerror?: (error: unknown) => void }).onerror = (error: unknown) => {
            if (!isStreamClosed) {
              isStreamClosed = true
              controller.error(new Error(`Channel error: ${error}`))
            }
          }
        }
      },
      cancel() {
        isStreamClosed = true
        // 取消后端命令
        const cancelCommand = options?.cancelCommand || `${command}_cancel`
        invoke(cancelCommand, payload).catch(console.warn)
      },
    })
  }
}

export const channelApi = new ChannelApi()

// 导出具体的 Channel API 实例，类似 invoke 的使用方式
export { channelApi as channel }

// 导出类型
export * from './types'
