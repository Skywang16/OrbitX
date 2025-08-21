/**
 * 主题业务领域类型定义
 */

// ===== 基础主题类型 =====

export type ThemeType = 'light' | 'dark' | 'auto'

export interface ThemeInfo {
  name: string
  themeType: string
  isCurrent: boolean
}

export interface ThemeConfig {
  autoSwitchTime: string
  terminalTheme: string
  lightTheme: string
  darkTheme: string
  followSystem: boolean
}

export interface ThemeConfigStatus {
  currentThemeName: string
  themeConfig: ThemeConfig
  isSystemDark: boolean | null
  availableThemes: ThemeInfo[]
}

// ===== 颜色配置类型 =====

export interface AnsiColors {
  black: string
  red: string
  green: string
  yellow: string
  blue: string
  magenta: string
  cyan: string
  white: string
}

export interface UIColors {
  bg_100: string
  bg_200: string
  bg_300: string
  bg_400: string
  bg_500: string
  bg_600: string
  bg_700: string
  text_100: string
  text_200: string
  text_300: string
  text_400: string
  text_500: string
  primary: string
  primary_hover: string
  primary_alpha: string
  success: string
  warning: string
  error: string
  info: string
  border_200: string
  border_300: string
  border_400: string
  hover: string
  active: string
  focus: string
  selection: string
}

// ===== 完整主题定义 =====

export interface SyntaxHighlight {
  keyword: string
  string: string
  comment: string
  number: string
  operator: string
  function: string
  variable: string
}

export interface Theme {
  name: string
  themeType: ThemeType
  ansi: AnsiColors
  bright: AnsiColors
  syntax: SyntaxHighlight
  ui: UIColors
}

// ===== 主题选项类型 =====

export interface ThemeOption {
  value: string
  label: string
  type: string
  isCurrent: boolean
}

// ===== 主题管理类型 =====

export interface ThemeValidationResult {
  isValid: boolean
  errors: string[]
  warnings: string[]
}

export interface ThemeLoadingState {
  loading: boolean
  error: string | null
  initialized: boolean
}
