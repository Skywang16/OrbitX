/**
 * 窗口管理 API
 *
 * 提供窗口管理的统一接口，包括：
 * - 窗口状态管理
 * - 目录操作
 * - 路径处理
 * - 文件拖放事件监听
 */

import { createMessage } from '@/ui'
import { invoke } from '@/utils/request'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
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

  manageWindowState = async (request: WindowStateBatchRequest): Promise<WindowStateBatchResponse> => {
    return await invoke<WindowStateBatchResponse>('window_manage_state', { request })
  }

  getCompleteWindowState = async (): Promise<CompleteWindowState> => {
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
      if (state && typeof state === 'object' && 'alwaysOnTop' in state) {
        this.alwaysOnTopState = state.alwaysOnTop
        return state as CompleteWindowState
      }
    }
    throw new Error('获取窗口状态失败')
  }

  setAlwaysOnTop = async (alwaysOnTop: boolean): Promise<void> => {
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

  toggleAlwaysOnTop = async (): Promise<boolean> => {
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
      if (typeof newState === 'boolean') {
        this.alwaysOnTopState = newState
        return newState
      }
    }
    throw new Error('切换窗口置顶状态失败')
  }

  getAlwaysOnTopState = (): boolean => {
    return this.alwaysOnTopState
  }

  // ===== 目录操作 =====

  getCurrentDirectory = async (options: DirectoryOptions = {}): Promise<string> => {
    return await invoke<string>('window_get_current_directory', { useCache: options.useCache })
  }

  getHomeDirectory = async (): Promise<string> => {
    return await invoke<string>('window_get_home_directory', { forceRefresh: true })
  }

  clearDirectoryCache = async (): Promise<void> => {
    await invoke<void>('window_clear_directory_cache')
    createMessage.success(useI18n().t('cache.directory_cleared'))
  }

  // ===== 路径操作 =====

  pathExists = async (path: string): Promise<boolean> => {
    return await invoke<boolean>('window_path_exists', { path })
  }

  normalizePath = async (path: string): Promise<string> => {
    return await invoke<string>('window_normalize_path', { path })
  }

  joinPaths = async (...paths: string[]): Promise<string> => {
    return await invoke<string>('window_join_paths', { paths })
  }

  isAbsolutePath = (path: string): boolean => {
    if (!path) return false
    if (path.startsWith('/')) return true
    if (path.match(/^[A-Za-z]:/)) return true
    if (path.startsWith('\\\\')) return true
    return false
  }

  getParentDirectory = (path: string): string => {
    if (!path) return ''
    const normalized = path.replace(/\\/g, '/').replace(/\/+/g, '/')
    const lastSlash = normalized.lastIndexOf('/')
    if (lastSlash <= 0) {
      return '/'
    }
    return normalized.substring(0, lastSlash)
  }

  getFileName = (path: string): string => {
    if (!path) return ''
    const normalized = path.replace(/\\/g, '/').replace(/\/+/g, '/')
    const lastSlash = normalized.lastIndexOf('/')
    return normalized.substring(lastSlash + 1)
  }

  getPathInfo = async (path: string): Promise<PathInfo> => {
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

  getPlatformInfo = async (): Promise<PlatformInfo> => {
    if (this.platformInfoCache) {
      return this.platformInfoCache
    }

    const platformInfo = await invoke<PlatformInfo>('window_get_platform_info')
    this.platformInfoCache = platformInfo
    return platformInfo
  }

  isMac = async (): Promise<boolean> => {
    const platformInfo = await this.getPlatformInfo()
    return platformInfo.is_mac
  }

  // ===== 综合状态 =====

  getWindowState = async (): Promise<WindowState> => {
    const completeState = await this.getCompleteWindowState()
    this.alwaysOnTopState = completeState.alwaysOnTop

    return {
      alwaysOnTop: completeState.alwaysOnTop,
      currentDirectory: completeState.currentDirectory,
      homeDirectory: completeState.homeDirectory,
    }
  }

  // ===== 透明度管理 =====

  setWindowOpacity = async (opacity: number): Promise<void> => {
    if (opacity < 0 || opacity > 1) {
      throw new Error('透明度值必须在 0 到 1 之间')
    }

    await invoke<void>('window_set_opacity', { opacity })
  }

  getWindowOpacity = async (): Promise<number> => {
    const opacity = await invoke<number>('window_get_opacity')
    return opacity
  }

  resetWindowOpacity = async (): Promise<void> => {
    await this.setWindowOpacity(1.0)
  }

  // ===== 文件处理 =====

  handleFileOpen = async (path: string): Promise<string> => {
    return await invoke<string>('file_handle_open', { path })
  }

  // ===== 事件监听 =====

  /**
   * 监听启动文件事件
   */
  onStartupFile = async (callback: (filePath: string) => void): Promise<UnlistenFn> => {
    return await listen<string>('startup-file', event => {
      callback(event.payload)
    })
  }

  /**
   * 监听文件拖放事件（应用图标拖放）
   */
  onFileDropped = async (callback: (filePath: string) => void): Promise<UnlistenFn> => {
    return await listen<string>('file-dropped', event => {
      callback(event.payload)
    })
  }

  /**
   * 监听窗口拖放事件
   */
  onWindowDragDrop = async (callback: (filePath: string) => void): Promise<UnlistenFn> => {
    const webview = getCurrentWebview()
    return await webview.onDragDropEvent(event => {
      if (
        event.event === 'tauri://drag-drop' &&
        event.payload &&
        'paths' in event.payload &&
        event.payload.paths &&
        event.payload.paths.length > 0
      ) {
        callback(event.payload.paths[0])
      }
    })
  }
}

export const windowApi = new WindowApi()

export type * from './types'

// 默认导出
export default windowApi
