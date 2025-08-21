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
 * 快捷键动作定义
 * 动作键 -> 中文名称的简单映射
 */
export const SHORTCUT_ACTIONS = {
  // 全局动作
  copy_to_clipboard: '复制到剪贴板',
  paste_from_clipboard: '从剪贴板粘贴',
  terminal_search: '终端搜索',
  open_settings: '打开设置',
  toggle_theme: '切换主题',

  // 标签页管理
  new_tab: '新建标签页',
  close_tab: '关闭标签页',
  switch_to_tab_1: '切换到标签页 1',
  switch_to_tab_2: '切换到标签页 2',
  switch_to_tab_3: '切换到标签页 3',
  switch_to_tab_4: '切换到标签页 4',
  switch_to_tab_5: '切换到标签页 5',
  switch_to_last_tab: '切换到最后标签页',

  // 终端功能
  clear_terminal: '清空终端',
  accept_completion: '接受补全',
  increase_font_size: '增大字体',
  decrease_font_size: '减小字体',
} as const
