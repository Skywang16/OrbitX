import { createI18n } from 'vue-i18n'
import { storageApi } from '@/api/storage'
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
  try {
    const appConfig = await storageApi.getAppConfig()
    const savedLocale = appConfig?.language

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
  } catch (error) {
    console.warn('Failed to load locale from storage, using default:', error)
    // 使用浏览器语言作为回退
    const browserLang = navigator?.language?.toLowerCase() || ''
    const locale = browserLang.startsWith('zh') ? 'zh-CN' : 'en-US'
    i18n.global.locale.value = locale
  }
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

    // 保存到后端配置
    const currentConfig = await storageApi.getAppConfig()
    await storageApi.updateAppConfig({
      ...currentConfig,
      language: locale,
    })
  } catch (error) {
    console.error('Failed to save locale to backend:', error)
  }
}

export function getCurrentLocale() {
  return i18n.global.locale.value
}
