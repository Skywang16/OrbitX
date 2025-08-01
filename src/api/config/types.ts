/**
 * 配置模块相关的类型定义
 */

// 重新导出配置相关的类型
export type {
  // AI配置
  AIConfig,
  APIConfig,
  AnsiColors,
  AppConfig,
  // 应用配置
  AppConfigApp,
  // 外观配置
  AppearanceConfig,
  BackupInfo,
  ChatConfig,
  ColorScheme,
  CompletionConfig,
  ConfigChangeEvent,
  ConfigChangeType,
  ConfigFileInfo,
  ConfigFileState,
  ConfigLoadingState,
  ConfigMetadata,
  ConfigUpdateOptions,
  ConfigValidationError,
  ConfigValidationResult,
  ConfigValidationWarning,
  CursorConfig,
  CursorStyle,
  FontConfig,
  FontStyle,
  FontWeight,
  ModelConfig,
  ModelParameters,
  ScrollingConfig,
  SelectionConfig,
  // 快捷键配置
  ShortcutsConfig,
  SyntaxHighlight,
  // 终端配置
  TerminalConfig,
  Theme,
  // 主题配置
  ThemeConfig,
  ThemeType,
  UIColors,
} from '../../components/settings/components/Config/types'

// 配置API错误类
export class ConfigApiError extends Error {
  constructor(
    message: string,
    public readonly cause?: unknown
  ) {
    super(message)
    this.name = 'ConfigApiError'
  }
}

// ===== 配置操作结果类型 =====

export interface ConfigOperationResult<T = void> {
  success: boolean
  data?: T
  error?: string
}

// ===== 配置部分更新类型 =====

export interface ConfigSectionUpdate<T = any> {
  section: string
  updates: Partial<T>
}

// ===== 主题相关类型 =====

// 重新导出主题相关类型
export type { ThemeConfigStatus, ThemeInfo, ThemeConfig, Theme, ThemeType } from '@/types/theme'
