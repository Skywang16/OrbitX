import type { Theme } from '@/types'

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
export const convertThemeToXTerm = (theme: Theme): XTermTheme => {
  return {
    foreground: theme.ui.text_200,

    background: 'transparent',
    cursor: theme.ui.text_100,
    selectionBackground: theme.ui.selection,

    black: theme.ansi.black,
    red: theme.ansi.red,
    green: theme.ansi.green,
    yellow: theme.ansi.yellow,
    blue: theme.ansi.blue,
    magenta: theme.ansi.magenta,
    cyan: theme.ansi.cyan,
    white: theme.ansi.white,

    brightBlack: theme.bright.black,
    brightRed: theme.bright.red,
    brightGreen: theme.bright.green,
    brightYellow: theme.bright.yellow,
    brightBlue: theme.bright.blue,
    brightMagenta: theme.bright.magenta,
    brightCyan: theme.bright.cyan,
    brightWhite: theme.bright.white,
  }
}

/**
 * 创建默认的 XTerm.js 主题
 * 当无法获取主题数据时使用
 *
 * @returns 默认的 XTerm.js 主题对象
 */
export const createDefaultXTermTheme = (): XTermTheme => {
  return {
    foreground: '#f0f0f0',
    background: 'transparent',
    cursor: '#ffffff',
    selectionBackground: '#3391ff',

    black: '#000000',
    red: '#cd3131',
    green: '#0dbc79',
    yellow: '#e5e510',
    blue: '#2472c8',
    magenta: '#bc3fbc',
    cyan: '#11a8cd',
    white: '#e5e5e5',

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
