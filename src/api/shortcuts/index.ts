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
    return await invoke<ShortcutsConfig>('get_shortcuts_config')
  }

  async updateConfig(config: ShortcutsConfig): Promise<void> {
    await invoke('update_shortcuts_config', { config: config })
  }

  async validateConfig(config: ShortcutsConfig): Promise<ShortcutValidationResult> {
    return await invoke<ShortcutValidationResult>('validate_shortcuts_config', {
      config: config,
    })
  }

  async detectConflicts(config: ShortcutsConfig): Promise<ConflictDetectionResult> {
    return await invoke<ConflictDetectionResult>('detect_shortcuts_conflicts', {
      config: config,
    })
  }

  async getCurrentPlatform(): Promise<Platform> {
    return await invoke<Platform>('get_current_platform')
  }

  async resetToDefaults(): Promise<void> {
    await invoke('reset_shortcuts_to_defaults')
  }

  async getStatistics(): Promise<ShortcutStatistics> {
    return await invoke<ShortcutStatistics>('get_shortcuts_statistics')
  }

  async addShortcut(shortcut: ShortcutBinding): Promise<void> {
    await invoke('add_shortcut', { binding: shortcut })
  }

  async removeShortcut(index: number): Promise<ShortcutBinding> {
    return await invoke<ShortcutBinding>('remove_shortcut', { index })
  }

  async updateShortcut(index: number, shortcut: ShortcutBinding): Promise<void> {
    await invoke('update_shortcut', { index, binding: shortcut })
  }

  async executeAction(
    action: any,
    keyCombination: string,
    activeTerminalId?: string | null,
    metadata?: any
  ): Promise<any> {
    return await invoke('execute_shortcut_action', {
      action,
      keyCombination,
      activeTerminalId,
      metadata,
    })
  }
}

// 导出单例实例
export const shortcutsApi = new ShortcutsApi()

// 导出类型
export type * from './types'

// 默认导出
export default shortcutsApi
