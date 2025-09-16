import { channelApi } from './index'
import type { ChannelSubscription } from './types'

export type TerminalChannelMessage =
  | { type: 'Data'; pane_id: number; data: number[] }
  | { type: 'Error'; pane_id: number; error: string }
  | { type: 'Close'; pane_id: number }

/**
 * 终端专用 Channel API
 */
class TerminalChannelApi {
  private decoders = new Map<number, TextDecoder>()

  /**
   * 订阅终端输出
   */
  subscribe(paneId: number, onOutput: (text: string) => void): ChannelSubscription {
    if (!this.decoders.has(paneId)) {
      this.decoders.set(paneId, new TextDecoder('utf-8', { fatal: false }))
    }

    return channelApi.subscribe<TerminalChannelMessage>(
      'terminal_subscribe_output',
      { args: { pane_id: paneId } },
      {
        onMessage: msg => {
          if (msg.type === 'Data') {
            const decoder = this.decoders.get(msg.pane_id)!
            const text = decoder.decode(new Uint8Array(msg.data), { stream: true })
            if (text) onOutput(text)
          } else if (msg.type === 'Close') {
            // flush decoder
            const decoder = this.decoders.get(msg.pane_id)
            if (decoder) {
              const remaining = decoder.decode()
              if (remaining) onOutput(remaining)
            }
            this.decoders.delete(msg.pane_id)
          }
        },
        onError: err => {
          console.warn('[terminalChannelApi] channel error:', err)
        },
      }
    )
  }
}

export const terminalChannelApi = new TerminalChannelApi()
