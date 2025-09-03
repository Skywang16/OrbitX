/**
 * 快捷键系统类型定义
 *
 * 定义快捷键相关的TypeScript类型，与后端Rust类型保持一致
 */

/**
 * 快捷键分类
 */
export enum ShortcutCategory {
  Global = 'global',
  Terminal = 'terminal',
  Tab = 'tab',
  AI = 'ai',
  Custom = 'custom',
}

/**
 * 快捷键动作类型
 */
export type ShortcutAction = string

/**
 * 快捷键绑定
 */
export interface ShortcutBinding {
  /** 按键 */
  key: string
  /** 修饰键列表 */
  modifiers: string[]
  /** 动作 */
  action: ShortcutAction
}

/**
 * 快捷键配置
 */
export type ShortcutsConfig = ShortcutBinding[]

/**
 * 支持的平台类型
 */
export enum Platform {
  Windows = 'Windows',
  MacOS = 'MacOS',
  Linux = 'Linux',
}

/**
 * 快捷键验证错误
 */
export interface ShortcutValidationError {
  /** 错误类型 */
  error_type: string
  /** 错误消息 */
  message: string
  /** 相关的快捷键绑定（可选） */
  shortcut?: ShortcutBinding
}

/**
 * 快捷键验证警告
 */
export interface ShortcutValidationWarning {
  /** 警告类型 */
  warning_type: string
  /** 警告消息 */
  message: string
  /** 相关的快捷键绑定（可选） */
  shortcut?: ShortcutBinding
}

/**
 * 快捷键验证结果
 */
export interface ShortcutValidationResult {
  /** 是否通过验证 */
  is_valid: boolean
  /** 验证错误列表 */
  errors: ShortcutValidationError[]
  /** 验证警告列表 */
  warnings: ShortcutValidationWarning[]
}

/**
 * 冲突的快捷键信息
 */
export interface ConflictingShortcut {
  /** 快捷键类别 */
  category: string
  /** 快捷键绑定 */
  binding: ShortcutBinding
}

/**
 * 快捷键冲突
 */
export interface ShortcutConflict {
  /** 冲突的快捷键组合 */
  key_combination: string
  /** 冲突的快捷键绑定列表 */
  conflicting_shortcuts: ConflictingShortcut[]
}

/**
 * 快捷键冲突检测结果
 */
export interface ConflictDetectionResult {
  /** 是否有冲突 */
  has_conflicts: boolean
  /** 冲突列表 */
  conflicts: ShortcutConflict[]
}

/**
 * 快捷键统计信息
 */
export interface ShortcutStatistics {
  /** 全局快捷键数量 */
  global_count: number
  /** 终端快捷键数量 */
  terminal_count: number
  /** 自定义快捷键数量 */
  custom_count: number
  /** 总快捷键数量 */
  total_count: number
}

/**
 * 快捷键API错误类型
 */
export class ShortcutApiError extends Error {
  constructor(
    message: string,
    public readonly code?: string
  ) {
    super(message)
    this.name = 'ShortcutApiError'
  }
}

/**
 * 快捷键操作选项
 */
export interface ShortcutOperationOptions {
  /** 是否验证快捷键 */
  validate?: boolean
  /** 是否检测冲突 */
  checkConflicts?: boolean
  /** 是否自动保存 */
  autoSave?: boolean
}

/**
 * 快捷键导入导出选项
 */
export interface ShortcutImportExportOptions {
  /** 导入/导出的类别 */
  categories?: ShortcutCategory[]
  /** 是否覆盖现有配置 */
  overwrite?: boolean
  /** 是否备份现有配置 */
  backup?: boolean
}

/**
 * 快捷键搜索选项
 */
export interface ShortcutSearchOptions {
  /** 搜索关键词 */
  query?: string
  /** 搜索的类别 */
  categories?: ShortcutCategory[]
  /** 搜索的按键 */
  key?: string
  /** 搜索的修饰键 */
  modifiers?: string[]
  /** 搜索的动作 */
  action?: string
}

/**
 * 快捷键搜索结果
 */
export interface ShortcutSearchResult {
  /** 匹配的快捷键 */
  shortcuts: Array<{
    category: ShortcutCategory
    index: number
    binding: ShortcutBinding
  }>
  /** 总匹配数量 */
  total: number
}

/**
 * 快捷键格式化选项
 */
export interface ShortcutFormatOptions {
  /** 目标平台 */
  platform?: Platform
  /** 是否使用符号 */
  useSymbols?: boolean
  /** 分隔符 */
  separator?: string
}

/**
 * 快捷键事件监听器类型
 */
export type ShortcutEventListener = (event: ShortcutEvent) => void

/**
 * 快捷键事件
 */
export interface ShortcutEvent {
  /** 事件类型 */
  type: 'shortcut_triggered' | 'shortcut_conflict' | 'shortcut_updated'
  /** 相关的快捷键绑定 */
  shortcut?: ShortcutBinding
  /** 事件数据 */
  data?: any
  /** 时间戳 */
  timestamp: number
}
