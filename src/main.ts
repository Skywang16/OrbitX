/*
 * Copyright (C) 2025 OrbitX Contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

import { completionApi } from '@/api'

import { useAISettingsStore } from '@/components/settings/components/AI'
import { useAIChatStore } from '@/components/AIChatSidebar/store'
import { useTheme } from '@/composables/useTheme'
import { useSessionStore } from '@/stores/session'

import { useTerminalStore } from '@/stores/Terminal'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
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

    const themeManager = useTheme()
    await themeManager.initialize()
  } catch (error) {
    console.warn('应用设置初始化失败:', error)
  }
}

const initializeServices = async () => {
  try {
    await completionApi.initEngine()

    const aiChatStore = useAIChatStore()
    await aiChatStore.initializeEko()
  } catch (error) {
    console.warn('服务初始化失败:', error)
  }
}

const initializeOpacity = async () => {
  try {
    const { configApi } = await import('@/api/config')
    const { windowApi } = await import('@/api/window')

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
    const themeManager = useTheme()
    await Promise.allSettled([themeManager.initialize(), initLocale(), initializeOpacity()])

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
    await saveWindowState(StateFlags.ALL)

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
    const unlisten = await getCurrentWindow().onCloseRequested(async event => {
      event.preventDefault()

      try {
        await handleAppClose()
      } catch (error) {
        console.error('保存失败:', error)
      }

      unlisten()
      await getCurrentWindow().close()
    })

    return unlisten
  } catch (error) {
    console.error(error)
  }
}
