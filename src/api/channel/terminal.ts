import { channelApi } from '@/api/channel'

export type TerminalChannelMessage =
  | { type: 'Data'; pane_id: number; data: number[] }
  | { type: 'Error'; pane_id: number; error: string }
  | { type: 'Close'; pane_id: number }

class TerminalChannelApi {
  private decoders = new Map<number, TextDecoder>()

  subscribe(paneId: number, onOutput: (text: string) => void) {
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
          console.warn('[terminalChannelApi] 通道错误:', err)
        },
      }
    )
  }

  /**
   * 二进制流订阅（高吞吐渲染）
   * 直接向上游传递 Uint8Array，供前端使用 xterm 的 writeUtf8 渲染。
   * 与 subscribe 的区别：不做任何文本解码。
   */
  subscribeBinary(paneId: number, onOutput: (bytes: Uint8Array) => void) {
    return channelApi.subscribe<TerminalChannelMessage>(
      'terminal_subscribe_output',
      { args: { pane_id: paneId } },
      {
        onMessage: msg => {
          if (msg.type === 'Data') {
            const bytes = new Uint8Array(msg.data)
            if (bytes.length) onOutput(bytes)
          } else if (msg.type === 'Close') {
            // no-op for binary variant (caller may handle flushing/cleanup)
          }
        },
        onError: err => {
          console.warn('[terminalChannelApi] 通道错误:', err)
        },
      }
    )
  }
}

export const terminalChannelApi = new TerminalChannelApi()
