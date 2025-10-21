/**
 * 快捷键管理 API
 *
 * 提供快捷键管理的统一接口，包括：
 * - 配置获取和更新
 * - 验证和冲突检测
 * - 搜索和格式化
 */

import { invoke } from '@/utils/request'
import type { ShortcutAction } from '@/types/domain/shortcuts'
import type {
  ShortcutsConfig,
  ShortcutBinding,
  Platform,
  ShortcutValidationResult,
  ConflictDetectionResult,
  ShortcutStatistics,
} from './types'

/**
 * 快捷键 API 接口类
 */
export class ShortcutsApi {
  getConfig = async (): Promise<ShortcutsConfig> => {
    return await invoke<ShortcutsConfig>('shortcuts_get_config')
  }

  updateConfig = async (config: ShortcutsConfig): Promise<void> => {
    await invoke('shortcuts_update_config', { config: config })
  }

  validateConfig = async (config: ShortcutsConfig): Promise<ShortcutValidationResult> => {
    return await invoke<ShortcutValidationResult>('shortcuts_validate_config', {
      config: config,
    })
  }

  detectConflicts = async (config: ShortcutsConfig): Promise<ConflictDetectionResult> => {
    return await invoke<ConflictDetectionResult>('shortcuts_detect_conflicts', {
      config: config,
    })
  }

  getCurrentPlatform = async (): Promise<Platform> => {
    return await invoke<Platform>('shortcuts_get_current_platform')
  }

  resetToDefaults = async (): Promise<void> => {
    await invoke('shortcuts_reset_to_defaults')
  }

  getStatistics = async (): Promise<ShortcutStatistics> => {
    return await invoke<ShortcutStatistics>('shortcuts_get_statistics')
  }

  addShortcut = async (shortcut: ShortcutBinding): Promise<void> => {
    await invoke('shortcuts_add', { binding: shortcut })
  }

  removeShortcut = async (index: number): Promise<ShortcutBinding> => {
    return await invoke<ShortcutBinding>('shortcuts_remove', { index })
  }

  updateShortcut = async (index: number, shortcut: ShortcutBinding): Promise<void> => {
    await invoke('shortcuts_update', { index, binding: shortcut })
  }

  executeAction = async (
    action: ShortcutAction,
    keyCombination: string,
    activeTerminalId?: string | null,
    metadata?: Record<string, unknown>
  ): Promise<unknown> => {
    return await invoke('shortcuts_execute_action', {
      action,
      keyCombination,
      activeTerminalId,
      metadata,
    })
  }
}

export const shortcutsApi = new ShortcutsApi()
export type * from './types'
export default shortcutsApi
