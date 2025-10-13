/**
 * 快捷键系统工具函数
 */

import { KEY_NORMALIZATION_MAP, MODIFIER_KEYS } from './constants'
import type { ShortcutBinding } from '@/types'

/**
 * 标准化按键名称
 */
export const normalizeKey = (key: string): string => {
  return KEY_NORMALIZATION_MAP[key] || key.toLowerCase()
}

/**
 * 获取事件的修饰键
 */
export const getEventModifiers = (event: KeyboardEvent): string[] => {
  const modifiers: string[] = []

  if (event.ctrlKey) modifiers.push(MODIFIER_KEYS.CTRL)
  if (event.altKey) modifiers.push(MODIFIER_KEYS.ALT)
  if (event.shiftKey) modifiers.push(MODIFIER_KEYS.SHIFT)
  if (event.metaKey || event.ctrlKey) {
    // macOS使用cmd，其他平台使用ctrl
    if (navigator.platform.includes('Mac')) {
      if (event.metaKey) modifiers.push(MODIFIER_KEYS.CMD)
    } else {
      if (event.ctrlKey) modifiers.push(MODIFIER_KEYS.CMD)
    }
  }

  return modifiers.sort()
}

/**
 * 标准化修饰键数组
 */
export const normalizeModifiers = (modifiers: string[]): string[] => {
  return modifiers.map(m => m.toLowerCase()).sort()
}

/**
 * 比较修饰键是否相等
 */
export const areModifiersEqual = (mods1: string[], mods2: string[]): boolean => {
  if (mods1.length !== mods2.length) return false

  for (let i = 0; i < mods1.length; i++) {
    if (mods1[i] !== mods2[i]) return false
  }

  return true
}

/**
 * 格式化按键组合为字符串
 */
export const formatKeyCombo = (event: KeyboardEvent): string => {
  const modifiers = getEventModifiers(event)
  const key = normalizeKey(event.key)

  if (modifiers.length > 0) {
    return `${modifiers.join('+')}+${key}`
  }
  return key
}

/**
 * 检查按键事件是否匹配快捷键
 */
export const isShortcutMatch = (event: KeyboardEvent, shortcut: ShortcutBinding): boolean => {
  // 检查主按键
  const normalizedKey = normalizeKey(event.key)
  const shortcutKey = normalizeKey(shortcut.key)

  if (normalizedKey !== shortcutKey) {
    return false
  }

  // 检查修饰键
  const eventModifiers = getEventModifiers(event)
  const shortcutModifiers = normalizeModifiers(shortcut.modifiers)

  return areModifiersEqual(eventModifiers, shortcutModifiers)
}

/**
 * 提取动作名称
 */
export const extractActionName = (action: string): string => {
  return action
}

/**
 * 检查是否为平台特定的快捷键
 */
export const isPlatformShortcut = (keyCombo: string): boolean => {
  const isMac = navigator.platform.includes('Mac')

  // 常见的平台快捷键
  const macShortcuts = ['cmd+c', 'cmd+v', 'cmd+x', 'cmd+z', 'cmd+a', 'cmd+s']
  const winShortcuts = ['ctrl+c', 'ctrl+v', 'ctrl+x', 'ctrl+z', 'ctrl+a', 'ctrl+s']

  if (isMac) {
    return macShortcuts.includes(keyCombo.toLowerCase())
  } else {
    return winShortcuts.includes(keyCombo.toLowerCase())
  }
}

/**
 * 生成调试信息
 */
export const generateDebugInfo = (event: KeyboardEvent, shortcut?: ShortcutBinding) => {
  return {
    timestamp: new Date().toISOString(),
    keyInfo: {
      key: event.key,
      code: event.code,
      normalizedKey: normalizeKey(event.key),
      modifiers: getEventModifiers(event),
      keyCombo: formatKeyCombo(event),
    },
    shortcutInfo: shortcut
      ? {
          key: shortcut.key,
          modifiers: shortcut.modifiers,
          action: extractActionName(shortcut.action),
          normalizedKey: normalizeKey(shortcut.key),
          normalizedModifiers: normalizeModifiers(shortcut.modifiers),
        }
      : null,
    browserInfo: {
      userAgent: navigator.userAgent,
      platform: navigator.platform,
      isMac: navigator.platform.includes('Mac'),
    },
  }
}
