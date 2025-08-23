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

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(ui)

// ============================================================================
// 应用初始化 - 在挂载前完成关键初始化
// ============================================================================

/**
 * 初始化应用状态管理
 */
const initializeStores = async () => {
  try {
    // 初始化会话状态管理
    const sessionStore = useSessionStore()
    await sessionStore.initialize()

    // 初始化终端Store（包括会话恢复）
    const terminalStore = useTerminalStore()
    await terminalStore.initializeTerminalStore()

    // 初始化AI聊天Store
    const aiChatStore = useAIChatStore()
    await aiChatStore.initialize()
  } catch (error) {
    console.error('应用状态管理初始化失败:', error)
  }
}

/**
 * 初始化应用设置
 */
const initializeSettings = async () => {
  try {
    // 初始化AI设置
    const aiSettingsStore = useAISettingsStore()
    await aiSettingsStore.loadSettings()

    // 初始化主题系统
    const themeManager = useTheme()
    await themeManager.initialize()
  } catch (error) {
    console.warn('应用设置初始化失败:', error)
  }
}

/**
 * 初始化其他服务
 */
const initializeServices = async () => {
  try {
    // 初始化补全引擎
    await completionApi.initEngine()

    // 初始化AI聊天服务（包括Eko实例）
    const aiChatStore = useAIChatStore()
    await aiChatStore.initializeEko()
  } catch (error) {
    console.warn('服务初始化失败:', error)
  }
}

/**
 * 应用启动初始化
 */
const initializeApplication = async () => {
  try {
    // 先初始化主题系统，确保在DOM渲染前主题就绪
    console.log('开始初始化主题系统...')
    const themeManager = useTheme()
    await themeManager.initialize()
    console.log('主题系统初始化完成')

    // 挂载应用 - 此时主题已就绪
    app.mount('#app')
    console.log('应用挂载完成')

    // 并行初始化其他系统
    await Promise.allSettled([initializeStores(), initializeServices()])

    // 设置窗口关闭监听器
    setupWindowCloseListener()
  } catch (error) {
    console.error('应用初始化过程中发生错误:', error)
    // 即使主题初始化失败，也要挂载应用
    if (!document.getElementById('app')?.hasChildNodes()) {
      app.mount('#app')
    }
  }
}

// 启动应用初始化
initializeApplication()

// ============================================================================
// 生产环境安全设置
// ============================================================================

/**
 * 在生产环境中禁用右键菜单
 */
const disableContextMenuInProduction = () => {
  // 只在生产环境（打包后）禁用右键菜单
  if (import.meta.env.PROD) {
    document.addEventListener('contextmenu', event => {
      event.preventDefault()
      return false
    })

    // 禁用F12开发者工具
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

// 应用安全设置
disableContextMenuInProduction()

// ============================================================================
// 应用生命周期管理
// ============================================================================

/**
 * 应用关闭时的清理工作
 */
const handleAppClose = async () => {
  try {
    // 保存窗口状态（使用官方插件）
    await saveWindowState(StateFlags.ALL)

    // 保存终端状态（这会自动同步并保存会话状态）
    const terminalStore = useTerminalStore()
    await terminalStore.saveSessionState()

    // 清理会话存储资源
    const sessionStore = useSessionStore()
    sessionStore.cleanup()
  } catch (error) {
    console.error('应用关闭清理失败:', error)
  }
}

/**
 * 设置 Tauri 窗口关闭事件监听器
 */
const setupWindowCloseListener = async () => {
  try {
    // 监听窗口关闭请求事件
    const unlisten = await getCurrentWindow().onCloseRequested(async event => {
      // 阻止默认关闭行为，这样我们可以先执行保存操作
      event.preventDefault()

      try {
        // 执行保存操作
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
