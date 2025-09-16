export interface ChannelCallbacks<T> {
  onMessage: (message: T) => void
  onError?: (error: unknown) => void
}

export interface ChannelSubscription {
  unsubscribe: () => Promise<void>
}

export interface ChannelOptions<T = unknown> {
  /** 自定义取消命令名称 */
  cancelCommand?: string
  /** 判断是否应该关闭流的函数（用于 ReadableStream） */
  shouldClose?: (chunk: T) => boolean
}
