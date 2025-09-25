export interface TaskEvent {
  type: string
  payload: unknown
  timestamp?: number
}

export type EventListener = (event: TaskEvent) => void

/**
 * 轻量事件发射器，用于在核心模块之间解耦消息传递
 */
export class EventEmitter {
  private listeners: Map<string, EventListener[]> = new Map()

  emit(event: TaskEvent): void {
    const listeners = this.listeners.get(event.type) || []
    for (const listener of listeners) {
      try {
        listener(event)
      } catch (error) {
        // 降级处理，避免监听器异常中断主流程

        console.error(`Error in event listener for ${event.type}:`, error)
      }
    }
  }

  on(eventType: string, listener: EventListener): void {
    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, [])
    }
    this.listeners.get(eventType)!.push(listener)
  }

  off(eventType: string, listener: EventListener): void {
    const listeners = this.listeners.get(eventType)
    if (listeners) {
      const index = listeners.indexOf(listener)
      if (index > -1) {
        listeners.splice(index, 1)
      }
    }
  }

  once(eventType: string, listener: EventListener): void {
    const onceListener: EventListener = event => {
      listener(event)
      this.off(eventType, onceListener)
    }
    this.on(eventType, onceListener)
  }

  clear(): void {
    this.listeners.clear()
  }
}
