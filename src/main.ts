import { completionAPI } from '@/api'
import { storage } from '@/api/storage'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useTheme } from '@/composables/useTheme'
import { useSessionStore } from '@/stores/session'
import { useSystemStore } from '@/stores/system'
import { useTerminalStore } from '@/stores/Terminal'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import './styles/variables.css'
import ui from './ui'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(router)
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
    console.log('存储系统缓存预加载完成')
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
    console.log('会话状态管理初始化完成')

    // 初始化系统监控
    const systemStore = useSystemStore()
    await systemStore.initialize()
    console.log('系统监控初始化完成')

    // 初始化终端Store（包括会话恢复）
    const terminalStore = useTerminalStore()
    await terminalStore.initializeTerminalStore()
    console.log('终端Store初始化完成')
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
    console.log('AI设置初始化完成')

    // 初始化主题系统
    const themeManager = useTheme()
    await themeManager.initialize()
    console.log('主题系统初始化完成')
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
    console.log('补全引擎初始化完成')
  } catch (error) {
    console.warn('补全引擎初始化失败，使用本地补全作为后备:', error)
  }
}

/**
 * 应用启动初始化
 */
const initializeApplication = async () => {
  console.log('开始初始化应用...')

  try {
    // 并行初始化各个系统
    await Promise.allSettled([
      initializeStorageSystem(),
      initializeStores(),
      initializeSettings(),
      initializeServices(),
    ])

    console.log('应用初始化完成')

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
    console.log('🔄 [应用] 开始应用关闭清理...')

    // 保存终端状态（这会自动同步并保存会话状态）
    const terminalStore = useTerminalStore()
    await terminalStore.saveSessionState()

    // 停止自动刷新
    const systemStore = useSystemStore()
    systemStore.stopAutoRefresh()

    // 停止会话自动保存
    const sessionStore = useSessionStore()
    sessionStore.stopAutoSave()

    console.log('✅ [应用] 应用关闭清理完成')
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
      console.log('🔄 [应用] 收到窗口关闭请求')

      // 阻止默认关闭行为，这样我们可以先执行保存操作
      event.preventDefault()

      try {
        // 执行保存操作
        await handleAppClose()
        console.log('✅ [应用] 保存完成')
      } catch (error) {
        console.error('❌ [应用] 保存失败:', error)
      }

      // 但要避免循环，所以我们移除监听器后再关闭
      unlisten()
      await getCurrentWebviewWindow().close()
    })

    console.log('✅ [应用] 窗口关闭监听器已设置')

    // 返回取消监听的函数，以便在需要时清理
    return unlisten
  } catch (error) {
    console.error('❌ [应用] 设置窗口关闭监听器失败:', error)
  }
}
