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
 * 快捷键动作常量
 */
export const SHORTCUT_ACTIONS = {
  // 全局动作
  COPY_TO_CLIPBOARD: 'copy_to_clipboard',
  PASTE_FROM_CLIPBOARD: 'paste_from_clipboard',

  // 标签页管理
  NEW_TAB: 'new_tab',
  CLOSE_TAB: 'close_tab',
  SWITCH_TO_TAB_1: 'switch_to_tab_1',
  SWITCH_TO_TAB_2: 'switch_to_tab_2',
  SWITCH_TO_TAB_3: 'switch_to_tab_3',
  SWITCH_TO_TAB_4: 'switch_to_tab_4',
  SWITCH_TO_TAB_5: 'switch_to_tab_5',
  SWITCH_TO_LAST_TAB: 'switch_to_last_tab',

  // 补全功能
  ACCEPT_COMPLETION: 'accept_completion',
} as const

/**
 * 不阻止默认行为的动作
 * 这些动作需要保持浏览器的原生行为
 */
export const NON_BLOCKING_ACTIONS = new Set(['copy_to_clipboard', 'paste_from_clipboard'])

/**
 * 需要终端上下文的动作
 */
export const TERMINAL_CONTEXT_ACTIONS = new Set([SHORTCUT_ACTIONS.ACCEPT_COMPLETION, SHORTCUT_ACTIONS.CLOSE_TAB])

/**
 * 快捷键动作中文名称映射
 */
export const SHORTCUT_ACTION_NAMES: Record<string, string> = {
  // 全局动作
  copy_to_clipboard: '复制到剪贴板',
  paste_from_clipboard: '从剪贴板粘贴',
  search_forward: '向前搜索',
  search_backward: '向后搜索',
  toggle_fullscreen: '切换全屏',
  quit_application: '退出应用',

  // 终端动作
  new_tab: '新建标签页',
  close_tab: '关闭标签页',
  new_window: '新建窗口',
  close_window: '关闭窗口',
  split_vertical: '垂直分割',
  split_horizontal: '水平分割',
  clear_terminal: '清空终端',
  scroll_up: '向上滚动',
  scroll_down: '向下滚动',
  next_tab: '下一个标签页',
  previous_tab: '上一个标签页',

  // 标签页切换
  switch_to_tab_1: '切换到标签页 1',
  switch_to_tab_2: '切换到标签页 2',
  switch_to_tab_3: '切换到标签页 3',
  switch_to_tab_4: '切换到标签页 4',
  switch_to_tab_5: '切换到标签页 5',
  switch_to_last_tab: '切换到最后标签页',

  // AI相关动作
  toggle_ai_chat: '切换AI聊天',
  send_to_ai: '发送到AI',
  clear_ai_chat: '清空AI聊天',

  // 设置相关动作
  open_settings: '打开设置',
  toggle_theme: '切换主题',

  // 补全功能
  accept_completion: '接受补全',

  // 字体大小调整（如果系统中有实现）
  increase_font_size: '增大字体',
  decrease_font_size: '减小字体',
  reset_font_size: '重置字体大小',

  // 其他常见动作
  run_command: '运行命令',
  execute_script: '执行脚本',
  open_url: '打开链接',
}

/**
 * 获取快捷键动作的中文名称
 */
export function getActionDisplayName(action: string): string {
  return SHORTCUT_ACTION_NAMES[action] || action
}
