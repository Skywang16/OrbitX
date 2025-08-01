/**
 * 主题相关类型定义
 * 统一管理所有主题相关的类型，避免重复定义
 */

// ===== 基础主题类型 =====

/**
 * 主题类型枚举
 */
export type ThemeType = 'light' | 'dark' | 'auto'

/**
 * 主题信息
 */
export interface ThemeInfo {
  /** 主题名称 */
  name: string
  /** 主题类型 */
  themeType: string
  /** 是否为当前主题 */
  isCurrent: boolean
}

/**
 * 主题配置
 */
export interface ThemeConfig {
  /** 自动切换时间 */
  autoSwitchTime: string
  /** 终端主题名称，引用themes/目录下的文件 */
  terminalTheme: string
  /** 浅色主题 */
  lightTheme: string
  /** 深色主题 */
  darkTheme: string
  /** 跟随系统主题 */
  followSystem: boolean
}

/**
 * 主题配置状态
 */
export interface ThemeConfigStatus {
  /** 当前使用的主题名称 */
  currentThemeName: string
  /** 主题配置 */
  themeConfig: ThemeConfig
  /** 系统是否为深色模式 */
  isSystemDark: boolean | null
  /** 所有可用主题 */
  availableThemes: ThemeInfo[]
}

// ===== 颜色相关类型 =====

/**
 * ANSI 颜色配置
 */
export interface AnsiColors {
  black: string
  red: string
  green: string
  yellow: string
  blue: string
  magenta: string
  cyan: string
  white: string
  brightBlack: string
  brightRed: string
  brightGreen: string
  brightYellow: string
  brightBlue: string
  brightMagenta: string
  brightCyan: string
  brightWhite: string
}

/**
 * 颜色方案
 */
export interface ColorScheme {
  background: string
  foreground: string
  cursor: string
  selection: string
  ansi: AnsiColors
}

/**
 * 语法高亮配置
 */
export interface SyntaxHighlight {
  keyword: string
  string: string
  comment: string
  number: string
  operator: string
  function: string
  variable: string
}

/**
 * UI 颜色配置
 */
export interface UIColors {
  primary: string
  secondary: string
  accent: string
  background: string
  surface: string
  error: string
  warning: string
  info: string
  success: string
}

// ===== 完整主题定义 =====

/**
 * 完整的主题定义
 */
export interface Theme {
  /** 主题名称 */
  name: string
  /** 主题类型 */
  themeType: ThemeType
  /** 颜色配置 */
  colors: ColorScheme
  /** 语法高亮 */
  syntax: SyntaxHighlight
  /** UI 颜色 */
  ui: UIColors
}

// ===== 主题选项类型 =====

/**
 * 主题选项（用于UI显示）
 */
export interface ThemeOption {
  /** 主题值 */
  value: string
  /** 显示标签 */
  label: string
  /** 主题类型 */
  type: string
  /** 是否为当前主题 */
  isCurrent: boolean
}

// ===== 主题管理相关类型 =====

/**
 * 主题验证结果
 */
export interface ThemeValidationResult {
  /** 是否有效 */
  isValid: boolean
  /** 错误信息 */
  errors: string[]
  /** 警告信息 */
  warnings: string[]
}

/**
 * 主题加载状态
 */
export interface ThemeLoadingState {
  /** 是否正在加载 */
  loading: boolean
  /** 错误信息 */
  error: string | null
  /** 是否已初始化 */
  initialized: boolean
}
