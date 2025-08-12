/**
 * 主题管理 API
 *
 * 提供与后端主题系统交互的 API 接口，包括主题获取、切换、验证等功能。
 * 支持手动选择主题和跟随系统主题两种模式。
 */

import { invoke } from '@/utils/request'
import { handleError } from '../../utils/errorHandler'
import type { Theme, ThemeInfo, ThemeConfigStatus } from '@/types/theme'

// ============================================================================
// 主题管理 API 类
// ============================================================================

/**
 * 主题管理API类
 * 提供主题的获取、切换、验证等功能
 */
export class ThemeAPI {
  /**
   * 获取当前主题配置状态
   */
  async getThemeConfigStatus(): Promise<ThemeConfigStatus> {
    try {
      return await invoke<ThemeConfigStatus>('get_theme_config_status')
    } catch (error) {
      throw new Error(handleError(error, '获取主题配置状态失败'))
    }
  }

  /**
   * 获取当前主题数据
   */
  async getCurrentTheme(): Promise<Theme> {
    try {
      return await invoke<Theme>('get_current_theme')
    } catch (error) {
      throw new Error(handleError(error, '获取当前主题失败'))
    }
  }

  /**
   * 设置终端主题（手动模式）
   */
  async setTerminalTheme(themeName: string): Promise<void> {
    try {
      return await invoke<void>('set_terminal_theme', { themeName: themeName })
    } catch (error) {
      throw new Error(handleError(error, `设置终端主题失败: ${themeName}`))
    }
  }

  /**
   * 设置跟随系统主题
   */
  async setFollowSystemTheme(followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> {
    try {
      return await invoke<void>('set_follow_system_theme', {
        followSystem: followSystem,
        lightTheme: lightTheme || null,
        darkTheme: darkTheme || null,
      })
    } catch (error) {
      throw new Error(handleError(error, '设置跟随系统主题失败'))
    }
  }

  /**
   * 获取所有可用主题列表
   */
  async getAvailableThemes(): Promise<ThemeInfo[]> {
    try {
      return await invoke<ThemeInfo[]>('get_available_themes')
    } catch (error) {
      throw new Error(handleError(error, '获取可用主题列表失败'))
    }
  }
}

// ============================================================================
// 单例实例和便捷函数
// ============================================================================

/**
 * 主题API单例实例
 */
export const themeAPI = new ThemeAPI()

/**
 * 便捷的主题操作函数
 */
export const theme = {
  // 获取状态和数据
  getConfigStatus: () => themeAPI.getThemeConfigStatus(),
  getCurrentTheme: () => themeAPI.getCurrentTheme(),
  getAvailableThemes: () => themeAPI.getAvailableThemes(),

  // 主题切换
  setTerminalTheme: (name: string) => themeAPI.setTerminalTheme(name),
  setFollowSystemTheme: (followSystem: boolean, lightTheme?: string, darkTheme?: string) =>
    themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme),
}

// 导出单独的函数以保持向后兼容
export const getThemeConfigStatus = () => themeAPI.getThemeConfigStatus()
export const getCurrentTheme = () => themeAPI.getCurrentTheme()
export const getAvailableThemes = () => themeAPI.getAvailableThemes()
export const setTerminalTheme = (name: string) => themeAPI.setTerminalTheme(name)
export const setFollowSystemTheme = (followSystem: boolean, lightTheme?: string, darkTheme?: string) =>
  themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme)
