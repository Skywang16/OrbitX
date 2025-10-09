/**
 * 存储管理 API
 *
 * 提供存储系统的统一接口，包括：
 * - 配置管理
 * - 会话状态管理
 * - 数据查询和保存
 */

import { invoke } from '@/utils/request'
import { ConfigSection } from '@/types'
import type {
  SessionState,
  AppSection,
  AppearanceSection,
  TerminalSection,
  ShortcutsSection,
  AiSection,
  ConfigSectionMap,
  RuntimeTerminalState,
} from './types'

/**
 * 存储 API 接口类
 */
export class StorageApi {
  // ===== 配置管理 =====

  async getConfig<S extends ConfigSection>(section: S): Promise<ConfigSectionMap[S]> {
    return await invoke<ConfigSectionMap[S]>('storage_get_config', { section })
  }

  async updateConfig<S extends ConfigSection>(section: S, data: ConfigSectionMap[S]): Promise<void> {
    await invoke<void>('storage_update_config', { section, data })
  }

  // ===== 会话状态管理 =====

  async saveSessionState(sessionState: SessionState): Promise<void> {
    await invoke<void>('storage_save_session_state', { sessionState })
  }

  async loadSessionState(): Promise<SessionState | null> {
    return await invoke<SessionState | null>('storage_load_session_state')
  }

  // ===== 便捷方法 =====

  async getAppConfig(): Promise<AppSection> {
    return this.getConfig(ConfigSection.App)
  }

  async getAppearanceConfig(): Promise<AppearanceSection> {
    return this.getConfig(ConfigSection.Appearance)
  }

  async getTerminalConfig(): Promise<TerminalSection> {
    return this.getConfig(ConfigSection.Terminal)
  }

  async getShortcutsConfig(): Promise<ShortcutsSection> {
    return this.getConfig(ConfigSection.Shortcuts)
  }

  async getAiConfig(): Promise<AiSection> {
    return this.getConfig(ConfigSection.Ai)
  }

  async updateAppConfig(data: AppSection): Promise<void> {
    return this.updateConfig(ConfigSection.App, data)
  }

  async updateAppearanceConfig(data: AppearanceSection): Promise<void> {
    return this.updateConfig(ConfigSection.Appearance, data)
  }

  async updateTerminalConfig(data: TerminalSection): Promise<void> {
    return this.updateConfig(ConfigSection.Terminal, data)
  }

  async updateShortcutsConfig(data: ShortcutsSection): Promise<void> {
    return this.updateConfig(ConfigSection.Shortcuts, data)
  }

  async updateAiConfig(data: AiSection): Promise<void> {
    return this.updateConfig(ConfigSection.Ai, data)
  }

  // ===== 终端状态管理（新增：后端唯一数据源） =====

  /**
   * 获取所有终端的运行时状态（包含实时 CWD）
   *
   * 设计说明：
   * - 直接从后端 ShellIntegration 查询实时 CWD
   * - 不依赖前端缓存，确保数据准确性
   * - 用于应用启动、会话恢复、前端同步等场景
   */
  async getTerminalsState(): Promise<RuntimeTerminalState[]> {
    return await invoke<RuntimeTerminalState[]>('storage_get_terminals_state')
  }

  /**
   * 获取指定终端的当前工作目录
   *
   * @param paneId 后端 pane ID
   */
  async getTerminalCwd(paneId: number): Promise<string> {
    return await invoke<string>('storage_get_terminal_cwd', { paneId })
  }
}

export const storageApi = new StorageApi()
export type * from './types'
export default storageApi
