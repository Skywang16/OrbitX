import { completionAPI } from '@/api'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useTheme } from '@/composables/useTheme'
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

// 应用启动后初始化设置
const aiSettingsStore = useAISettingsStore()
aiSettingsStore.loadSettings().catch(error => {
  console.warn('AI设置初始化失败:', error)
})

// 初始化主题系统
const themeManager = useTheme()
themeManager.initialize().catch(error => {
  console.warn('主题系统初始化失败:', error)
})

// 应用启动后初始化补全引擎
completionAPI.initEngine().catch(() => {
  // 静默处理，使用本地补全作为后备
})
