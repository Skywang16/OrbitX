/**
 * 主题应用工具
 *
 * 负责将主题数据应用到前端界面，包括CSS变量更新和DOM属性设置
 */

import type { Theme } from '@/api/config/types'

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

  console.log(`已应用主题: ${theme.name} (${theme.themeType})`)
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
    themeAttribute = themeNameMap[theme.name]
  }

  root.setAttribute('data-theme', themeAttribute)
}

/**
 * 更新 CSS 变量
 *
 * @param theme 主题数据
 */
const updateCSSVariables = (theme: Theme): void => {
  const root = document.documentElement
  const style = root.style

  // 基础颜色
  style.setProperty('--color-background', theme.colors.background)
  style.setProperty('--color-foreground', theme.colors.foreground)
  style.setProperty('--color-cursor', theme.colors.cursor)
  style.setProperty('--color-selection', theme.colors.selection)

  // UI 颜色
  if (theme.ui) {
    style.setProperty('--color-background-secondary', theme.ui.background_secondary || theme.colors.background)
    style.setProperty('--color-background-hover', theme.ui.background_hover || theme.colors.background)
    style.setProperty('--color-border', theme.ui.border || 'rgba(255, 255, 255, 0.1)')
    style.setProperty('--border-color', theme.ui.border || 'rgba(255, 255, 255, 0.1)')

    // 主色调
    if (theme.ui.primary) {
      style.setProperty('--color-primary', theme.ui.primary)
      style.setProperty('--color-primary-hover', adjustColorBrightness(theme.ui.primary, -10))
      style.setProperty('--color-primary-alpha', addAlphaToColor(theme.ui.primary, 0.1))
    }

    // 文本颜色
    style.setProperty('--text-primary', theme.colors.foreground)
    style.setProperty('--text-secondary', adjustColorBrightness(theme.colors.foreground, -20))
    style.setProperty('--text-muted', adjustColorBrightness(theme.colors.foreground, -40))
  }

  // ANSI 颜色（用于终端和语法高亮）
  const ansiColors = theme.colors.ansi
  style.setProperty('--ansi-black', ansiColors.black)
  style.setProperty('--ansi-red', ansiColors.red)
  style.setProperty('--ansi-green', ansiColors.green)
  style.setProperty('--ansi-yellow', ansiColors.yellow)
  style.setProperty('--ansi-blue', ansiColors.blue)
  style.setProperty('--ansi-magenta', ansiColors.magenta)
  style.setProperty('--ansi-cyan', ansiColors.cyan)
  style.setProperty('--ansi-white', ansiColors.white)

  // 明亮 ANSI 颜色
  const brightColors = theme.colors.bright
  style.setProperty('--ansi-bright-black', brightColors.black)
  style.setProperty('--ansi-bright-red', brightColors.red)
  style.setProperty('--ansi-bright-green', brightColors.green)
  style.setProperty('--ansi-bright-yellow', brightColors.yellow)
  style.setProperty('--ansi-bright-blue', brightColors.blue)
  style.setProperty('--ansi-bright-magenta', brightColors.magenta)
  style.setProperty('--ansi-bright-cyan', brightColors.cyan)
  style.setProperty('--ansi-bright-white', brightColors.white)
}

/**
 * 调整颜色亮度
 *
 * @param color 颜色值 (hex)
 * @param percent 亮度调整百分比 (-100 到 100)
 * @returns 调整后的颜色值
 */
const adjustColorBrightness = (color: string, percent: number): string => {
  // 简单的颜色亮度调整实现
  // 这里可以使用更复杂的颜色处理库，但为了减少依赖，使用简单实现

  if (!color.startsWith('#')) {
    return color // 如果不是 hex 颜色，直接返回
  }

  const hex = color.slice(1)
  const num = parseInt(hex, 16)

  let r = (num >> 16) + Math.round((255 * percent) / 100)
  let g = ((num >> 8) & 0x00ff) + Math.round((255 * percent) / 100)
  let b = (num & 0x0000ff) + Math.round((255 * percent) / 100)

  r = Math.max(0, Math.min(255, r))
  g = Math.max(0, Math.min(255, g))
  b = Math.max(0, Math.min(255, b))

  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, '0')}`
}

/**
 * 为颜色添加透明度
 *
 * @param color 颜色值 (hex)
 * @param alpha 透明度 (0-1)
 * @returns rgba 颜色值
 */
const addAlphaToColor = (color: string, alpha: number): string => {
  if (!color.startsWith('#')) {
    return color
  }

  const hex = color.slice(1)
  const num = parseInt(hex, 16)

  const r = num >> 16
  const g = (num >> 8) & 0x00ff
  const b = num & 0x0000ff

  return `rgba(${r}, ${g}, ${b}, ${alpha})`
}

/**
 * 重置所有自定义 CSS 变量
 */
export const resetCSSVariables = (): void => {
  const root = document.documentElement
  const style = root.style

  // 移除所有自定义设置的 CSS 变量
  const customProperties = [
    '--color-background',
    '--color-foreground',
    '--color-cursor',
    '--color-selection',
    '--color-background-secondary',
    '--color-background-hover',
    '--color-border',
    '--border-color',
    '--color-primary',
    '--color-primary-hover',
    '--color-primary-alpha',
    '--text-primary',
    '--text-secondary',
    '--text-muted',
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
  ]

  customProperties.forEach(property => {
    style.removeProperty(property)
  })
}
