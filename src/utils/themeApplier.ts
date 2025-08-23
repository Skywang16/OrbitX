/**
 * 主题应用工具
 *
 * 负责将主题数据应用到前端界面，包括CSS变量更新和DOM属性设置
 */

import type { Theme, ThemeType } from '@/types'

/**
 * 将主题数据应用到前端界面
 *
 * @param theme 主题数据
 */
export const applyThemeToUI = (theme: Theme): void => {
  // 更新 data-theme 属性
  updateDataThemeAttribute(theme)

  // 更新 CSS 变量
  updateCSSVariables(theme)
}

/**
 * 更新 DOM 的 data-theme 属性
 *
 * @param theme 主题数据
 */
const updateDataThemeAttribute = (theme: Theme): void => {
  const root = document.documentElement

  // 根据主题类型设置 data-theme 属性
  let themeAttribute = theme.themeType

  // 特殊主题名称映射
  const themeNameMap: Record<string, string> = {
    'one-dark': 'one-dark',
    'solarized-light': 'light',
    'solarized-dark': 'dark',
    dracula: 'dracula',
    monokai: 'monokai',
  }

  // 如果有特殊映射，使用映射的名称
  if (themeNameMap[theme.name]) {
    themeAttribute = themeNameMap[theme.name] as ThemeType
  }

  root.setAttribute('data-theme', themeAttribute)
}

/**
 * 更新 CSS 变量 - 全新的层次系统
 *
 * @param theme 主题数据
 */
const updateCSSVariables = (theme: Theme): void => {
  const root = document.documentElement
  const style = root.style

  // 清除所有旧变量
  clearAllOldVariables(style)

  // 应用新的颜色层次系统
  if (theme.ui) {
    // 背景色层次
    style.setProperty('--bg-100', theme.ui.bg_100)
    style.setProperty('--bg-200', theme.ui.bg_200)
    style.setProperty('--bg-300', theme.ui.bg_300)
    style.setProperty('--bg-400', theme.ui.bg_400)
    style.setProperty('--bg-500', theme.ui.bg_500)
    style.setProperty('--bg-600', theme.ui.bg_600)
    style.setProperty('--bg-700', theme.ui.bg_700)

    // 边框层次
    style.setProperty('--border-200', theme.ui.border_200)
    style.setProperty('--border-300', theme.ui.border_300)
    style.setProperty('--border-400', theme.ui.border_400)

    // 文本层次
    style.setProperty('--text-100', theme.ui.text_100)
    style.setProperty('--text-200', theme.ui.text_200)
    style.setProperty('--text-300', theme.ui.text_300)
    style.setProperty('--text-400', theme.ui.text_400)
    style.setProperty('--text-500', theme.ui.text_500)

    // 状态颜色
    style.setProperty('--color-primary', theme.ui.primary)
    style.setProperty('--color-primary-hover', theme.ui.primary_hover)
    style.setProperty('--color-primary-alpha', theme.ui.primary_alpha)
    style.setProperty('--color-success', theme.ui.success)
    style.setProperty('--color-warning', theme.ui.warning)
    style.setProperty('--color-error', theme.ui.error)
    style.setProperty('--color-info', theme.ui.info)

    // 交互状态
    style.setProperty('--color-hover', theme.ui.hover)
    style.setProperty('--color-active', theme.ui.active)
    style.setProperty('--color-focus', theme.ui.focus)
    style.setProperty('--color-selection', theme.ui.selection)
  }

  // ANSI 颜色（用于终端和语法高亮）
  style.setProperty('--ansi-black', theme.ansi.black)
  style.setProperty('--ansi-red', theme.ansi.red)
  style.setProperty('--ansi-green', theme.ansi.green)
  style.setProperty('--ansi-yellow', theme.ansi.yellow)
  style.setProperty('--ansi-blue', theme.ansi.blue)
  style.setProperty('--ansi-magenta', theme.ansi.magenta)
  style.setProperty('--ansi-cyan', theme.ansi.cyan)
  style.setProperty('--ansi-white', theme.ansi.white)

  // 明亮 ANSI 颜色
  style.setProperty('--ansi-bright-black', theme.bright.black)
  style.setProperty('--ansi-bright-red', theme.bright.red)
  style.setProperty('--ansi-bright-green', theme.bright.green)
  style.setProperty('--ansi-bright-yellow', theme.bright.yellow)
  style.setProperty('--ansi-bright-blue', theme.bright.blue)
  style.setProperty('--ansi-bright-magenta', theme.bright.magenta)
  style.setProperty('--ansi-bright-cyan', theme.bright.cyan)
  style.setProperty('--ansi-bright-white', theme.bright.white)

  // 语法高亮颜色
  if (theme.syntax) {
    style.setProperty('--syntax-comment', theme.syntax.comment)
    style.setProperty('--syntax-keyword', theme.syntax.keyword)
    style.setProperty('--syntax-string', theme.syntax.string)
    style.setProperty('--syntax-number', theme.syntax.number)
    style.setProperty('--syntax-function', theme.syntax.function)
    style.setProperty('--syntax-variable', theme.syntax.variable)
    style.setProperty('--syntax-operator', theme.syntax.operator)
  }
}

/**
 * 清除所有旧变量
 */
const clearAllOldVariables = (style: CSSStyleDeclaration) => {
  // 删除所有旧变量
  const oldVariables = [
    // 旧的UI变量
    '--color-background-secondary',
    '--color-background-hover',
    '--color-border',
    '--border-color',
    '--border-color-hover',
    '--text-primary',
    '--text-secondary',
    '--text-muted',
    // 其他可能的旧变量
    '--color-accent',
    '--color-surface',
    '--color-foreground',
    '--color-cursor',
  ]

  oldVariables.forEach(variable => {
    style.removeProperty(variable)
  })
}

/**
 * 重置所有自定义 CSS 变量
 */
export const resetCSSVariables = (): void => {
  const root = document.documentElement
  const style = root.style

  // 移除所有CSS变量
  const allProperties = [
    // 新的层次变量
    '--bg-100',
    '--bg-200',
    '--bg-300',
    '--bg-400',
    '--bg-500',
    '--bg-600',
    '--bg-700',
    '--border-200',
    '--border-300',
    '--border-400',
    '--text-100',
    '--text-200',
    '--text-300',
    '--text-400',
    '--text-500',
    '--color-primary',
    '--color-primary-hover',
    '--color-primary-alpha',
    '--color-success',
    '--color-warning',
    '--color-error',
    '--color-info',
    '--color-hover',
    '--color-active',
    '--color-focus',
    '--color-selection',

    // ANSI 颜色
    '--ansi-black',
    '--ansi-red',
    '--ansi-green',
    '--ansi-yellow',
    '--ansi-blue',
    '--ansi-magenta',
    '--ansi-cyan',
    '--ansi-white',
    '--ansi-bright-black',
    '--ansi-bright-red',
    '--ansi-bright-green',
    '--ansi-bright-yellow',
    '--ansi-bright-blue',
    '--ansi-bright-magenta',
    '--ansi-bright-cyan',
    '--ansi-bright-white',

    // 语法高亮
    '--syntax-comment',
    '--syntax-keyword',
    '--syntax-string',
    '--syntax-number',
    '--syntax-function',
    '--syntax-variable',
    '--syntax-operator',
  ]

  allProperties.forEach(property => {
    style.removeProperty(property)
  })
}
