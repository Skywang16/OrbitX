/**
 * 快捷键监听器 Composable
 *
 * 负责监听全局键盘事件并执行对应的快捷键动作
 */

import { ref, onMounted, onUnmounted } from 'vue'
import { shortcutsApi } from '@/api/shortcuts'
import type { ShortcutsConfig, ShortcutBinding } from '@/api/shortcuts/types'
import { shortcutActionsService } from './actions'
import { formatKeyCombo, isShortcutMatch, extractActionName } from './utils'
import { NON_BLOCKING_ACTIONS } from './constants'

export function useShortcutListener() {
  const isListening = ref(false)
  const config = ref<ShortcutsConfig | null>(null)
  let keydownHandler: ((event: KeyboardEvent) => void) | null = null

  /**
   * 初始化快捷键监听器
   */
  const initializeListener = async () => {
    try {
      config.value = await shortcutsApi.getConfig()

      keydownHandler = (event: KeyboardEvent) => {
        handleKeyDown(event)
      }

      document.addEventListener('keydown', keydownHandler, true)
      isListening.value = true
    } catch (error) {
      console.error('快捷键监听器初始化失败:', error)
    }
  }

  /**
   * 处理键盘按下事件
   */
  const handleKeyDown = async (event: KeyboardEvent) => {
    if (!config.value) return

    const keyCombo = formatKeyCombo(event)
    const matchedShortcut = findMatchingShortcut(event, config.value)

    if (matchedShortcut) {
      const actionName = extractActionName(matchedShortcut.action)
      const frontendResult = await executeShortcutAction(matchedShortcut, keyCombo)

      // 特殊处理 close_tab：如果前端返回 false（表示无法关闭），则不阻止默认行为
      if (actionName === 'close_tab' && !frontendResult) {
        // 不阻止默认行为，让 cmd+w 能够关闭窗口
        console.log('最后一个标签页，允许 cmd+w 关闭窗口')
        return
      }

      if (!NON_BLOCKING_ACTIONS.has(actionName)) {
        event.preventDefault()
        event.stopPropagation()
      }
    }
  }

  /**
   * 查找匹配的快捷键
   */
  const findMatchingShortcut = (event: KeyboardEvent, config: ShortcutsConfig): ShortcutBinding | null => {
    const allShortcuts = [...config.global, ...config.terminal, ...config.custom]

    for (const shortcut of allShortcuts) {
      if (isShortcutMatch(event, shortcut)) {
        return shortcut
      }
    }

    return null
  }

  /**
   * 执行快捷键动作
   */
  const executeShortcutAction = async (shortcut: ShortcutBinding, keyCombo: string) => {
    try {
      const actionName = extractActionName(shortcut.action)
      let frontendResult = false

      switch (actionName) {
        case 'switch_to_tab_1':
          frontendResult = shortcutActionsService.switchToTab(0)
          break
        case 'switch_to_tab_2':
          frontendResult = shortcutActionsService.switchToTab(1)
          break
        case 'switch_to_tab_3':
          frontendResult = shortcutActionsService.switchToTab(2)
          break
        case 'switch_to_tab_4':
          frontendResult = shortcutActionsService.switchToTab(3)
          break
        case 'switch_to_tab_5':
          frontendResult = shortcutActionsService.switchToTab(4)
          break
        case 'switch_to_last_tab':
          frontendResult = shortcutActionsService.switchToLastTab()
          break
        case 'new_tab':
          frontendResult = await shortcutActionsService.newTab()
          break
        case 'close_tab':
          frontendResult = shortcutActionsService.closeCurrentTab()
          break
        case 'copy_to_clipboard':
          frontendResult = await shortcutActionsService.copyToClipboard()
          break
        case 'paste_from_clipboard':
          frontendResult = await shortcutActionsService.pasteFromClipboard()
          break
        case 'accept_completion':
          frontendResult = shortcutActionsService.acceptCompletion()
          break
      }

      await shortcutsApi.executeAction(shortcut.action, keyCombo, getCurrentTerminalId(), {
        timestamp: new Date().toISOString(),
        userAgent: navigator.userAgent,
        frontendResult,
      })

      return frontendResult
    } catch (error) {
      console.error('快捷键动作执行失败:', error)
      return false
    }
  }

  const getCurrentTerminalId = (): string | null => {
    return null
  }

  const reloadConfig = async () => {
    try {
      config.value = await shortcutsApi.getConfig()
    } catch (error) {
      console.error('快捷键配置重新加载失败:', error)
    }
  }

  const stopListener = () => {
    if (keydownHandler) {
      document.removeEventListener('keydown', keydownHandler, true)
      keydownHandler = null
    }
    isListening.value = false
  }

  // 自动初始化
  onMounted(() => {
    initializeListener()
  })

  // 清理
  onUnmounted(() => {
    stopListener()
  })

  return {
    isListening,
    config,
    reloadConfig,
  }
}
