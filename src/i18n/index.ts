import { createI18n } from 'vue-i18n'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@/utils/request'
import zh from './locales/zh.json'
import en from './locales/en.json'

export type SupportedLanguage = 'zh-CN' | 'en-US'

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

const getPersistedLanguage = async (): Promise<SupportedLanguage | null> => {
  const lang = await invoke<string>('language_get_app_language').catch(() => null)
  return lang === 'zh-CN' || lang === 'en-US' ? lang : null
}

const persistLanguage = async (language: SupportedLanguage): Promise<void> => {
  await invoke<void>('language_set_app_language', { language })
}

// 异步初始化语言设置
export const initLocale = async () => {
  let locale: SupportedLanguage | null = null
  try {
    locale = await getPersistedLanguage()
  } catch {
    locale = null
  }

  if (!locale) {
    const browserLang = navigator?.language?.toLowerCase() || ''
    locale = browserLang.startsWith('zh') ? 'zh-CN' : 'en-US'
  }

  i18n.global.locale.value = locale

  // 监听后端语言变化事件，保持回显同步
  await listen<string>('language-changed', event => {
    const next = event.payload
    if (next === 'zh-CN' || next === 'en-US') {
      i18n.global.locale.value = next
    }
  }).catch(error => {
    console.warn('Failed to setup language listener:', error)
  })
}

// 切换语言函数
export const setLocale = async (locale: string) => {
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
    await persistLanguage(locale as SupportedLanguage)
  } catch (error) {
    console.error('Failed to save locale to backend:', error)
  }
}

export const getCurrentLocale = () => {
  return i18n.global.locale.value
}
