/**
 * 快捷键监听器 Composable
 *
 * 负责监听全局键盘事件并执行对应的快捷键动作
 */

import { ref, onMounted, onUnmounted } from 'vue'
import { shortcutsApi } from '@/api/shortcuts'
import type { ShortcutsConfig, ShortcutBinding } from '@/types'
import { shortcutActionsService } from './actions'
import { formatKeyCombo, isShortcutMatch, extractActionName } from './utils'

export function useShortcutListener() {
  const isListening = ref(false)
  const config = ref<ShortcutsConfig | null>(null)
  let keydownHandler: ((event: KeyboardEvent) => void) | null = null
  let wheelHandler: ((event: WheelEvent) => void) | null = null

  const initializeListener = async () => {
    config.value = await shortcutsApi.getConfig()

    keydownHandler = (event: KeyboardEvent) => {
      handleKeyDown(event)
    }

    wheelHandler = (event: WheelEvent) => {
      handleWheel(event)
    }

    document.addEventListener('keydown', keydownHandler, true)
    document.addEventListener('wheel', wheelHandler, true)
    isListening.value = true
  }

  const handleKeyDown = async (event: KeyboardEvent) => {
    if (!config.value) return

    const keyCombo = formatKeyCombo(event)
    const matchedShortcut = findMatchingShortcut(event, config.value)

    if (matchedShortcut) {
      const actionName = extractActionName(matchedShortcut.action)

      // 复制粘贴不阻止默认行为，其他都阻止
      // 必须在同步阶段调用 preventDefault，否则系统默认行为可能已经触发
      if (actionName !== 'copy_to_clipboard' && actionName !== 'paste_from_clipboard') {
        event.preventDefault()
        event.stopPropagation()
      }

      await executeShortcutAction(matchedShortcut, keyCombo)
    }
  }

  const handleWheel = async (event: WheelEvent) => {
    // 检查是否按下了 Cmd (Mac) 或 Ctrl (Windows/Linux)
    const isModifierPressed = event.metaKey || event.ctrlKey

    if (!isModifierPressed) return

    // 阻止默认滚轮行为
    event.preventDefault()
    event.stopPropagation()

    // 根据滚轮方向确定动作
    const action = event.deltaY < 0 ? 'increase_opacity' : 'decrease_opacity'
    const keyCombo = `${event.metaKey ? 'cmd' : 'ctrl'}+wheel`

    // 创建虚拟快捷键绑定
    const virtualShortcut: ShortcutBinding = {
      key: 'wheel',
      modifiers: [event.metaKey ? 'cmd' : 'ctrl'],
      action,
    }

    await executeShortcutAction(virtualShortcut, keyCombo)
  }

  const findMatchingShortcut = (event: KeyboardEvent, config: ShortcutsConfig): ShortcutBinding | null => {
    for (const shortcut of config) {
      if (isShortcutMatch(event, shortcut)) {
        return shortcut
      }
    }

    return null
  }

  const executeShortcutAction = async (shortcut: ShortcutBinding, keyCombo: string) => {
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
      case 'terminal_search':
        frontendResult = shortcutActionsService.terminalSearch()
        break
      case 'open_settings':
        frontendResult = shortcutActionsService.openSettings()
        break

      case 'new_window':
        frontendResult = await shortcutActionsService.newWindow()
        break
      case 'clear_terminal':
        frontendResult = shortcutActionsService.clearTerminal()
        break
      case 'increase_font_size':
        frontendResult = shortcutActionsService.increaseFontSize()
        break
      case 'decrease_font_size':
        frontendResult = shortcutActionsService.decreaseFontSize()
        break
      case 'increase_opacity':
        frontendResult = await shortcutActionsService.increaseOpacity()
        break
      case 'decrease_opacity':
        frontendResult = await shortcutActionsService.decreaseOpacity()
        break
    }

    await shortcutsApi.executeAction(shortcut.action, keyCombo, getCurrentTerminalId(), {
      timestamp: new Date().toISOString(),
      userAgent: navigator.userAgent,
      frontendResult,
    })

    return frontendResult
  }

  const getCurrentTerminalId = (): string | null => {
    return null
  }

  const reloadConfig = async () => {
    config.value = await shortcutsApi.getConfig()
  }

  const stopListener = () => {
    if (keydownHandler) {
      document.removeEventListener('keydown', keydownHandler, true)
      keydownHandler = null
    }
    if (wheelHandler) {
      document.removeEventListener('wheel', wheelHandler, true)
      wheelHandler = null
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
