import { completionApi } from '@/api'
import { configApi } from '@/api/config'
import { windowApi } from '@/api/window'

import { useAISettingsStore } from '@/components/settings/components/AI'
import { useAIChatStore } from '@/components/AIChatSidebar/store'
import { useThemeStore } from '@/stores/theme'
import { useSessionStore } from '@/stores/session'

import { useTerminalStore } from '@/stores/Terminal'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'

import './styles/variables.css'
import ui from './ui'
import { i18n, initLocale } from './i18n'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(ui)
app.use(i18n)

const initializeStores = async () => {
  try {
    const sessionStore = useSessionStore()
    await sessionStore.initialize()

    const terminalStore = useTerminalStore()
    await terminalStore.initializeTerminalStore()

    const aiChatStore = useAIChatStore()
    await aiChatStore.initialize()
  } catch (error) {
    console.error('应用状态管理初始化失败:', error)
  }
}

const initializeSettings = async () => {
  try {
    const aiSettingsStore = useAISettingsStore()
    await aiSettingsStore.loadSettings()

    const themeStore = useThemeStore()
    await themeStore.initialize()
  } catch (error) {
    console.warn('应用设置初始化失败:', error)
  }
}

const initializeServices = async () => {
  await completionApi.initEngine()

  const aiChatStore = useAIChatStore()
  await aiChatStore.initialize()
}

const initializeOpacity = async () => {
  try {
    const config = await configApi.getConfig()

    if (config.appearance.opacity !== undefined) {
      await windowApi.setWindowOpacity(config.appearance.opacity)
    }
  } catch (error) {
    console.warn('初始化透明度失败:', error)
  }
}

const initializeApplication = async () => {
  try {
    const themeStore = useThemeStore()
    await Promise.allSettled([themeStore.initialize(), initLocale(), initializeOpacity()])

    app.mount('#app')

    await Promise.allSettled([initializeStores(), initializeSettings(), initializeServices()])

    setupWindowCloseListener()
  } catch (error) {
    console.error('应用初始化过程中发生错误:', error)
    if (!document.getElementById('app')?.hasChildNodes()) {
      app.mount('#app')
    }
  }
}

initializeApplication()

const disableContextMenuInProduction = () => {
  if (import.meta.env.PROD) {
    document.addEventListener('contextmenu', event => {
      event.preventDefault()
      return false
    })

    document.addEventListener('keydown', event => {
      if (
        event.key === 'F12' ||
        (event.ctrlKey && event.shiftKey && event.key === 'I') ||
        (event.ctrlKey && event.shiftKey && event.key === 'C') ||
        (event.ctrlKey && event.key === 'U')
      ) {
        event.preventDefault()
        return false
      }
    })
  }
}

disableContextMenuInProduction()

const handleAppClose = async () => {
  try {
    const terminalStore = useTerminalStore()
    await terminalStore.saveSessionState()

    const sessionStore = useSessionStore()
    sessionStore.cleanup()
  } catch (error) {
    console.error('应用关闭清理失败:', error)
  }
}

const setupWindowCloseListener = async () => {
  try {
    const currentWindow = getCurrentWindow()
    const unlistenClose = await currentWindow.onCloseRequested(async event => {
      event.preventDefault()

      await handleAppClose()

      unlistenClose()
      await currentWindow.close()
    })

    return () => {
      unlistenClose()
    }
  } catch (error) {
    console.error('设置窗口事件监听失败:', error)
  }
}
