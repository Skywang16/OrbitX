/**
 * 窗口管理 API
 *
 * 提供窗口管理的统一接口，包括：
 * - 窗口状态管理
 * - 目录操作
 * - 路径处理
 */

import { createMessage } from '@/ui'
import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type {
  CompleteWindowState,
  DirectoryOptions,
  PathInfo,
  PlatformInfo,
  WindowState,
  WindowStateBatchRequest,
  WindowStateBatchResponse,
} from './types'

/**
 * 窗口 API 接口类
 */
export class WindowApi {
  private alwaysOnTopState = false
  private platformInfoCache: PlatformInfo | null = null

  // ===== 窗口状态管理 =====

  async manageWindowState(request: WindowStateBatchRequest): Promise<WindowStateBatchResponse> {
    try {
      return await invoke<WindowStateBatchResponse>('manage_window_state', { request })
    } catch (error) {
      throw new Error(handleError(error, '批量窗口状态管理失败'))
    }
  }

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
      this.alwaysOnTopState = state.alwaysOnTop
      return state
    }
    throw new Error('获取窗口状态失败')
  }

  async setAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
    const request = {
      operations: [
        {
          operation: 'set_always_on_top' as const,
          params: { alwaysOnTop },
        },
      ],
    }

    const response = await this.manageWindowState(request)
    if (!response.overallSuccess) {
      throw new Error('设置窗口置顶失败')
    }

    this.alwaysOnTopState = alwaysOnTop
  }

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

  getAlwaysOnTopState(): boolean {
    return this.alwaysOnTopState
  }

  // ===== 目录操作 =====

  async getCurrentDirectory(options: DirectoryOptions = {}): Promise<string> {
    try {
      return await invoke<string>('get_current_directory', { useCache: options.useCache })
    } catch (error) {
      throw new Error(handleError(error, '获取当前目录失败'))
    }
  }

  async getHomeDirectory(): Promise<string> {
    try {
      return await invoke<string>('get_home_directory', { forceRefresh: true })
    } catch (error) {
      throw new Error(handleError(error, '获取家目录失败'))
    }
  }

  async clearDirectoryCache(): Promise<void> {
    try {
      await invoke('clear_directory_cache')
      createMessage.success('目录缓存已清除')
    } catch (error) {
      throw new Error(handleError(error, '清除目录缓存失败'))
    }
  }

  // ===== 路径操作 =====

  async pathExists(path: string): Promise<boolean> {
    try {
      return await invoke<boolean>('path_exists', { path })
    } catch (error) {
      console.warn('检查路径存在性失败:', handleError(error))
      return false
    }
  }

  async normalizePath(path: string): Promise<string> {
    try {
      return await invoke<string>('normalize_path', { path })
    } catch (error) {
      throw new Error(handleError(error, '路径规范化失败'))
    }
  }

  async joinPaths(...paths: string[]): Promise<string> {
    try {
      return await invoke<string>('join_paths', { paths })
    } catch (error) {
      throw new Error(handleError(error, '路径连接失败'))
    }
  }

  isAbsolutePath(path: string): boolean {
    if (!path) return false
    if (path.startsWith('/')) return true
    if (path.match(/^[A-Za-z]:/)) return true
    if (path.startsWith('\\\\')) return true
    return false
  }

  getParentDirectory(path: string): string {
    if (!path) return ''
    const normalized = path.replace(/\\/g, '/').replace(/\/+/g, '/')
    const lastSlash = normalized.lastIndexOf('/')
    if (lastSlash <= 0) {
      return '/'
    }
    return normalized.substring(0, lastSlash)
  }

  getFileName(path: string): string {
    if (!path) return ''
    const normalized = path.replace(/\\/g, '/').replace(/\/+/g, '/')
    const lastSlash = normalized.lastIndexOf('/')
    return normalized.substring(lastSlash + 1)
  }

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

  async getPlatformInfo(): Promise<PlatformInfo> {
    if (this.platformInfoCache) {
      return this.platformInfoCache
    }

    try {
      const platformInfo = await invoke<PlatformInfo>('get_platform_info')
      this.platformInfoCache = platformInfo
      return platformInfo
    } catch (error) {
      throw new Error(handleError(error, '获取平台信息失败'))
    }
  }

  async isMac(): Promise<boolean> {
    const platformInfo = await this.getPlatformInfo()
    return platformInfo.is_mac
  }

  // ===== 综合状态 =====

  async getWindowState(): Promise<WindowState> {
    try {
      const completeState = await this.getCompleteWindowState()
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
}

// 导出单例实例
export const windowApi = new WindowApi()

// 导出类型
export type * from './types'

// 默认导出
export default windowApi
