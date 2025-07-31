import { createApp, h } from 'vue'
import XMessage from '../components/Message.vue'

// 消息类型
export type MessageType = 'success' | 'error' | 'warning' | 'info'

// 消息配置接口
export interface MessageConfig {
  message: string
  type?: MessageType
  duration?: number
  closable?: boolean
  onClose?: () => void
  id?: string
}

// 消息实例接口
export interface MessageInstance {
  id: string
  close: () => void
  update: (config: Partial<MessageConfig>) => void
}

// 消息队列管理
class MessageManager {
  private instances: Map<string, MessageInstance> = new Map()
  private container: HTMLElement | null = null
  private zIndex = 1000

  // 获取或创建容器
  private getContainer(): HTMLElement {
    if (!this.container) {
      this.container = document.createElement('div')
      this.container.className = 'x-message-container'
      this.container.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        z-index: ${this.zIndex};
        pointer-events: none;
        display: flex;
        flex-direction: column;
        gap: 8px;
      `
      document.body.appendChild(this.container)
    }
    return this.container
  }

  // 创建消息实例
  create(config: MessageConfig): MessageInstance {
    const id = config.id || `message_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
    const container = this.getContainer()

    // 默认配置
    const defaultConfig = {
      duration: 3000,
      maxCount: 5,
      placement: 'top-right',
    }

    // 检查最大数量限制
    if (this.instances.size >= defaultConfig.maxCount) {
      // 移除最早的消息
      const firstId = this.instances.keys().next().value
      if (firstId) {
        this.instances.get(firstId)?.close()
      }
    }

    // 创建消息元素容器
    const messageElement = document.createElement('div')
    messageElement.style.pointerEvents = 'auto'
    container.appendChild(messageElement)

    // 创建Vue应用实例
    const app = createApp({
      render() {
        return h(XMessage, {
          visible: true,
          message: config.message,
          type: config.type || 'info',
          duration: config.duration ?? defaultConfig.duration,
          onClose: () => {
            instance.close()
          },
        })
      },
    })

    // 挂载应用
    app.mount(messageElement)

    // 创建实例对象
    const instance: MessageInstance = {
      id,
      close: () => {
        // 移除DOM元素
        if (messageElement.parentNode) {
          messageElement.parentNode.removeChild(messageElement)
        }
        // 卸载Vue应用
        app.unmount()
        // 从实例映射中移除
        this.instances.delete(id)
        // 调用关闭回调
        config.onClose?.()

        // 如果没有消息了，移除容器
        if (this.instances.size === 0 && this.container) {
          document.body.removeChild(this.container)
          this.container = null
        }
      },
      update: (newConfig: Partial<MessageConfig>) => {
        // 更新配置（这里可以根据需要实现更复杂的更新逻辑）
        Object.assign(config, newConfig)
      },
    }

    // 存储实例
    this.instances.set(id, instance)

    return instance
  }

  // 关闭所有消息
  closeAll(): void {
    this.instances.forEach(instance => instance.close())
  }

  // 根据ID关闭消息
  close(id: string): void {
    this.instances.get(id)?.close()
  }
}

// 全局消息管理器实例
const messageManager = new MessageManager()

// 创建消息的主函数
export const createMessage = (config: string | MessageConfig): MessageInstance => {
  const messageConfig: MessageConfig = typeof config === 'string' ? { message: config } : config

  return messageManager.create(messageConfig)
}

// 便捷方法
createMessage.success = (message: string, duration?: number): MessageInstance => {
  return createMessage({ message, type: 'success', duration })
}

createMessage.error = (message: string, duration?: number): MessageInstance => {
  return createMessage({ message, type: 'error', duration })
}

createMessage.warning = (message: string, duration?: number): MessageInstance => {
  return createMessage({ message, type: 'warning', duration })
}

createMessage.info = (message: string, duration?: number): MessageInstance => {
  return createMessage({ message, type: 'info', duration })
}

// 关闭所有消息
createMessage.closeAll = (): void => {
  messageManager.closeAll()
}

// 根据ID关闭消息
createMessage.close = (id: string): void => {
  messageManager.close(id)
}

// 默认导出
export default createMessage
