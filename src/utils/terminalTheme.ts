/**
 * 终端主题管理工具
 *
 * 提供从配置系统获取终端主题的功能，替代原来的 CSS 主题系统
 */

import { getCurrentTheme } from '@/api/config/theme'
import { convertThemeToXTerm, createDefaultXTermTheme, type XTermTheme } from './themeConverter'

/**
 * 从配置系统获取当前终端主题
 *
 * 这个函数替代了原来的 getTerminalThemeFromCSS 函数，
 * 现在从配置文件而不是 CSS 获取主题数据
 *
 * @returns Promise<XTermTheme> XTerm.js 主题对象
 */
export const getTerminalThemeFromConfig = async (): Promise<XTermTheme> => {
  try {
    // 从配置系统获取当前主题数据
    const theme = await getCurrentTheme()

    // 转换为 XTerm.js 主题格式
    return convertThemeToXTerm(theme)
  } catch (error) {
    // 如果获取主题失败，返回默认主题
    return createDefaultXTermTheme()
  }
}
