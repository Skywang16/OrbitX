import { completionApi, dockApi } from '@/api'

import { useAISettingsStore } from '@/components/settings/components/AI'
import { useAIChatStore } from '@/components/AIChatSidebar/store'
import { useThemeStore } from '@/stores/theme'
import { useSessionStore } from '@/stores/session'
import { useLayoutStore } from '@/stores/layout'

import { useTerminalStore } from '@/stores/Terminal'
import { useEditorStore } from '@/stores/Editor'
import { useFileWatcherStore } from '@/stores/fileWatcher'
import { createPinia } from 'pinia'
import { createApp } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import App from './App.vue'

import './styles/variables.css'
import ui from './ui'
import { i18n, initLocale } from './i18n'
import { getWindowOpacity } from '@/api/window/opacity'

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

    const fileWatcherStore = useFileWatcherStore()
    await fileWatcherStore.initialize()

    const editorStore = useEditorStore()
    await editorStore.initialize()

    const layoutStore = useLayoutStore()
    await layoutStore.initialize()

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
    const opacity = await getWindowOpacity()
    document.documentElement.style.setProperty('--bg-opacity', opacity.toString())
  } catch (error) {
    console.warn('初始化透明度失败:', error)
    document.documentElement.style.setProperty('--bg-opacity', '1.0')
  }
}

const initializeApplication = async () => {
  try {
    const themeStore = useThemeStore()
    const sessionStore = useSessionStore()
    await Promise.allSettled([themeStore.initialize(), initLocale(), initializeOpacity(), sessionStore.initialize()])

    app.mount('#app')

    await Promise.allSettled([initializeStores(), initializeSettings(), initializeServices()])

    setupDockFocusListener()
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

// 全局拦截外部链接点击，在系统浏览器中打开
const setupExternalLinkHandler = () => {
  document.addEventListener('click', (e: MouseEvent) => {
    const target = e.target as HTMLElement
    const link = target.closest('a[href]') as HTMLAnchorElement | null

    if (link) {
      const href = link.getAttribute('href')
      // 跳过内部锚点链接和 javascript: 链接
      if (!href || href.startsWith('#') || href.startsWith('javascript:')) {
        return
      }
      // 外部链接用系统浏览器打开
      if (href.startsWith('http://') || href.startsWith('https://')) {
        e.preventDefault()
        openUrl(href).catch(err => {
          console.error('Failed to open URL:', err)
        })
      }
    }
  })
}

setupExternalLinkHandler()

const setupDockFocusListener = async () => {
  await dockApi.onDockSwitchTab(payload => {
    const editor = useEditorStore()
    const loc = Object.values(editor.workspace.groups)
      .map(g => ({ groupId: g.id, tabId: g.tabs.find(t => t.id === payload.tabId)?.id }))
      .find(x => x.tabId)
    if (loc) {
      editor.setActiveTab(loc.groupId, payload.tabId)
    }
  })
}
