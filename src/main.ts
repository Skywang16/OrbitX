import { completionAPI } from '@/api'
import { storage } from '@/api/storage'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useAIChatStore } from '@/components/AIChatSidebar/store'
import { useTheme } from '@/composables/useTheme'
import { useSessionStore } from '@/stores/session'
import { useSystemStore } from '@/stores/system'
import { useTerminalStore } from '@/stores/Terminal'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'

import './styles/variables.css'
import ui from './ui'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(ui)

// 挂载应用
app.mount('#app')

// ============================================================================
// 应用初始化
// ============================================================================

/**
 * 初始化存储系统
 */
const initializeStorageSystem = async () => {
  try {
    // 预加载缓存，提升后续访问性能
    await storage.preloadCache()
  } catch (error) {
    console.warn('存储系统缓存预加载失败:', error)
  }
}

/**
 * 初始化应用状态管理
 */
const initializeStores = async () => {
  try {
    // 初始化会话状态管理
    const sessionStore = useSessionStore()
    await sessionStore.initialize()

    // 初始化系统监控
    const systemStore = useSystemStore()
    await systemStore.initialize()

    // 初始化终端Store（包括会话恢复）
    const terminalStore = useTerminalStore()
    await terminalStore.initializeTerminalStore()
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
    await completionAPI.initEngine()

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
    // 并行初始化各个系统
    await Promise.allSettled([
      initializeStorageSystem(),
      initializeStores(),
      initializeSettings(),
      initializeServices(),
    ])

    // 设置窗口关闭监听器
    setupWindowCloseListener()
  } catch (error) {
    console.error('应用初始化过程中发生错误:', error)
  }
}

// 启动应用初始化
initializeApplication()

// ============================================================================
// 应用生命周期管理
// ============================================================================

/**
 * 应用关闭时的清理工作
 */
const handleAppClose = async () => {
  try {
    // 保存终端状态（这会自动同步并保存会话状态）
    const terminalStore = useTerminalStore()
    await terminalStore.saveSessionState()

    // 停止自动刷新
    const systemStore = useSystemStore()
    systemStore.stopAutoRefresh()

    // 停止会话自动保存
    const sessionStore = useSessionStore()
    sessionStore.stopAutoSave()
  } catch (error) {
    console.error('❌ [应用] 应用关闭清理失败:', error)
  }
}

/**
 * 设置 Tauri 窗口关闭事件监听器
 */
const setupWindowCloseListener = async () => {
  try {
    // 监听窗口关闭请求事件
    const unlisten = await getCurrentWebviewWindow().onCloseRequested(async event => {
      // 阻止默认关闭行为，这样我们可以先执行保存操作
      event.preventDefault()

      try {
        // 执行保存操作
        await handleAppClose()
      } catch (error) {
        console.error('❌ [应用] 保存失败:', error)
      }

      unlisten()
      await getCurrentWebviewWindow().close()
    })

    return unlisten
  } catch (error) {
    console.error(error)
  }
}
