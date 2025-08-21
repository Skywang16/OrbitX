/**
 * 快捷键系统前端类型定义
 *
 * 定义前端快捷键系统使用的类型
 */

// 重新导出API类型
export type * from '@/api/shortcuts/types'

/**
 * 快捷键监听器配置
 */
export interface ShortcutListenerConfig {
  /** 是否启用调试模式 */
  debugMode?: boolean
  /** 是否自动启动监听 */
  autoStart?: boolean
  /** 监听器优先级 */
  priority?: number
}

/**
 * 快捷键执行结果
 */
export interface ShortcutExecutionResult {
  /** 是否执行成功 */
  success: boolean
  /** 执行的动作名称 */
  actionName: string
  /** 按键组合 */
  keyCombo: string
  /** 前端执行结果 */
  frontendResult?: boolean
  /** 后端执行结果 */
  backendResult?: any
  /** 错误信息 */
  error?: string
}

/**
 * 支持的快捷键动作类型
 */
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

/**
 * 快捷键事件数据
 */
export interface ShortcutEventData {
  /** 原始键盘事件 */
  originalEvent: KeyboardEvent
  /** 标准化的按键组合 */
  keyCombo: string
  /** 匹配的快捷键配置 */
  shortcut: any
  /** 是否阻止默认行为 */
  preventDefault: boolean
}
