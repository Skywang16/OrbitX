/**
 * Shell管理相关的API接口
 */

import { invoke } from '@/utils/request'
import { handleError } from '../../utils/errorHandler'
import type {
  ShellConfigPaths,
  ShellFeatures,
  ShellInfo,
  ShellManagerStats,
  ShellRecommendation,
  ShellSearchResult,
  ShellStartupArgs,
  ShellStats,
  ShellValidationResult,
} from './types'

/**
 * Shell管理API类
 * 提供Shell的发现、验证、管理等功能
 */
export class ShellAPI {
  // ===== 缓存管理 =====
  private shellsCache: ShellInfo[] | null = null
  private cacheExpiry: number = 0
  private readonly CACHE_DURATION = 5 * 60 * 1000 // 5分钟

  // ===== 基本Shell操作 =====

  /**
   * 获取可用的Shell列表
   */
  async getAvailableShells(forceRefresh = false): Promise<ShellInfo[]> {
    // 检查缓存
    if (!forceRefresh && this.shellsCache && Date.now() < this.cacheExpiry) {
      return this.shellsCache
    }

    try {
      const shells = await invoke<ShellInfo[]>('get_available_shells')
      // 更新缓存
      this.shellsCache = shells
      this.cacheExpiry = Date.now() + this.CACHE_DURATION
      return shells
    } catch (error) {
      // 如果API调用失败，返回缓存的数据（如果有的话）
      if (this.shellsCache) {
        console.warn('API调用失败，使用缓存数据:', handleError(error))
        return this.shellsCache
      }
      throw new Error(handleError(error, '获取可用Shell列表失败'))
    }
  }

  /**
   * 获取默认Shell信息
   */
  async getDefaultShell(): Promise<ShellInfo> {
    try {
      return await invoke<ShellInfo>('get_default_shell')
    } catch (error) {
      throw new Error(handleError(error, '获取默认Shell失败'))
    }
  }

  /**
   * 验证Shell路径是否有效
   */
  async validateShellPath(path: string): Promise<boolean> {
    try {
      return await invoke<boolean>('validate_shell_path', { path })
    } catch (error) {
      console.warn('验证Shell路径失败:', handleError(error))
      return false
    }
  }

  // ===== Shell查找功能 =====

  /**
   * 根据名称查找Shell
   */
  async findShellByName(name: string): Promise<ShellInfo | null> {
    try {
      const shells = await this.getAvailableShells()
      return shells.find(shell => shell.name.toLowerCase() === name.toLowerCase()) || null
    } catch (error) {
      console.warn('根据名称查找Shell失败:', handleError(error))
      return null
    }
  }

  /**
   * 根据路径查找Shell
   */
  async findShellByPath(path: string): Promise<ShellInfo | null> {
    try {
      const shells = await this.getAvailableShells()
      return shells.find(shell => shell.path === path) || null
    } catch (error) {
      console.warn('根据路径查找Shell失败:', handleError(error))
      return null
    }
  }

  /**
   * 搜索Shell（支持模糊匹配）
   */
  async searchShells(query: string): Promise<ShellSearchResult> {
    try {
      const shells = await this.getAvailableShells()
      const lowerQuery = query.toLowerCase()

      const matchedShells = shells.filter(
        shell =>
          shell.name.toLowerCase().includes(lowerQuery) ||
          shell.displayName.toLowerCase().includes(lowerQuery) ||
          shell.path.toLowerCase().includes(lowerQuery)
      )

      return {
        shells: matchedShells,
        query,
        totalFound: matchedShells.length,
      }
    } catch (error) {
      throw new Error(handleError(error, '搜索Shell失败'))
    }
  }

  // ===== Shell推荐和功能 =====

  /**
   * 获取推荐的Shell列表
   */
  async getRecommendedShells(): Promise<ShellRecommendation[]> {
    try {
      const shells = await this.getAvailableShells()
      const recommendations: ShellRecommendation[] = []

      for (const shell of shells) {
        const features = await this.checkShellFeatures(shell.name)
        const score = this.calculateShellScore(shell, features)
        const reasons = this.getRecommendationReasons(shell, features)

        recommendations.push({
          shell,
          score,
          reasons,
          features,
        })
      }

      // 按分数排序
      return recommendations.sort((a, b) => b.score - a.score)
    } catch (error) {
      throw new Error(handleError(error, '获取推荐Shell失败'))
    }
  }

  /**
   * 检查Shell功能支持
   */
  async checkShellFeatures(shellName: string): Promise<ShellFeatures> {
    try {
      // 这里可以根据Shell名称返回已知的功能支持情况
      const knownFeatures: Record<string, ShellFeatures> = {
        bash: {
          supportsColors: true,
          supportsUnicode: true,
          supportsTabCompletion: true,
          supportsHistory: true,
          supportsAliases: true,
          supportsScripting: true,
        },
        zsh: {
          supportsColors: true,
          supportsUnicode: true,
          supportsTabCompletion: true,
          supportsHistory: true,
          supportsAliases: true,
          supportsScripting: true,
        },
        fish: {
          supportsColors: true,
          supportsUnicode: true,
          supportsTabCompletion: true,
          supportsHistory: true,
          supportsAliases: true,
          supportsScripting: true,
        },
        powershell: {
          supportsColors: true,
          supportsUnicode: true,
          supportsTabCompletion: true,
          supportsHistory: true,
          supportsAliases: true,
          supportsScripting: true,
        },
        cmd: {
          supportsColors: false,
          supportsUnicode: false,
          supportsTabCompletion: true,
          supportsHistory: true,
          supportsAliases: false,
          supportsScripting: true,
        },
      }

      return (
        knownFeatures[shellName.toLowerCase()] || {
          supportsColors: false,
          supportsUnicode: false,
          supportsTabCompletion: false,
          supportsHistory: false,
          supportsAliases: false,
          supportsScripting: false,
        }
      )
    } catch (error) {
      console.warn('检查Shell功能失败:', handleError(error))
      return {
        supportsColors: false,
        supportsUnicode: false,
        supportsTabCompletion: false,
        supportsHistory: false,
        supportsAliases: false,
        supportsScripting: false,
      }
    }
  }

  // ===== 私有辅助方法 =====

  private calculateShellScore(shell: ShellInfo, features: ShellFeatures): number {
    let score = 0

    // 基础分数
    score += 10

    // 功能加分
    if (features.supportsColors) score += 15
    if (features.supportsUnicode) score += 10
    if (features.supportsTabCompletion) score += 20
    if (features.supportsHistory) score += 15
    if (features.supportsAliases) score += 10
    if (features.supportsScripting) score += 20

    // 常见Shell加分
    const popularShells = ['bash', 'zsh', 'fish', 'powershell']
    if (popularShells.includes(shell.name.toLowerCase())) {
      score += 10
    }

    return score
  }

  private getRecommendationReasons(shell: ShellInfo, features: ShellFeatures): string[] {
    const reasons: string[] = []

    if (features.supportsColors) reasons.push('支持颜色显示')
    if (features.supportsUnicode) reasons.push('支持Unicode字符')
    if (features.supportsTabCompletion) reasons.push('支持Tab补全')
    if (features.supportsHistory) reasons.push('支持命令历史')
    if (features.supportsAliases) reasons.push('支持别名')
    if (features.supportsScripting) reasons.push('支持脚本编程')

    const popularShells = ['bash', 'zsh', 'fish', 'powershell']
    if (popularShells.includes(shell.name.toLowerCase())) {
      reasons.push('广泛使用的Shell')
    }

    return reasons
  }

  // ===== Shell配置和工具 =====

  /**
   * 获取Shell配置文件路径
   */
  async getShellConfigPath(shell: ShellInfo): Promise<ShellConfigPaths> {
    try {
      const configPaths: ShellConfigPaths = {
        shell,
        configFiles: [],
        profileFiles: [],
        historyFiles: [],
      }

      // 根据Shell类型返回常见的配置文件路径
      switch (shell.name.toLowerCase()) {
        case 'bash':
          configPaths.configFiles = ['~/.bashrc', '~/.bash_profile', '/etc/bash.bashrc']
          configPaths.profileFiles = ['~/.bash_profile', '~/.profile']
          configPaths.historyFiles = ['~/.bash_history']
          break
        case 'zsh':
          configPaths.configFiles = ['~/.zshrc', '~/.zprofile', '/etc/zsh/zshrc']
          configPaths.profileFiles = ['~/.zprofile', '~/.profile']
          configPaths.historyFiles = ['~/.zsh_history']
          break
        case 'fish':
          configPaths.configFiles = ['~/.config/fish/config.fish']
          configPaths.profileFiles = ['~/.config/fish/config.fish']
          configPaths.historyFiles = ['~/.local/share/fish/fish_history']
          break
        case 'powershell':
          configPaths.configFiles = ['$PROFILE', '$PROFILE.AllUsersAllHosts']
          configPaths.profileFiles = ['$PROFILE']
          configPaths.historyFiles = ['%APPDATA%\\Microsoft\\Windows\\PowerShell\\PSReadLine\\ConsoleHost_history.txt']
          break
      }

      return configPaths
    } catch (error) {
      throw new Error(handleError(error, '获取Shell配置路径失败'))
    }
  }

  /**
   * 获取Shell启动参数
   */
  async getShellStartupArgs(shell: ShellInfo): Promise<ShellStartupArgs> {
    try {
      const startupArgs: ShellStartupArgs = {
        shell,
        args: shell.args || [],
        env: {},
      }

      // 根据Shell类型设置默认启动参数
      switch (shell.name.toLowerCase()) {
        case 'bash':
          startupArgs.args = ['-i', '-l'] // 交互式登录Shell
          break
        case 'zsh':
          startupArgs.args = ['-i', '-l'] // 交互式登录Shell
          break
        case 'fish':
          startupArgs.args = ['-i'] // 交互式Shell
          break
        case 'powershell':
          startupArgs.args = ['-NoLogo', '-NoExit'] // 无Logo，不退出
          break
      }

      return startupArgs
    } catch (error) {
      throw new Error(handleError(error, '获取Shell启动参数失败'))
    }
  }

  // ===== 批量操作 =====

  /**
   * 批量验证多个Shell路径
   */
  async validateMultipleShellPaths(paths: string[]): Promise<ShellValidationResult[]> {
    try {
      const results: ShellValidationResult[] = []

      for (const path of paths) {
        try {
          const valid = await this.validateShellPath(path)
          results.push({ valid, path })
        } catch (error) {
          results.push({
            valid: false,
            path,
            error: handleError(error),
          })
        }
      }

      return results
    } catch (error) {
      throw new Error(handleError(error, '批量验证Shell路径失败'))
    }
  }

  // ===== 缓存和统计 =====

  /**
   * 清除Shell缓存
   */
  clearCache(): void {
    this.shellsCache = null
    this.cacheExpiry = 0
  }

  /**
   * 获取Shell统计信息
   */
  async getShellStats(): Promise<ShellStats> {
    try {
      const shells = await this.getAvailableShells()
      const defaultShell = await this.getDefaultShell()

      return {
        totalShells: shells.length,
        availableShells: shells.length,
        defaultShell: defaultShell.name,
        lastRefresh: new Date().toISOString(),
      }
    } catch (error) {
      throw new Error(handleError(error, '获取Shell统计信息失败'))
    }
  }

  // ===== Shell管理器功能 =====

  /**
   * 获取Shell管理器统计信息
   */
  async getShellManagerStats(): Promise<ShellManagerStats> {
    try {
      const stats = await this.getShellStats()

      return {
        initialized: true,
        shellCount: stats.totalShells,
        cacheHitRate: this.shellsCache ? 1.0 : 0.0,
        lastUpdate: stats.lastRefresh,
      }
    } catch (error) {
      throw new Error(handleError(error, '获取Shell管理器统计失败'))
    }
  }

  /**
   * 初始化Shell管理器
   */
  async initializeShellManager(): Promise<void> {
    try {
      // 预加载Shell列表
      await this.getAvailableShells(true)
    } catch (error) {
      throw new Error(handleError(error, '初始化Shell管理器失败'))
    }
  }

  /**
   * 验证Shell管理器状态
   */
  async validateShellManager(): Promise<boolean> {
    try {
      const stats = await this.getShellManagerStats()
      return stats.initialized && stats.shellCount > 0
    } catch (error) {
      console.warn('验证Shell管理器状态失败:', handleError(error))
      return false
    }
  }
}

/**
 * Shell API实例
 */
export const shellAPI = new ShellAPI()

/**
 * 便捷的Shell操作函数集合
 */
export const shell = {
  // 基本操作
  getAvailable: (forceRefresh?: boolean) => shellAPI.getAvailableShells(forceRefresh),
  getDefault: () => shellAPI.getDefaultShell(),
  validate: (path: string) => shellAPI.validateShellPath(path),

  // 查找功能
  findByName: (name: string) => shellAPI.findShellByName(name),
  findByPath: (path: string) => shellAPI.findShellByPath(path),
  search: (query: string) => shellAPI.searchShells(query),

  // 推荐和功能
  getRecommended: () => shellAPI.getRecommendedShells(),
  checkFeatures: (shellName: string) => shellAPI.checkShellFeatures(shellName),

  // 配置和工具
  getConfigPaths: (shell: ShellInfo) => shellAPI.getShellConfigPath(shell),
  getStartupArgs: (shell: ShellInfo) => shellAPI.getShellStartupArgs(shell),

  // 批量操作
  validateMultiple: (paths: string[]) => shellAPI.validateMultipleShellPaths(paths),

  // 缓存和统计
  clearCache: () => shellAPI.clearCache(),
  getStats: () => shellAPI.getShellStats(),

  // 管理器功能
  getManagerStats: () => shellAPI.getShellManagerStats(),
  initialize: () => shellAPI.initializeShellManager(),
  validateManager: () => shellAPI.validateShellManager(),
}

// 重新导出类型
export type * from './types'
