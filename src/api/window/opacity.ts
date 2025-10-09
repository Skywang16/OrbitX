/**
 * 窗口透明度 API
 *
 * 提供类型安全的透明度管理接口
 */

import { invoke } from '@/utils/request'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/**
 * 透明度变化事件的 payload 结构
 */
export interface OpacityChangedPayload {
  opacity: number
}

/**
 * 设置窗口透明度
 * @param opacity 透明度值 (0.05 - 1.0)
 */
export async function setWindowOpacity(opacity: number): Promise<void> {
  if (opacity < 0.05 || opacity > 1.0) {
    throw new Error('透明度值必须在 0.05 到 1.0 之间')
  }
  await invoke<void>('window_set_opacity', { opacity })
}

/**
 * 获取当前窗口透明度
 * @returns 当前透明度值
 */
export async function getWindowOpacity(): Promise<number> {
  return await invoke<number>('window_get_opacity')
}

/**
 * 重置窗口透明度为完全不透明
 */
export async function resetWindowOpacity(): Promise<void> {
  await setWindowOpacity(1.0)
}

/**
 * 监听透明度变化事件
 * @param callback 透明度变化时的回调函数
 * @returns 取消监听的函数
 */
export async function onOpacityChanged(callback: (opacity: number) => void): Promise<UnlistenFn> {
  return await listen<OpacityChangedPayload>('opacity-changed', event => {
    callback(event.payload.opacity)
  })
}
