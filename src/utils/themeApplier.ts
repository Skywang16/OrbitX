import type { Theme, ThemeType } from '@/types'
import { applyOpacityToColor, getCurrentOpacity } from './colorUtils'

let cachedTheme: Theme | null = null

export const applyThemeToUI = (theme: Theme): void => {
  cachedTheme = theme
  updateDataThemeAttribute(theme)
  updateCSSVariables(theme)
}

const updateDataThemeAttribute = (theme: Theme): void => {
  const root = document.documentElement

  let themeAttribute = theme.themeType

  const themeNameMap: Record<string, string> = {
    'one-dark': 'one-dark',
    'solarized-light': 'light',
    'solarized-dark': 'dark',
    dracula: 'dracula',
    monokai: 'monokai',
  }

  if (themeNameMap[theme.name]) {
    themeAttribute = themeNameMap[theme.name] as ThemeType
  }

  root.setAttribute('data-theme', themeAttribute)
}

const updateCSSVariables = (theme: Theme): void => {
  const root = document.documentElement
  const style = root.style

  clearAllOldVariables(style)

  const opacity = getCurrentOpacity()

  if (theme.ui) {
    style.setProperty('--bg-100', applyOpacityToColor(theme.ui.bg_100, opacity))
    style.setProperty('--bg-200', applyOpacityToColor(theme.ui.bg_200, opacity))
    style.setProperty('--bg-300', applyOpacityToColor(theme.ui.bg_300, opacity))
    style.setProperty('--bg-400', applyOpacityToColor(theme.ui.bg_400, opacity))
    style.setProperty('--bg-500', applyOpacityToColor(theme.ui.bg_500, opacity))
    style.setProperty('--bg-600', applyOpacityToColor(theme.ui.bg_600, opacity))
    style.setProperty('--bg-700', applyOpacityToColor(theme.ui.bg_700, opacity))

    style.setProperty('--border-200', theme.ui.border_200)
    style.setProperty('--border-300', theme.ui.border_300)
    style.setProperty('--border-400', theme.ui.border_400)

    style.setProperty('--text-100', theme.ui.text_100)
    style.setProperty('--text-200', theme.ui.text_200)
    style.setProperty('--text-300', theme.ui.text_300)
    style.setProperty('--text-400', theme.ui.text_400)
    style.setProperty('--text-500', theme.ui.text_500)

    style.setProperty('--color-primary', theme.ui.primary)
    style.setProperty('--color-primary-hover', theme.ui.primary_hover)
    style.setProperty('--color-primary-alpha', applyOpacityToColor(theme.ui.primary_alpha, opacity))
    style.setProperty('--color-success', theme.ui.success)
    style.setProperty('--color-warning', theme.ui.warning)
    style.setProperty('--color-error', theme.ui.error)
    style.setProperty('--color-info', theme.ui.info)

    style.setProperty('--color-hover', applyOpacityToColor(theme.ui.hover, opacity))
    style.setProperty('--color-active', applyOpacityToColor(theme.ui.active, opacity))
    style.setProperty('--color-focus', theme.ui.focus)
    style.setProperty('--color-selection', theme.ui.selection)
  }

  style.setProperty('--ansi-black', theme.ansi.black)
  style.setProperty('--ansi-red', theme.ansi.red)
  style.setProperty('--ansi-green', theme.ansi.green)
  style.setProperty('--ansi-yellow', theme.ansi.yellow)
  style.setProperty('--ansi-blue', theme.ansi.blue)
  style.setProperty('--ansi-magenta', theme.ansi.magenta)
  style.setProperty('--ansi-cyan', theme.ansi.cyan)
  style.setProperty('--ansi-white', theme.ansi.white)

  style.setProperty('--ansi-bright-black', theme.bright.black)
  style.setProperty('--ansi-bright-red', theme.bright.red)
  style.setProperty('--ansi-bright-green', theme.bright.green)
  style.setProperty('--ansi-bright-yellow', theme.bright.yellow)
  style.setProperty('--ansi-bright-blue', theme.bright.blue)
  style.setProperty('--ansi-bright-magenta', theme.bright.magenta)
  style.setProperty('--ansi-bright-cyan', theme.bright.cyan)
  style.setProperty('--ansi-bright-white', theme.bright.white)

  if (theme.syntax) {
    style.setProperty('--syntax-comment', theme.syntax.comment)
    style.setProperty('--syntax-keyword', theme.syntax.keyword)
    style.setProperty('--syntax-string', theme.syntax.string)
    style.setProperty('--syntax-number', theme.syntax.number)
    style.setProperty('--syntax-function', theme.syntax.function)
    style.setProperty('--syntax-variable', theme.syntax.variable)
    style.setProperty('--syntax-type-name', theme.syntax.type_name)
    style.setProperty('--syntax-operator', theme.syntax.operator)
  }
}

const clearAllOldVariables = (style: CSSStyleDeclaration) => {
  const oldVariables = [
    '--color-background-secondary',
    '--color-background-hover',
    '--color-border',
    '--border-color',
    '--border-color-hover',
    '--text-primary',
    '--text-secondary',
    '--text-muted',
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
 * 应用背景透明度
 * 当透明度变化时,重新应用主题 CSS 变量
 */
export const applyBackgroundOpacity = (opacity: number): void => {
  if (!cachedTheme || !cachedTheme.ui) {
    return
  }
  void opacity
  // 重新应用 CSS 变量,getCurrentOpacity() 会读取最新的透明度值
  updateCSSVariables(cachedTheme)
}

export const resetCSSVariables = (): void => {
  const root = document.documentElement
  const style = root.style

  const allProperties = [
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

    '--syntax-comment',
    '--syntax-keyword',
    '--syntax-string',
    '--syntax-number',
    '--syntax-function',
    '--syntax-variable',
    '--syntax-type-name',
    '--syntax-operator',
  ]

  allProperties.forEach(property => {
    style.removeProperty(property)
  })
}
