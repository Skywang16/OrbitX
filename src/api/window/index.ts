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
import { useI18n } from 'vue-i18n'
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
    return await invoke<WindowStateBatchResponse>('manage_window_state', { request })
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
    return await invoke<string>('get_current_directory', { useCache: options.useCache })
  }

  async getHomeDirectory(): Promise<string> {
    return await invoke<string>('get_home_directory', { forceRefresh: true })
  }

  async clearDirectoryCache(): Promise<void> {
    await invoke<void>('clear_directory_cache')
    createMessage.success(useI18n().t('cache.directory_cleared'))
  }

  // ===== 路径操作 =====

  async pathExists(path: string): Promise<boolean> {
    return await invoke<boolean>('path_exists', { path })
  }

  async normalizePath(path: string): Promise<string> {
    return await invoke<string>('normalize_path', { path })
  }

  async joinPaths(...paths: string[]): Promise<string> {
    return await invoke<string>('join_paths', { paths })
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
    const [exists, normalized] = await Promise.all([this.pathExists(path), this.normalizePath(path)])

    return {
      path,
      exists,
      isAbsolute: this.isAbsolutePath(path),
      parent: this.getParentDirectory(path),
      fileName: this.getFileName(path),
      normalized,
    }
  }

  // ===== 平台信息 =====

  async getPlatformInfo(): Promise<PlatformInfo> {
    if (this.platformInfoCache) {
      return this.platformInfoCache
    }

    const platformInfo = await invoke<PlatformInfo>('get_platform_info')
    this.platformInfoCache = platformInfo
    return platformInfo
  }

  async isMac(): Promise<boolean> {
    const platformInfo = await this.getPlatformInfo()
    return platformInfo.is_mac
  }

  // ===== 综合状态 =====

  async getWindowState(): Promise<WindowState> {
    const completeState = await this.getCompleteWindowState()
    this.alwaysOnTopState = completeState.alwaysOnTop

    return {
      alwaysOnTop: completeState.alwaysOnTop,
      currentDirectory: completeState.currentDirectory,
      homeDirectory: completeState.homeDirectory,
    }
  }

  // ===== 透明度管理 =====

  async setWindowOpacity(opacity: number): Promise<void> {
    if (opacity < 0 || opacity > 1) {
      throw new Error('透明度值必须在 0 到 1 之间')
    }

    await invoke<void>('set_window_opacity', { opacity })
  }

  async getWindowOpacity(): Promise<number> {
    const opacity = await invoke<number>('get_window_opacity')
    return opacity
  }

  async resetWindowOpacity(): Promise<void> {
    await this.setWindowOpacity(1.0)
  }

  // ===== 文件处理 =====

  async handleFileOpen(path: string): Promise<string> {
    return await invoke<string>('handle_file_open', { path })
  }
}

// 导出单例实例
export const windowApi = new WindowApi()

// 导出类型
export type * from './types'

// 默认导出
export default windowApi
