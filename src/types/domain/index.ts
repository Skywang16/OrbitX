/**
 * 业务领域类型统一导出
 */

export * from './ai'
export * from './terminal'
export * from './storage'
export * from './shortcuts'
export * from './completion'

// UI和主题有Theme类型冲突，分别导出
export * from './ui'
export type {
  ThemeType,
  ThemeInfo,
  ThemeConfig,
  ThemeConfigStatus,
  AnsiColors,
  UIColors,
  Theme,
  ThemeOption,
  ThemeValidationResult,
  ThemeLoadingState,
} from './theme'
