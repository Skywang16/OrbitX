/**
 * 快捷键API模块
 *
 * 提供快捷键配置的增删改查、验证、冲突检测等功能的API封装
 */

import { invoke } from '@tauri-apps/api/core'
import {
  ShortcutsConfig,
  ShortcutBinding,
  ShortcutCategory,
  Platform,
  ShortcutValidationResult,
  ConflictDetectionResult,
  ShortcutStatistics,
  ShortcutApiError,
  ShortcutOperationOptions,
  ShortcutImportExportOptions,
  ShortcutSearchOptions,
  ShortcutSearchResult,
  ShortcutFormatOptions,
} from './types'

/**
 * 快捷键API类
 */
export class ShortcutApi {
  /**
   * 获取快捷键配置
   */
  static async getConfig(): Promise<ShortcutsConfig> {
    try {
      return await invoke<ShortcutsConfig>('get_shortcuts_config')
    } catch (error) {
      throw new ShortcutApiError(`获取快捷键配置失败: ${error}`)
    }
  }

  /**
   * 更新快捷键配置
   */
  static async updateConfig(config: ShortcutsConfig, options: ShortcutOperationOptions = {}): Promise<void> {
    try {
      // 如果需要验证，先验证配置
      if (options.validate !== false) {
        const validation = await this.validateConfig(config)
        if (!validation.is_valid) {
          throw new ShortcutApiError(`快捷键配置验证失败: ${validation.errors.map(e => e.message).join(', ')}`)
        }
      }

      // 如果需要检测冲突，先检测冲突
      if (options.checkConflicts) {
        const conflicts = await this.detectConflicts(config)
        if (conflicts.has_conflicts) {
          throw new ShortcutApiError(`快捷键配置存在冲突: ${conflicts.conflicts.length} 个冲突`)
        }
      }

      await invoke('update_shortcuts_config', { shortcutsConfig: config })
    } catch (error) {
      if (error instanceof ShortcutApiError) {
        throw error
      }
      throw new ShortcutApiError(`更新快捷键配置失败: ${error}`)
    }
  }

  /**
   * 验证快捷键配置
   */
  static async validateConfig(config: ShortcutsConfig): Promise<ShortcutValidationResult> {
    try {
      return await invoke<ShortcutValidationResult>('validate_shortcuts_config', {
        shortcutsConfig: config,
      })
    } catch (error) {
      throw new ShortcutApiError(`验证快捷键配置失败: ${error}`)
    }
  }

  /**
   * 验证单个快捷键绑定
   */
  static async validateBinding(binding: ShortcutBinding): Promise<ShortcutValidationResult> {
    try {
      return await invoke<ShortcutValidationResult>('validate_shortcut_binding', {
        shortcutBinding: binding,
      })
    } catch (error) {
      throw new ShortcutApiError(`验证快捷键绑定失败: ${error}`)
    }
  }

  /**
   * 检测快捷键冲突
   */
  static async detectConflicts(config: ShortcutsConfig): Promise<ConflictDetectionResult> {
    try {
      return await invoke<ConflictDetectionResult>('detect_shortcut_conflicts', {
        shortcutsConfig: config,
      })
    } catch (error) {
      throw new ShortcutApiError(`检测快捷键冲突失败: ${error}`)
    }
  }

  /**
   * 适配快捷键到指定平台
   */
  static async adaptToPlatform(config: ShortcutsConfig, platform: Platform): Promise<ShortcutsConfig> {
    try {
      return await invoke<ShortcutsConfig>('adapt_shortcuts_for_platform', {
        shortcutsConfig: config,
        targetPlatform: platform,
      })
    } catch (error) {
      throw new ShortcutApiError(`适配快捷键到平台失败: ${error}`)
    }
  }

  /**
   * 获取当前平台
   */
  static async getCurrentPlatform(): Promise<Platform> {
    try {
      return await invoke<Platform>('get_current_platform')
    } catch (error) {
      throw new ShortcutApiError(`获取当前平台失败: ${error}`)
    }
  }

  /**
   * 重置快捷键配置到默认值
   */
  static async resetToDefaults(): Promise<void> {
    try {
      await invoke('reset_shortcuts_to_defaults')
    } catch (error) {
      throw new ShortcutApiError(`重置快捷键配置失败: ${error}`)
    }
  }

  /**
   * 获取快捷键统计信息
   */
  static async getStatistics(): Promise<ShortcutStatistics> {
    try {
      return await invoke<ShortcutStatistics>('get_shortcuts_statistics')
    } catch (error) {
      throw new ShortcutApiError(`获取快捷键统计信息失败: ${error}`)
    }
  }

  /**
   * 添加快捷键
   */
  static async addShortcut(
    category: ShortcutCategory,
    shortcut: ShortcutBinding,
    options: ShortcutOperationOptions = {}
  ): Promise<void> {
    try {
      // 如果需要验证，先验证快捷键
      if (options.validate !== false) {
        const validation = await this.validateBinding(shortcut)
        if (!validation.is_valid) {
          throw new ShortcutApiError(`快捷键验证失败: ${validation.errors.map(e => e.message).join(', ')}`)
        }
      }

      await invoke('add_shortcut', {
        category,
        shortcut,
      })
    } catch (error) {
      if (error instanceof ShortcutApiError) {
        throw error
      }
      throw new ShortcutApiError(`添加快捷键失败: ${error}`)
    }
  }

  /**
   * 删除快捷键
   */
  static async removeShortcut(category: ShortcutCategory, index: number): Promise<ShortcutBinding> {
    try {
      return await invoke<ShortcutBinding>('remove_shortcut', {
        category,
        index,
      })
    } catch (error) {
      throw new ShortcutApiError(`删除快捷键失败: ${error}`)
    }
  }

  /**
   * 更新快捷键
   */
  static async updateShortcut(
    category: ShortcutCategory,
    index: number,
    shortcut: ShortcutBinding,
    options: ShortcutOperationOptions = {}
  ): Promise<void> {
    try {
      // 如果需要验证，先验证快捷键
      if (options.validate !== false) {
        const validation = await this.validateBinding(shortcut)
        if (!validation.is_valid) {
          throw new ShortcutApiError(`快捷键验证失败: ${validation.errors.map(e => e.message).join(', ')}`)
        }
      }

      await invoke('update_shortcut', {
        category,
        index,
        shortcut,
      })
    } catch (error) {
      if (error instanceof ShortcutApiError) {
        throw error
      }
      throw new ShortcutApiError(`更新快捷键失败: ${error}`)
    }
  }

  /**
   * 搜索快捷键
   */
  static async searchShortcuts(options: ShortcutSearchOptions): Promise<ShortcutSearchResult> {
    try {
      const config = await this.getConfig()
      const results: ShortcutSearchResult['shortcuts'] = []

      const searchInCategory = (shortcuts: ShortcutBinding[], category: ShortcutCategory) => {
        shortcuts.forEach((binding, index) => {
          let matches = true

          // 检查查询关键词
          if (options.query) {
            const query = options.query.toLowerCase()
            const keyMatches = binding.key.toLowerCase().includes(query)
            const modifierMatches = binding.modifiers.some(m => m.toLowerCase().includes(query))
            const actionMatches =
              typeof binding.action === 'string'
                ? binding.action.toLowerCase().includes(query)
                : binding.action.action_type.toLowerCase().includes(query)

            matches = matches && (keyMatches || modifierMatches || actionMatches)
          }

          // 检查按键
          if (options.key && matches) {
            matches = binding.key.toLowerCase() === options.key.toLowerCase()
          }

          // 检查修饰键
          if (options.modifiers && matches) {
            matches = options.modifiers.every(modifier =>
              binding.modifiers.some(m => m.toLowerCase() === modifier.toLowerCase())
            )
          }

          // 检查动作
          if (options.action && matches) {
            const actionStr = typeof binding.action === 'string' ? binding.action : binding.action.action_type
            matches = actionStr.toLowerCase().includes(options.action.toLowerCase())
          }

          if (matches) {
            results.push({ category, index, binding })
          }
        })
      }

      // 搜索指定类别或所有类别
      const categoriesToSearch = options.categories || [
        ShortcutCategory.Global,
        ShortcutCategory.Terminal,
        ShortcutCategory.Custom,
      ]

      for (const category of categoriesToSearch) {
        switch (category) {
          case ShortcutCategory.Global:
            searchInCategory(config.global, category)
            break
          case ShortcutCategory.Terminal:
            searchInCategory(config.terminal, category)
            break
          case ShortcutCategory.Custom:
            searchInCategory(config.custom, category)
            break
        }
      }

      return {
        shortcuts: results,
        total: results.length,
      }
    } catch (error) {
      throw new ShortcutApiError(`搜索快捷键失败: ${error}`)
    }
  }

  /**
   * 格式化快捷键为显示字符串
   */
  static formatShortcut(binding: ShortcutBinding, options: ShortcutFormatOptions = {}): string {
    const { platform, useSymbols = false, separator = '+' } = options

    let modifiers = [...binding.modifiers]

    // 根据平台调整修饰键显示
    if (platform === Platform.MacOS && useSymbols) {
      modifiers = modifiers.map(modifier => {
        switch (modifier.toLowerCase()) {
          case 'cmd':
          case 'meta':
            return '⌘'
          case 'ctrl':
            return '⌃'
          case 'alt':
            return '⌥'
          case 'shift':
            return '⇧'
          default:
            return modifier
        }
      })
    }

    // 组合修饰键和按键
    const parts = [...modifiers, binding.key]
    return parts.join(separator)
  }

  /**
   * 导出快捷键配置为JSON
   */
  static async exportConfig(options: ShortcutImportExportOptions = {}): Promise<string> {
    try {
      const config = await this.getConfig()

      // 如果指定了类别，只导出指定类别
      if (options.categories && options.categories.length > 0) {
        const filteredConfig: Partial<ShortcutsConfig> = {}

        for (const category of options.categories) {
          switch (category) {
            case ShortcutCategory.Global:
              filteredConfig.global = config.global
              break
            case ShortcutCategory.Terminal:
              filteredConfig.terminal = config.terminal
              break
            case ShortcutCategory.Custom:
              filteredConfig.custom = config.custom
              break
          }
        }

        return JSON.stringify(filteredConfig, null, 2)
      }

      return JSON.stringify(config, null, 2)
    } catch (error) {
      throw new ShortcutApiError(`导出快捷键配置失败: ${error}`)
    }
  }

  /**
   * 从JSON导入快捷键配置
   */
  static async importConfig(json: string, options: ShortcutImportExportOptions = {}): Promise<void> {
    try {
      const importedConfig = JSON.parse(json) as Partial<ShortcutsConfig>

      // 验证导入的配置
      const fullConfig: ShortcutsConfig = {
        global: importedConfig.global || [],
        terminal: importedConfig.terminal || [],
        custom: importedConfig.custom || [],
      }

      const validation = await this.validateConfig(fullConfig)
      if (!validation.is_valid) {
        throw new ShortcutApiError(`导入的配置验证失败: ${validation.errors.map(e => e.message).join(', ')}`)
      }

      // 如果需要备份现有配置
      if (options.backup) {
        const currentConfig = await this.getConfig()
        // 这里可以实现备份逻辑，比如保存到本地存储
      }

      // 根据选项决定是覆盖还是合并
      if (options.overwrite) {
        await this.updateConfig(fullConfig)
      } else {
        // 合并配置
        const currentConfig = await this.getConfig()
        const mergedConfig: ShortcutsConfig = {
          global: [...currentConfig.global, ...fullConfig.global],
          terminal: [...currentConfig.terminal, ...fullConfig.terminal],
          custom: [...currentConfig.custom, ...fullConfig.custom],
        }

        await this.updateConfig(mergedConfig)
      }
    } catch (error) {
      if (error instanceof ShortcutApiError) {
        throw error
      }
      throw new ShortcutApiError(`导入快捷键配置失败: ${error}`)
    }
  }
}

// 导出类型和API
export * from './types'
export { ShortcutApi as default }
