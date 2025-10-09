/**
 * 窗口透明度管理 Composable
 *
 * 统一管理窗口透明度状态和事件监听
 */

import { ref, onMounted, onUnmounted } from 'vue'
import { setWindowOpacity, getWindowOpacity, onOpacityChanged } from '@/api/window/opacity'
import { applyBackgroundOpacity } from '@/utils/themeApplier'

/**
 * 窗口透明度管理 hook
 */
export function useWindowOpacity() {
  const opacity = ref(1.0)
  const isLoading = ref(false)
  let unlisten: (() => void) | null = null

  /**
   * 更新透明度
   */
  const updateOpacity = async (value: number) => {
    try {
      isLoading.value = true
      await setWindowOpacity(value)
      // 后端会发送事件,监听器会自动处理
    } catch (error) {
      console.error('Failed to set opacity:', error)
      throw error
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 重置透明度为完全不透明
   */
  const resetOpacity = async () => {
    await updateOpacity(1.0)
  }

  /**
   * 初始化透明度监听
   */
  const initOpacityListener = async () => {
    try {
      // 获取初始透明度
      opacity.value = await getWindowOpacity()

      // 设置 CSS 变量
      if (typeof document !== 'undefined') {
        document.documentElement.style.setProperty('--bg-opacity', String(opacity.value))
      }

      applyBackgroundOpacity(opacity.value)

      // 监听透明度变化
      unlisten = await onOpacityChanged(newOpacity => {
        opacity.value = newOpacity

        // 1. 设置 CSS 变量 (colorUtils.ts 依赖这个变量)
        if (typeof document !== 'undefined') {
          document.documentElement.style.setProperty('--bg-opacity', String(newOpacity))
        }

        // 2. 应用透明度到主题颜色
        applyBackgroundOpacity(newOpacity)

        // 3. 触发自定义事件,让终端等组件响应
        if (typeof window !== 'undefined') {
          window.dispatchEvent(
            new CustomEvent('opacity-changed', {
              detail: { opacity: newOpacity },
            })
          )
        }
      })
    } catch (error) {
      console.error('Failed to initialize opacity listener:', error)
    }
  }

  /**
   * 清理监听器
   */
  const cleanupListener = () => {
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }

  onMounted(() => {
    initOpacityListener()
  })

  onUnmounted(() => {
    cleanupListener()
  })

  return {
    opacity,
    isLoading,
    updateOpacity,
    resetOpacity,
  }
}
