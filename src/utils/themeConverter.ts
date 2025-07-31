/**
 * 主题转换工具
 *
 * 将项目的主题数据转换为 XTerm.js 可用的主题格式
 */

import type { Theme } from '@/api/config/types'

/**
 * 获取应用的主背景颜色
 * 从CSS变量中读取当前应用的背景颜色
 *
 * @returns 应用背景颜色字符串，如果无法获取则返回null
 */
function getAppBackgroundColor(): string | null {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return null
  }

  try {
    // 获取根元素的计算样式
    const rootElement = document.documentElement
    const computedStyle = window.getComputedStyle(rootElement)

    // 尝试获取应用背景颜色CSS变量
    const backgroundColor = computedStyle.getPropertyValue('--color-background').trim()

    if (backgroundColor && backgroundColor !== '') {
      return backgroundColor
    }

    // 如果没有找到CSS变量，尝试获取body的背景颜色
    const bodyStyle = window.getComputedStyle(document.body)
    const bodyBackground = bodyStyle.backgroundColor

    if (bodyBackground && bodyBackground !== 'rgba(0, 0, 0, 0)' && bodyBackground !== 'transparent') {
      return bodyBackground
    }

    return null
  } catch (error) {
    console.warn('获取应用背景颜色失败:', error)
    return null
  }
}

/**
 * XTerm.js 主题接口
 * 基于 XTerm.js 官方文档定义
 */
export interface XTermTheme {
  /** 前景色 */
  foreground?: string
  /** 背景色 */
  background?: string
  /** 光标颜色 */
  cursor?: string
  /** 光标强调色 */
  cursorAccent?: string
  /** 选择背景色 */
  selectionBackground?: string
  /** 选择前景色 */
  selectionForeground?: string
  /** 选择非活跃背景色 */
  selectionInactiveBackground?: string

  // ANSI 颜色 (0-7)
  /** ANSI 黑色 */
  black?: string
  /** ANSI 红色 */
  red?: string
  /** ANSI 绿色 */
  green?: string
  /** ANSI 黄色 */
  yellow?: string
  /** ANSI 蓝色 */
  blue?: string
  /** ANSI 洋红色 */
  magenta?: string
  /** ANSI 青色 */
  cyan?: string
  /** ANSI 白色 */
  white?: string

  // 明亮 ANSI 颜色 (8-15)
  /** 明亮黑色 */
  brightBlack?: string
  /** 明亮红色 */
  brightRed?: string
  /** 明亮绿色 */
  brightGreen?: string
  /** 明亮黄色 */
  brightYellow?: string
  /** 明亮蓝色 */
  brightBlue?: string
  /** 明亮洋红色 */
  brightMagenta?: string
  /** 明亮青色 */
  brightCyan?: string
  /** 明亮白色 */
  brightWhite?: string
}

/**
 * 将项目主题数据转换为 XTerm.js 主题格式
 *
 * @param theme 项目主题数据
 * @returns XTerm.js 主题对象
 */
export function convertThemeToXTerm(theme: Theme): XTermTheme {
  const { colors } = theme

  // 使用应用的主背景颜色，而不是终端特定的背景颜色
  // 这样可以确保终端背景与应用背景保持一致
  const appBackground = getAppBackgroundColor()

  return {
    // 基础颜色 - 使用应用背景颜色
    foreground: colors.foreground,
    background: appBackground || colors.background,
    cursor: colors.cursor,
    selectionBackground: colors.selection,

    // ANSI 标准颜色 (0-7)
    black: colors.ansi.black,
    red: colors.ansi.red,
    green: colors.ansi.green,
    yellow: colors.ansi.yellow,
    blue: colors.ansi.blue,
    magenta: colors.ansi.magenta,
    cyan: colors.ansi.cyan,
    white: colors.ansi.white,

    // ANSI 明亮颜色 (8-15)
    brightBlack: colors.bright.black,
    brightRed: colors.bright.red,
    brightGreen: colors.bright.green,
    brightYellow: colors.bright.yellow,
    brightBlue: colors.bright.blue,
    brightMagenta: colors.bright.magenta,
    brightCyan: colors.bright.cyan,
    brightWhite: colors.bright.white,
  }
}

/**
 * 创建默认的 XTerm.js 主题
 * 当无法获取主题数据时使用
 *
 * @returns 默认的 XTerm.js 主题对象
 */
export function createDefaultXTermTheme(): XTermTheme {
  // 尝试获取应用背景颜色，如果失败则使用默认深色背景
  const appBackground = getAppBackgroundColor() || '#1e1e1e'

  return {
    foreground: '#f0f0f0',
    background: appBackground,
    cursor: '#ffffff',
    selectionBackground: '#3391ff',

    // ANSI 标准颜色
    black: '#000000',
    red: '#cd3131',
    green: '#0dbc79',
    yellow: '#e5e510',
    blue: '#2472c8',
    magenta: '#bc3fbc',
    cyan: '#11a8cd',
    white: '#e5e5e5',

    // ANSI 明亮颜色
    brightBlack: '#666666',
    brightRed: '#f14c4c',
    brightGreen: '#23d18b',
    brightYellow: '#f5f543',
    brightBlue: '#3b8eea',
    brightMagenta: '#d670d6',
    brightCyan: '#29b8db',
    brightWhite: '#ffffff',
  }
}
