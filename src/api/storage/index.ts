/**
 * 存储管理 API
 *
 * 职责边界：只处理 State/Data/Runtime。
 * - State：msgpack 会话状态
 * - Runtime：后端 Mux 运行时终端状态（例如实时 CWD）
 *
 * Config(TOML) 请走 `src/api/config`（避免两套写入入口）。
 */

import { invoke } from '@/utils/request'
import type { SessionState, RuntimeTerminalState } from './types'

/**
 * 存储 API 接口类
 */
export class StorageApi {
  // ===== 会话状态管理 =====

  saveSessionState = async (sessionState: SessionState): Promise<void> => {
    await invoke<void>('storage_save_session_state', { sessionState })
  }

  loadSessionState = async (): Promise<SessionState | null> => {
    return await invoke<SessionState | null>('storage_load_session_state')
  }

  // ===== 终端状态管理 =====

  /** 获取所有终端的运行时状态 */
  getTerminalsState = async (): Promise<RuntimeTerminalState[]> => {
    return await invoke<RuntimeTerminalState[]>('storage_get_terminals_state')
  }

  /** 获取单个终端的运行时状态 */
  getTerminalState = async (paneId: number): Promise<RuntimeTerminalState | null> => {
    return await invoke<RuntimeTerminalState | null>('storage_get_terminal_state', { paneId })
  }

  /** 获取指定终端的当前工作目录 */
  getTerminalCwd = async (paneId: number): Promise<string> => {
    return await invoke<string>('storage_get_terminal_cwd', { paneId })
  }
}

export const storageApi = new StorageApi()
export type * from './types'
export default storageApi
