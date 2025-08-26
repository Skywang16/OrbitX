/**
 * 快捷键系统常量定义
 */

/**
 * 支持的修饰键
 */
export const MODIFIER_KEYS = {
  CMD: 'cmd',
  CTRL: 'ctrl',
  ALT: 'alt',
  SHIFT: 'shift',
  META: 'meta',
} as const

/**
 * 按键标准化映射
 */
export const KEY_NORMALIZATION_MAP: Record<string, string> = {
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowLeft: 'left',
  ArrowRight: 'right',
  ' ': 'space',
  Enter: 'return',
  Escape: 'esc',
  Backspace: 'backspace',
  Delete: 'delete',
  Tab: 'tab',
}

/**
 * Shortcut action definitions
 * Action key -> action name mapping (for internal use)
 * Display names should be handled by i18n system
 */
export const SHORTCUT_ACTIONS = {
  // Global actions
  copy_to_clipboard: 'copy_to_clipboard',
  paste_from_clipboard: 'paste_from_clipboard',
  terminal_search: 'terminal_search',
  open_settings: 'open_settings',

  // Tab management
  new_tab: 'new_tab',
  close_tab: 'close_tab',
  switch_to_tab_1: 'switch_to_tab_1',
  switch_to_tab_2: 'switch_to_tab_2',
  switch_to_tab_3: 'switch_to_tab_3',
  switch_to_tab_4: 'switch_to_tab_4',
  switch_to_tab_5: 'switch_to_tab_5',
  switch_to_last_tab: 'switch_to_last_tab',

  // Terminal functions
  clear_terminal: 'clear_terminal',
  accept_completion: 'accept_completion',
  increase_font_size: 'increase_font_size',
  decrease_font_size: 'decrease_font_size',
} as const
