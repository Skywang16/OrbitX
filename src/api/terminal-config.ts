/**
 * 终端配置管理 API
 *
 * 提供终端配置的获取、更新、验证和 Shell 管理功能
 */

import { invoke } from '@/utils/request'

// ===== 类型定义 =====

export interface TerminalConfig {
  scrollback: number
  shell: ShellConfig
  cursor: CursorConfig
  behavior: TerminalBehaviorConfig
}

export interface ShellConfig {
  default: string
  args: string[]
  working_directory: string
}

export interface CursorConfig {
  style: 'block' | 'underline' | 'beam'
  blink: boolean
  color: string
  thickness: number
}

export interface TerminalBehaviorConfig {
  close_on_exit: boolean
  confirm_close: boolean
}

export interface TerminalConfigUpdateRequest {
  scrollback?: number
  shell?: ShellConfig
  cursor?: CursorConfig
  behavior?: TerminalBehaviorConfig
}

export interface TerminalConfigValidationResult {
  is_valid: boolean
  errors: string[]
  warnings: string[]
  validated_fields: string[]
}

export interface ShellInfo {
  name: string
  path: string
  display_name: string
  available: boolean
}

export interface SystemShellsResult {
  available_shells: ShellInfo[]
  default_shell?: ShellInfo
  current_shell?: ShellInfo
}

// ===== 终端配置管理 API =====

/**
 * 获取当前终端配置
 */
export async function getTerminalConfig(): Promise<TerminalConfig> {
  return await invoke('get_terminal_config')
}

/**
 * 更新终端配置
 * @param updateRequest 配置更新请求
 */
export async function updateTerminalConfig(updateRequest: TerminalConfigUpdateRequest): Promise<void> {
  return await invoke('update_terminal_config', { updateRequest })
}

/**
 * 验证终端配置
 */
export async function validateTerminalConfig(): Promise<TerminalConfigValidationResult> {
  return await invoke('validate_terminal_config')
}

/**
 * 重置终端配置为默认值
 */
export async function resetTerminalConfigToDefaults(): Promise<void> {
  return await invoke('reset_terminal_config_to_defaults')
}

// ===== Shell 管理 API =====

/**
 * 检测系统可用的 Shell
 */
export async function detectSystemShells(): Promise<SystemShellsResult> {
  return await invoke('detect_system_shells')
}

/**
 * 验证 Shell 路径
 * @param shellPath Shell 路径
 */
export async function validateShellPath(shellPath: string): Promise<boolean> {
  return await invoke('validate_terminal_shell_path', { shellPath })
}

/**
 * 获取 Shell 信息
 * @param shellPath Shell 路径
 */
export async function getShellInfo(shellPath: string): Promise<ShellInfo | null> {
  return await invoke('get_shell_info', { shellPath })
}

// ===== 光标配置 API =====

/**
 * 更新光标配置
 * @param cursorConfig 新的光标配置
 */
export async function updateCursorConfig(cursorConfig: CursorConfig): Promise<void> {
  return await invoke('update_cursor_config', { cursorConfig })
}

// ===== 终端行为配置 API =====

/**
 * 更新终端行为配置
 * @param behaviorConfig 新的终端行为配置
 */
export async function updateTerminalBehaviorConfig(behaviorConfig: TerminalBehaviorConfig): Promise<void> {
  return await invoke('update_terminal_behavior_config', { behaviorConfig })
}

// ===== 便捷方法 =====

/**
 * 更新滚动缓冲区大小
 * @param scrollback 滚动缓冲区行数
 */
export async function updateScrollback(scrollback: number): Promise<void> {
  return await updateTerminalConfig({ scrollback })
}

/**
 * 更新默认 Shell
 * @param shellPath Shell 路径
 * @param args Shell 参数（可选）
 * @param workingDirectory 工作目录（可选）
 */
export async function updateDefaultShell(shellPath: string, args?: string[], workingDirectory?: string): Promise<void> {
  const shell: ShellConfig = {
    default: shellPath,
    args: args || [],
    working_directory: workingDirectory || '~',
  }
  return await updateTerminalConfig({ shell })
}

/**
 * 更新光标样式
 * @param style 光标样式
 */
export async function updateCursorStyle(style: 'block' | 'underline' | 'beam'): Promise<void> {
  const config = await getTerminalConfig()
  const cursor: CursorConfig = {
    ...config.cursor,
    style,
  }
  return await updateCursorConfig(cursor)
}

/**
 * 更新光标颜色
 * @param color 光标颜色（十六进制格式）
 */
export async function updateCursorColor(color: string): Promise<void> {
  const config = await getTerminalConfig()
  const cursor: CursorConfig = {
    ...config.cursor,
    color,
  }
  return await updateCursorConfig(cursor)
}

/**
 * 切换光标闪烁
 */
export async function toggleCursorBlink(): Promise<void> {
  const config = await getTerminalConfig()
  const cursor: CursorConfig = {
    ...config.cursor,
    blink: !config.cursor.blink,
  }
  return await updateCursorConfig(cursor)
}

/**
 * 切换退出时关闭终端
 */
export async function toggleCloseOnExit(): Promise<void> {
  const config = await getTerminalConfig()
  const behavior: TerminalBehaviorConfig = {
    ...config.behavior,
    close_on_exit: !config.behavior.close_on_exit,
  }
  return await updateTerminalBehaviorConfig(behavior)
}

/**
 * 切换关闭时确认
 */
export async function toggleConfirmClose(): Promise<void> {
  const config = await getTerminalConfig()
  const behavior: TerminalBehaviorConfig = {
    ...config.behavior,
    confirm_close: !config.behavior.confirm_close,
  }
  return await updateTerminalBehaviorConfig(behavior)
}

// ===== 验证工具 =====

/**
 * 验证颜色格式是否有效
 * @param color 颜色字符串
 */
export function isValidColor(color: string): boolean {
  return /^#[0-9A-Fa-f]{6}$/.test(color)
}

/**
 * 验证滚动缓冲区大小是否有效
 * @param scrollback 滚动缓冲区行数
 */
export function isValidScrollback(scrollback: number): boolean {
  return scrollback >= 0 && scrollback <= 1000000
}

/**
 * 验证光标粗细是否有效
 * @param thickness 光标粗细
 */
export function isValidCursorThickness(thickness: number): boolean {
  return thickness >= 0.0 && thickness <= 1.0
}

// ===== 默认值 =====

export const DEFAULT_TERMINAL_CONFIG: TerminalConfig = {
  scrollback: 1000,
  shell: {
    default: 'zsh',
    args: [],
    working_directory: '~',
  },
  cursor: {
    style: 'block',
    blink: true,
    color: '#ffffff',
    thickness: 0.15,
  },
  behavior: {
    close_on_exit: true,
    confirm_close: false,
  },
}

export const CURSOR_STYLES = [
  { value: 'block', label: '方块' },
  { value: 'underline', label: '下划线' },
  { value: 'beam', label: '竖线' },
] as const

export const COMMON_SHELLS = [
  { name: 'zsh', displayName: 'Zsh', path: '/bin/zsh' },
  { name: 'bash', displayName: 'Bash', path: '/bin/bash' },
  { name: 'fish', displayName: 'Fish', path: '/usr/bin/fish' },
  { name: 'sh', displayName: 'Bourne Shell', path: '/bin/sh' },
] as const
