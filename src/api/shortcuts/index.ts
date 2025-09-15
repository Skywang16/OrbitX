/**
 * 快捷键管理 API
 *
 * 提供快捷键管理的统一接口，包括：
 * - 配置获取和更新
 * - 验证和冲突检测
 * - 搜索和格式化
 */

import { invoke } from '@/utils/request'
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
  async getConfig(): Promise<ShortcutsConfig> {
    return await invoke<ShortcutsConfig>('shortcuts_get_config')
  }

  async updateConfig(config: ShortcutsConfig): Promise<void> {
    await invoke('shortcuts_update_config', { config: config })
  }

  async validateConfig(config: ShortcutsConfig): Promise<ShortcutValidationResult> {
    return await invoke<ShortcutValidationResult>('shortcuts_validate_config', {
      config: config,
    })
  }

  async detectConflicts(config: ShortcutsConfig): Promise<ConflictDetectionResult> {
    return await invoke<ConflictDetectionResult>('shortcuts_detect_conflicts', {
      config: config,
    })
  }

  async getCurrentPlatform(): Promise<Platform> {
    return await invoke<Platform>('shortcuts_get_current_platform')
  }

  async resetToDefaults(): Promise<void> {
    await invoke('shortcuts_reset_to_defaults')
  }

  async getStatistics(): Promise<ShortcutStatistics> {
    return await invoke<ShortcutStatistics>('shortcuts_get_statistics')
  }

  async addShortcut(shortcut: ShortcutBinding): Promise<void> {
    await invoke('shortcuts_add', { binding: shortcut })
  }

  async removeShortcut(index: number): Promise<ShortcutBinding> {
    return await invoke<ShortcutBinding>('shortcuts_remove', { index })
  }

  async updateShortcut(index: number, shortcut: ShortcutBinding): Promise<void> {
    await invoke('shortcuts_update', { index, binding: shortcut })
  }

  async executeAction(
    action: any,
    keyCombination: string,
    activeTerminalId?: string | null,
    metadata?: any
  ): Promise<any> {
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
