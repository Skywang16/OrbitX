/**
 * 窗口管理相关的API接口
 */

import { createMessage } from '@/ui'
import { invoke } from '@/utils/request'
import { handleError } from '../../utils/errorHandler'
import type {
  CompleteWindowState,
  DirectoryCache,
  DirectoryOptions,
  PathInfo,
  PathOperationResult,
  PathResolveOptions,
  PlatformInfo,
  WindowProperties,
  WindowState,
  WindowStateBatchRequest,
  WindowStateBatchResponse,
  WindowStateOperationResult,
} from './types'

/**
 * 窗口管理API
 * 提供窗口相关的管理和操作功能
 */
export class WindowAPI {
  private alwaysOnTopState = false
  private currentDirectory: string | null = null
  private directoryCache: DirectoryCache | null = null
  private readonly DIRECTORY_CACHE_DURATION = 30 * 1000 // 30秒缓存

  // 平台信息缓存
  private platformInfoCache: PlatformInfo | null = null

  // ===== 新的批量窗口状态管理方法 =====

  /**
   * 批量执行窗口状态操作
   */
  async manageWindowState(request: WindowStateBatchRequest): Promise<WindowStateBatchResponse> {
    try {
      return await invoke<WindowStateBatchResponse>('manage_window_state', { request })
    } catch (error) {
      throw new Error(handleError(error, '批量窗口状态管理失败'))
    }
  }

  /**
   * 获取完整的窗口状态
   */
  async getCompleteWindowState(): Promise<CompleteWindowState> {
    const request = {
      operations: [
        {
          operation: 'get_state' as const,
        },
      ],
    }

    const response = await this.manageWindowState(request)
    if (response.overallSuccess && response.results.length > 0) {
      const state = response.results[0].data
      // 更新本地缓存
      this.alwaysOnTopState = state.alwaysOnTop
      return state
    }
    throw new Error('获取窗口状态失败')
  }

  /**
   * 设置窗口置顶状态
   */
  async setAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
    const request = {
      operations: [
        {
          operation: 'set_always_on_top' as const,
          params: { alwaysOnTop: alwaysOnTop },
        },
      ],
    }

    const response = await this.manageWindowState(request)
    if (!response.overallSuccess) {
      throw new Error('设置窗口置顶失败')
    }

    // 更新本地状态
    this.alwaysOnTopState = alwaysOnTop
  }

  /**
   * 切换窗口置顶状态
   */
  async toggleAlwaysOnTop(): Promise<boolean> {
    const request = {
      operations: [
        {
          operation: 'toggle_always_on_top' as const,
        },
      ],
    }

    const response = await this.manageWindowState(request)
    if (response.overallSuccess && response.results.length > 0) {
      const newState = response.results[0].data
      this.alwaysOnTopState = newState
      return newState
    }
    throw new Error('切换窗口置顶状态失败')
  }

  /**
   * 获取当前窗口置顶状态
   */
  getAlwaysOnTopState(): boolean {
    return this.alwaysOnTopState
  }

  // ===== 目录操作 =====

  /**
   * 获取当前工作目录
   */
  async getCurrentDirectory(options: DirectoryOptions = {}): Promise<string> {
    const { useCache = true } = options

    // 检查缓存
    if (useCache && this.directoryCache) {
      const now = Date.now()
      if (now - this.directoryCache.timestamp < this.DIRECTORY_CACHE_DURATION) {
        return this.directoryCache.value
      }
    }

    try {
      const directory = await invoke<string>('get_current_directory', { useCache })
      // 更新缓存
      this.directoryCache = {
        value: directory,
        timestamp: Date.now(),
      }
      this.currentDirectory = directory
      return directory
    } catch (error) {
      // 如果API调用失败，返回缓存的目录（如果有的话）
      if (this.currentDirectory) {
        console.warn('获取当前目录失败，使用缓存:', handleError(error))
        return this.currentDirectory
      }
      throw new Error(handleError(error, '获取当前目录失败'))
    }
  }

  /**
   * 刷新当前目录缓存
   */
  async refreshCurrentDirectory(): Promise<string> {
    return this.getCurrentDirectory({ useCache: false })
  }

  /**
   * 清除目录缓存
   */
  async clearDirectoryCache(): Promise<void> {
    try {
      await invoke<void>('clear_directory_cache')
      this.directoryCache = null
      this.currentDirectory = null
      createMessage.success('目录缓存已清除')
    } catch (error) {
      throw new Error(handleError(error, '清除目录缓存失败'))
    }
  }

  /**
   * 获取用户主目录
   */
  async getHomeDirectory(): Promise<string> {
    try {
      return await invoke<string>('get_home_directory', { forceRefresh: true })
    } catch (error) {
      throw new Error(handleError(error, '获取家目录失败'))
    }
  }

  // ===== 路径操作 =====

  /**
   * 检查路径是否存在
   */
  async pathExists(path: string): Promise<boolean> {
    try {
      return await invoke<boolean>('path_exists', { path })
    } catch (error) {
      console.warn('检查路径存在性失败:', handleError(error))
      return false
    }
  }

  /**
   * 检查目录是否存在（兼容性方法）
   */
  async directoryExists(path: string): Promise<boolean> {
    return this.pathExists(path)
  }

  /**
   * 规范化路径
   */
  async normalizePath(path: string): Promise<string> {
    try {
      return await invoke<string>('normalize_path', { path })
    } catch (error) {
      throw new Error(handleError(error, '路径规范化失败'))
    }
  }

  /**
   * 解析相对路径为绝对路径
   */
  async resolveRelativePath(relativePath: string, options: PathResolveOptions = {}): Promise<string> {
    const { basePath, normalize = true } = options

    if (!relativePath) return ''

    // 如果已经是绝对路径，直接返回
    if (this.isAbsolutePath(relativePath)) {
      return normalize ? this.normalizePath(relativePath) : relativePath
    }

    try {
      // 获取基础路径
      const base = basePath || (await this.getCurrentDirectory({ useCache: true }))
      // 组合路径
      const separator = base.includes('\\') ? '\\' : '/'
      const fullPath = `${base}${separator}${relativePath}`
      return normalize ? this.normalizePath(fullPath) : fullPath
    } catch (error) {
      throw new Error(handleError(error, '解析相对路径失败'))
    }
  }

  /**
   * 检查是否为绝对路径
   */
  isAbsolutePath(path: string): boolean {
    if (!path) return false
    // Unix/Linux 绝对路径
    if (path.startsWith('/')) return true
    // Windows 绝对路径
    if (path.match(/^[A-Za-z]:/)) return true
    // UNC 路径
    if (path.startsWith('\\\\')) return true
    return false
  }

  /**
   * 获取路径的父目录
   */
  getParentDirectory(path: string): string {
    if (!path) return ''
    // 本地路径处理，不需要调用后端API
    const normalized = path.replace(/\\/g, '/').replace(/\/+/g, '/')
    const lastSlash = normalized.lastIndexOf('/')
    if (lastSlash <= 0) {
      return '/' // 根目录
    }
    return normalized.substring(0, lastSlash)
  }

  /**
   * 获取路径的文件名部分
   */
  getFileName(path: string): string {
    if (!path) return ''
    // 本地路径处理，不需要调用后端API
    const normalized = path.replace(/\\/g, '/').replace(/\/+/g, '/')
    const lastSlash = normalized.lastIndexOf('/')
    return normalized.substring(lastSlash + 1)
  }

  /**
   * 连接路径
   */
  async joinPaths(...paths: string[]): Promise<string> {
    try {
      return await invoke<string>('join_paths', { paths })
    } catch (error) {
      throw new Error(handleError(error, '路径连接失败'))
    }
  }

  /**
   * 获取路径详细信息
   */
  async getPathInfo(path: string): Promise<PathInfo> {
    try {
      const [exists, normalized] = await Promise.all([this.pathExists(path), this.normalizePath(path)])

      return {
        path,
        exists,
        isAbsolute: this.isAbsolutePath(path),
        parent: this.getParentDirectory(path),
        fileName: this.getFileName(path),
        normalized,
      }
    } catch (error) {
      throw new Error(handleError(error, '获取路径信息失败'))
    }
  }

  // ===== 平台信息 =====

  /**
   * 获取平台信息
   * 首次调用时从后端获取并缓存，后续调用直接返回缓存
   */
  async getPlatformInfo(): Promise<PlatformInfo> {
    // 如果已有缓存，直接返回
    if (this.platformInfoCache) {
      return this.platformInfoCache
    }

    try {
      // 从后端获取平台信息
      const platformInfo = await invoke<PlatformInfo>('get_platform_info')

      // 缓存结果
      this.platformInfoCache = platformInfo

      return platformInfo
    } catch (error) {
      throw new Error(handleError(error, '获取平台信息失败'))
    }
  }

  /**
   * 检查是否为Mac系统
   * 使用缓存的平台信息
   */
  async isMac(): Promise<boolean> {
    const platformInfo = await this.getPlatformInfo()
    return platformInfo.is_mac
  }

  // ===== 高级功能 =====

  /**
   * 获取窗口状态信息
   * 使用新的批量接口实现，获取更完整的状态
   */
  async getWindowState(): Promise<WindowState> {
    try {
      const completeState = await this.getCompleteWindowState()

      // 更新本地缓存状态
      this.alwaysOnTopState = completeState.alwaysOnTop

      return {
        alwaysOnTop: completeState.alwaysOnTop,
        currentDirectory: completeState.currentDirectory,
        homeDirectory: completeState.homeDirectory,
      }
    } catch (error) {
      throw new Error(handleError(error, '获取窗口状态失败'))
    }
  }

  /**
   * 批量设置窗口属性
   */
  async setWindowProperties(properties: WindowProperties): Promise<void> {
    const operations = []

    if (properties.alwaysOnTop !== undefined) {
      operations.push({
        operation: 'set_always_on_top' as const,
        params: { alwaysOnTop: properties.alwaysOnTop },
      })
    }

    if (operations.length === 0) {
      return
    }

    const request = { operations }
    const response = await this.manageWindowState(request)

    if (!response.overallSuccess) {
      const errors = response.results
        .filter((r: WindowStateOperationResult) => !r.success)
        .map((r: WindowStateOperationResult) => r.error)
        .join(', ')
      throw new Error(`设置窗口属性失败: ${errors}`)
    }

    // 更新本地状态
    if (properties.alwaysOnTop !== undefined) {
      this.alwaysOnTopState = properties.alwaysOnTop
    }
  }

  /**
   * 重置窗口状态
   */
  async resetWindowState(): Promise<void> {
    const request = {
      operations: [
        {
          operation: 'reset_state' as const,
        },
      ],
    }

    const response = await this.manageWindowState(request)
    if (!response.overallSuccess) {
      throw new Error('重置窗口状态失败')
    }

    // 重置本地状态
    this.alwaysOnTopState = false
    this.currentDirectory = null
    this.directoryCache = null
  }

  /**
   * 安全的路径操作（带错误处理）
   */
  async safePathOperation<T>(operation: () => Promise<T>): Promise<PathOperationResult<T>> {
    try {
      const data = await operation()
      return { success: true, data }
    } catch (error) {
      return {
        success: false,
        error: handleError(error),
      }
    }
  }
}

/**
 * 窗口API实例
 */
export const windowAPI = new WindowAPI()

/**
 * 便捷的窗口操作函数
 */
export const window = {
  // 窗口状态
  setAlwaysOnTop: (alwaysOnTop: boolean) => windowAPI.setAlwaysOnTop(alwaysOnTop),
  toggleAlwaysOnTop: () => windowAPI.toggleAlwaysOnTop(),
  getAlwaysOnTopState: () => windowAPI.getAlwaysOnTopState(),

  // 目录操作
  getCurrentDir: (options?: DirectoryOptions) => windowAPI.getCurrentDirectory(options),
  refreshDir: () => windowAPI.refreshCurrentDirectory(),
  getHomeDir: () => windowAPI.getHomeDirectory(),
  clearDirCache: () => windowAPI.clearDirectoryCache(),

  // 路径操作
  normalizePath: (path: string) => windowAPI.normalizePath(path),
  resolveRelativePath: (relativePath: string, options?: PathResolveOptions) =>
    windowAPI.resolveRelativePath(relativePath, options),
  joinPaths: (...paths: string[]) => windowAPI.joinPaths(...paths),
  pathExists: (path: string) => windowAPI.pathExists(path),
  directoryExists: (path: string) => windowAPI.directoryExists(path),
  getPathInfo: (path: string) => windowAPI.getPathInfo(path),

  // 路径工具（本地处理）
  isAbsolutePath: (path: string) => windowAPI.isAbsolutePath(path),
  getParentDirectory: (path: string) => windowAPI.getParentDirectory(path),
  getFileName: (path: string) => windowAPI.getFileName(path),

  // 平台信息
  getPlatformInfo: () => windowAPI.getPlatformInfo(),
  isMac: () => windowAPI.isMac(),

  // 高级功能
  getState: () => windowAPI.getWindowState(),
  getCompleteState: () => windowAPI.getCompleteWindowState(),
  setProperties: (properties: WindowProperties) => windowAPI.setWindowProperties(properties),
  reset: () => windowAPI.resetWindowState(),
  safeOperation: <T>(operation: () => Promise<T>) => windowAPI.safePathOperation(operation),

  // 批量操作方法
  manageState: (request: WindowStateBatchRequest) => windowAPI.manageWindowState(request),
}

// 重新导出类型
export type * from './types'
