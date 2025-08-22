/**
 * 快捷键业务领域类型定义
 */

// ===== 基础快捷键类型 =====

export type ShortcutAction = string

export interface ShortcutBinding {
  key: string
  modifiers: string[]
  action: ShortcutAction
}

export interface ShortcutsConfig {
  global: ShortcutBinding[]
  terminal: ShortcutBinding[]
  custom: ShortcutBinding[]
}

export enum ShortcutCategory {
  Global = 'Global',
  Terminal = 'Terminal',
  Custom = 'Custom',
}

export enum Platform {
  Windows = 'Windows',
  MacOS = 'MacOS',
  Linux = 'Linux',
}

// ===== 验证和冲突检测类型 =====

export interface ShortcutValidationError {
  error_type: string
  message: string
  shortcut?: ShortcutBinding
}

export interface ShortcutValidationWarning {
  warning_type: string
  message: string
  shortcut?: ShortcutBinding
}

export interface ShortcutValidationResult {
  is_valid: boolean
  errors: ShortcutValidationError[]
  warnings: ShortcutValidationWarning[]
}

export interface ConflictingShortcut {
  category: string
  binding: ShortcutBinding
}

export interface ShortcutConflict {
  key_combination: string
  conflicting_shortcuts: ConflictingShortcut[]
}

export interface ConflictDetectionResult {
  has_conflicts: boolean
  conflicts: ShortcutConflict[]
}

// ===== 统计和搜索类型 =====

export interface ShortcutStatistics {
  global_count: number
  terminal_count: number
  custom_count: number
  total_count: number
}

export interface ShortcutSearchOptions {
  query?: string
  categories?: ShortcutCategory[]
  key?: string
  modifiers?: string[]
  action?: string
}

// ===== 操作选项类型 =====

export interface ShortcutOperationOptions {
  validate?: boolean
  checkConflicts?: boolean
  autoSave?: boolean
}

export interface ShortcutFormatOptions {
  platform?: Platform
  useSymbols?: boolean
  separator?: string
}

// ===== 前端扩展类型 =====

export type SupportedShortcutAction =
  | 'copy_to_clipboard'
  | 'paste_from_clipboard'
  | 'new_tab'
  | 'close_tab'
  | 'switch_to_tab_1'
  | 'switch_to_tab_2'
  | 'switch_to_tab_3'
  | 'switch_to_tab_4'
  | 'switch_to_tab_5'
  | 'switch_to_last_tab'
  | 'accept_completion'

export interface ShortcutExecutionResult {
  success: boolean
  actionName: string
  keyCombo: string
  frontendResult?: boolean
  backendResult?: any
  error?: string
}

export interface ShortcutListenerConfig {
  debugMode?: boolean
  autoStart?: boolean
  priority?: number
}

// ===== 事件类型 =====

export interface ShortcutEvent {
  type: 'shortcut_triggered' | 'shortcut_conflict' | 'shortcut_updated'
  shortcut?: ShortcutBinding
  data?: any
  timestamp: number
}

export type ShortcutEventListener = (event: ShortcutEvent) => void
