import { createI18n } from 'vue-i18n'
import { storageApi } from '@/api/storage'
import { invoke } from '@/utils/request'
import { listen } from '@tauri-apps/api/event'
import { useSessionStore } from '@/stores/session'
import zh from './locales/zh.json'
import en from './locales/en.json'

export type MessageLanguages = keyof typeof zh

const messages = {
  'zh-CN': zh,
  'en-US': en,
}

export const i18n = createI18n({
  legacy: false,
  locale: 'en-US', // 先设置默认值，避免初始化问题
  fallbackLocale: 'en-US',
  messages,
  globalInjection: true,
  silentFallbackWarn: true,
  silentTranslationWarn: true,
})

// 异步初始化语言设置
export async function initLocale() {
  // 优先从后端语言管理器获取（与后端 i18n 保持一致）
  let savedLocale: string | undefined
  const appLanguage = await invoke<string>('language_get_app_language').catch(error => {
    console.warn('Failed to get app language:', error)
    return undefined
  })
  savedLocale = appLanguage

  if (!savedLocale) {
    const appConfig = await storageApi.getAppConfig()
    savedLocale = appConfig?.language
  }

  let locale = 'en-US'
  if (savedLocale && (savedLocale === 'zh-CN' || savedLocale === 'en-US')) {
    locale = savedLocale
  } else {
    // 回退到浏览器语言
    const browserLang = navigator?.language?.toLowerCase() || ''
    if (browserLang.startsWith('zh')) {
      locale = 'zh-CN'
    }
  }

  i18n.global.locale.value = locale as 'zh-CN' | 'en-US'

  // 监听后端语言变化事件，保持回显同步
  await listen<string>('language-changed', event => {
    const next = event.payload
    if (next === 'zh-CN' || next === 'en-US') {
      i18n.global.locale.value = next
      const sessionStore = useSessionStore()
      sessionStore.updateUiState({ language: next })
    }
  }).catch(error => {
    console.warn('Failed to setup language listener:', error)
  })
}

// 切换语言函数
export async function setLocale(locale: string) {
  // 验证locale参数
  if (!locale || typeof locale !== 'string') {
    console.error('Invalid locale type:', typeof locale, locale)
    return
  }

  // 确保locale是支持的语言
  if (locale !== 'zh-CN' && locale !== 'en-US') {
    console.error('Unsupported locale:', locale)
    return
  }

  try {
    i18n.global.locale.value = locale as 'zh-CN' | 'en-US'

    // 通知后端语言管理器（写配置并广播事件）
    await invoke<void>('language_set_app_language', { language: locale }).catch(error => {
      console.warn('Failed to set app language:', error)
    })

    // 立即保存会话状态
    const sessionStore = useSessionStore()
    sessionStore.updateUiState({ language: locale })
  } catch (error) {
    console.error('Failed to save locale to backend:', error)
  }
}

export function getCurrentLocale() {
  return i18n.global.locale.value
}
