/**
 * 统一的确认对话框API
 */

import { createApp, h, ref } from 'vue'
import XModal from '../components/Modal.vue'
import { createI18n } from 'vue-i18n'
import zhMessages from '@/i18n/locales/zh.json'
import enMessages from '@/i18n/locales/en.json'

export interface ConfirmConfig {
  /** 确认消息内容 */
  message: string
  /** 对话框标题 */
  title?: string
  /** 确认按钮文字 */
  confirmText?: string
  /** 取消按钮文字 */
  cancelText?: string
  /** 对话框类型，影响样式 */
  type?: 'info' | 'warning' | 'danger'
  /** 是否显示取消按钮 */
  showCancelButton?: boolean
}

/**
 * 显示确认对话框
 */
export const confirm = (config: string | ConfirmConfig): Promise<boolean> => {
  return new Promise(resolve => {
    // 标准化配置
    const normalizedConfig: Required<ConfirmConfig> = {
      message: typeof config === 'string' ? config : config.message,
      title: typeof config === 'string' ? '确认' : config.title || '确认',
      confirmText: typeof config === 'string' ? '确定' : config.confirmText || '确定',
      cancelText: typeof config === 'string' ? '取消' : config.cancelText || '取消',
      type: typeof config === 'string' ? 'info' : config.type || 'info',
      showCancelButton: typeof config === 'string' ? true : (config.showCancelButton ?? true),
    }

    // 创建容器元素
    const container = document.createElement('div')
    document.body.appendChild(container)

    // 响应式状态
    const visible = ref(true)

    // 清理状态标记，防止重复清理
    let isCleanedUp = false

    // 安全的清理函数
    const cleanup = () => {
      if (isCleanedUp) {
        return
      }

      isCleanedUp = true

      try {
        // 安全地卸载Vue应用
        if (app) {
          app.unmount()
        }
      } catch (error) {
        console.warn('Failed to unmount confirm dialog app:', error)
      }

      try {
        // 安全地移除DOM元素
        if (container && container.parentNode === document.body) {
          document.body.removeChild(container)
        }
      } catch (error) {
        console.warn('Failed to remove confirm dialog container:', error)
      }
    }

    // 结果处理状态，防止重复处理
    let isResolved = false

    // 统一的结果处理函数
    const handleResult = (result: boolean) => {
      if (isResolved) {
        return
      }

      isResolved = true
      visible.value = false

      setTimeout(() => {
        cleanup()
        resolve(result)
      }, 150) // 等待动画完成
    }

    // 处理确认
    const handleConfirm = () => {
      handleResult(true)
    }

    // 处理取消
    const handleCancel = () => {
      handleResult(false)
    }

    // 处理关闭
    const handleClose = () => {
      handleResult(false)
    }

    // 创建i18n实例
    const i18n = createI18n({
      legacy: false,
      locale: 'zh',
      fallbackLocale: 'en',
      messages: {
        zh: zhMessages,
        en: enMessages,
      },
    })

    // 创建Vue应用实例
    const app = createApp({
      setup() {
        return () =>
          h(
            XModal,
            {
              visible: visible.value,
              'onUpdate:visible': (newVisible: boolean) => {
                visible.value = newVisible
                // 不在这里调用handleClose，避免与其他事件处理器冲突
                // 让具体的事件处理器（onCancel、onClose等）来处理结果
              },
              title: normalizedConfig.title,
              size: 'small',
              showFooter: true,
              showCancelButton: normalizedConfig.showCancelButton,
              showConfirmButton: true,
              cancelText: normalizedConfig.cancelText,
              confirmText: normalizedConfig.confirmText,
              maskClosable: true,
              closable: true,
              onConfirm: handleConfirm,
              onCancel: handleCancel,
              onClose: handleClose,
              class: `confirm-modal confirm-modal--${normalizedConfig.type}`,
            },
            {
              default: () =>
                h(
                  'div',
                  {
                    class: 'confirm-content',
                  },
                  [
                    // 图标
                    normalizedConfig.type === 'warning' &&
                      h(
                        'div',
                        {
                          class: 'confirm-icon confirm-icon--warning',
                        },
                        [
                          h(
                            'svg',
                            {
                              width: '24',
                              height: '24',
                              viewBox: '0 0 24 24',
                              fill: 'none',
                              stroke: 'currentColor',
                              'stroke-width': '2',
                            },
                            [
                              h('path', {
                                d: 'm21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z',
                              }),
                              h('path', { d: 'M12 9v4' }),
                              h('path', { d: 'm12 17 .01 0' }),
                            ]
                          ),
                        ]
                      ),

                    normalizedConfig.type === 'danger' &&
                      h(
                        'div',
                        {
                          class: 'confirm-icon confirm-icon--danger',
                        },
                        [
                          h(
                            'svg',
                            {
                              width: '24',
                              height: '24',
                              viewBox: '0 0 24 24',
                              fill: 'none',
                              stroke: 'currentColor',
                              'stroke-width': '2',
                            },
                            [
                              h('circle', { cx: '12', cy: '12', r: '10' }),
                              h('path', { d: 'M15 9l-6 6' }),
                              h('path', { d: 'M9 9l6 6' }),
                            ]
                          ),
                        ]
                      ),

                    normalizedConfig.type === 'info' &&
                      h(
                        'div',
                        {
                          class: 'confirm-icon confirm-icon--info',
                        },
                        [
                          h(
                            'svg',
                            {
                              width: '24',
                              height: '24',
                              viewBox: '0 0 24 24',
                              fill: 'none',
                              stroke: 'currentColor',
                              'stroke-width': '2',
                            },
                            [
                              h('circle', { cx: '12', cy: '12', r: '10' }),
                              h('path', { d: 'M12 16v-4' }),
                              h('path', { d: 'M12 8h.01' }),
                            ]
                          ),
                        ]
                      ),

                    // 消息内容
                    h(
                      'div',
                      {
                        class: 'confirm-message',
                      },
                      normalizedConfig.message
                    ),
                  ]
                ),
            }
          )
      },
    })

    // 安装i18n插件
    app.use(i18n)

    // 挂载应用
    app.mount(container)
  })
}

/**
 * 便捷方法：显示警告确认对话框
 */
export const confirmWarning = (message: string, title = '警告'): Promise<boolean> => {
  return confirm({
    message,
    title,
    type: 'warning',
  })
}

/**
 * 便捷方法：显示危险确认对话框
 */
export const confirmDanger = (message: string, title = '危险操作'): Promise<boolean> => {
  return confirm({
    message,
    title,
    type: 'danger',
  })
}

/**
 * 便捷方法：显示信息确认对话框
 */
export const confirmInfo = (message: string, title = '确认'): Promise<boolean> => {
  return confirm({
    message,
    title,
    type: 'info',
  })
}
