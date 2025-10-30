/**
 * 应用管理 API
 *
 * 提供应用级别的事件监听和管理功能
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/**
 * 应用 API 接口类
 */
export class AppApi {
  /**
   * 监听清空所有标签页事件（macOS 窗口关闭时触发）
   */
  onClearAllTabs = async (callback: () => void | Promise<void>): Promise<UnlistenFn> => {
    return await listen('clear-all-tabs', async () => {
      await callback()
    })
  }

  /**
   * 监听自定义事件
   */
  onCustomEvent = async <T = unknown>(
    eventName: string,
    callback: (payload: T) => void | Promise<void>
  ): Promise<UnlistenFn> => {
    return await listen<T>(eventName, async event => {
      await callback(event.payload)
    })
  }
}

export const appApi = new AppApi()

// 默认导出
export default appApi
