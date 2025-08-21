/**
 * 快捷键管理 API
 *
 * 提供快捷键管理的统一接口，包括：
 * - 配置获取和更新
 * - 验证和冲突检测
 * - 搜索和格式化
 */

import { invoke } from '@tauri-apps/api/core'
import type {
  ShortcutsConfig,
  ShortcutBinding,
  ShortcutCategory,
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
    try {
      return await invoke<ShortcutsConfig>('get_shortcuts_config')
    } catch (error) {
      throw new Error(`获取快捷键配置失败: ${error}`)
    }
  }

  async updateConfig(config: ShortcutsConfig): Promise<void> {
    try {
      await invoke('update_shortcuts_config', { config: config })
    } catch (error) {
      throw new Error(`更新快捷键配置失败: ${error}`)
    }
  }

  async validateConfig(config: ShortcutsConfig): Promise<ShortcutValidationResult> {
    try {
      return await invoke<ShortcutValidationResult>('validate_shortcuts_config', {
        config: config,
      })
    } catch (error) {
      throw new Error(`验证快捷键配置失败: ${error}`)
    }
  }

  async validateBinding(binding: ShortcutBinding): Promise<ShortcutValidationResult> {
    try {
      return await invoke<ShortcutValidationResult>('validate_shortcut_binding', {
        shortcutBinding: binding,
      })
    } catch (error) {
      throw new Error(`验证快捷键绑定失败: ${error}`)
    }
  }

  async detectConflicts(config: ShortcutsConfig): Promise<ConflictDetectionResult> {
    try {
      return await invoke<ConflictDetectionResult>('detect_shortcuts_conflicts', {
        config: config,
      })
    } catch (error) {
      throw new Error(`检测快捷键冲突失败: ${error}`)
    }
  }

  async adaptToPlatform(config: ShortcutsConfig, platform: Platform): Promise<ShortcutsConfig> {
    try {
      return await invoke<ShortcutsConfig>('adapt_shortcuts_for_platform', {
        shortcutsConfig: config,
        targetPlatform: platform,
      })
    } catch (error) {
      throw new Error(`适配快捷键到平台失败: ${error}`)
    }
  }

  async getCurrentPlatform(): Promise<Platform> {
    try {
      return await invoke<Platform>('get_current_platform')
    } catch (error) {
      throw new Error(`获取当前平台失败: ${error}`)
    }
  }

  async resetToDefaults(): Promise<void> {
    try {
      await invoke('reset_shortcuts_to_defaults')
    } catch (error) {
      throw new Error(`重置快捷键配置失败: ${error}`)
    }
  }

  async getStatistics(): Promise<ShortcutStatistics> {
    try {
      return await invoke<ShortcutStatistics>('get_shortcuts_statistics')
    } catch (error) {
      throw new Error(`获取快捷键统计信息失败: ${error}`)
    }
  }

  async addShortcut(category: ShortcutCategory, shortcut: ShortcutBinding): Promise<void> {
    try {
      await invoke('add_shortcut', { category, shortcut })
    } catch (error) {
      throw new Error(`添加快捷键失败: ${error}`)
    }
  }

  async removeShortcut(category: ShortcutCategory, index: number): Promise<ShortcutBinding> {
    try {
      return await invoke<ShortcutBinding>('remove_shortcut', { category, index })
    } catch (error) {
      throw new Error(`删除快捷键失败: ${error}`)
    }
  }

  async updateShortcut(category: ShortcutCategory, index: number, shortcut: ShortcutBinding): Promise<void> {
    try {
      await invoke('update_shortcut', { category, index, shortcut })
    } catch (error) {
      throw new Error(`更新快捷键失败: ${error}`)
    }
  }

  async executeAction(
    action: any,
    keyCombination: string,
    activeTerminalId?: string | null,
    metadata?: any
  ): Promise<any> {
    try {
      return await invoke('execute_shortcut_action', {
        action,
        keyCombination,
        activeTerminalId,
        metadata,
      })
    } catch (error) {
      throw new Error(`执行快捷键动作失败: ${error}`)
    }
  }
}

// 导出单例实例
export const shortcutsApi = new ShortcutsApi()

// 导出类型
export type * from './types'

// 默认导出
export default shortcutsApi
