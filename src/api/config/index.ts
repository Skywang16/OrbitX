import { invoke } from '@/utils/request'
import type { AppConfig, ConfigFileInfo, Theme, ThemeInfo, ThemeConfigStatus } from './types'

class ThemeAPI {
  async getThemeConfigStatus(): Promise<ThemeConfigStatus> {
    return await invoke<ThemeConfigStatus>('theme_get_config_status')
  }

  async getCurrentTheme(): Promise<Theme> {
    return await invoke<Theme>('theme_get_current')
  }

  async getAvailableThemes(): Promise<ThemeInfo[]> {
    return await invoke<ThemeInfo[]>('theme_get_available')
  }

  async setTerminalTheme(name: string): Promise<void> {
    await invoke<void>('theme_set_terminal', { themeName: name })
  }

  async setFollowSystemTheme(followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> {
    await invoke<void>('theme_set_follow_system', {
      followSystem,
      lightTheme: lightTheme || null,
      darkTheme: darkTheme || null,
    })
  }
}

export class ConfigApi {
  private themeAPI = new ThemeAPI()

  async getConfig(): Promise<AppConfig> {
    return await invoke<AppConfig>('config_get')
  }

  async updateConfig(config: AppConfig): Promise<void> {
    await invoke<void>('config_update', { newConfig: config })
  }

  async saveConfig(): Promise<void> {
    await invoke<void>('config_save')
  }

  async validateConfig(): Promise<void> {
    await invoke<void>('config_validate')
  }

  async resetConfig(): Promise<void> {
    await invoke<void>('reset_config')
  }

  async reloadConfig(): Promise<void> {
    await invoke<void>('reload_config')
  }

  async getConfigFileInfo(): Promise<ConfigFileInfo> {
    return await invoke<ConfigFileInfo>('config_get_file_info')
  }

  async exportConfig(): Promise<string> {
    return await invoke<string>('export_config')
  }

  async importConfig(configData: string): Promise<void> {
    await invoke<void>('import_config', { data: configData })
  }

  async resetToDefaults(): Promise<void> {
    await invoke<void>('config_reset_to_defaults')
  }

  async getFilePath(): Promise<string> {
    return await invoke<string>('config_get_file_path')
  }

  async getFileInfo(): Promise<ConfigFileInfo> {
    return await invoke<ConfigFileInfo>('config_get_file_info')
  }

  async openFile(): Promise<void> {
    await invoke<void>('config_open_file')
  }

  async getConfigFolderPath(): Promise<string> {
    return await invoke<string>('config_get_folder_path')
  }

  async openConfigFolder(): Promise<void> {
    await invoke<void>('config_open_folder')
  }

  async getThemeConfigStatus(): Promise<ThemeConfigStatus> {
    return this.themeAPI.getThemeConfigStatus()
  }

  async getCurrentTheme(): Promise<Theme> {
    return this.themeAPI.getCurrentTheme()
  }

  async getAvailableThemes(): Promise<ThemeInfo[]> {
    return this.themeAPI.getAvailableThemes()
  }

  async setTerminalTheme(name: string): Promise<void> {
    return this.themeAPI.setTerminalTheme(name)
  }

  async setFollowSystemTheme(followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> {
    return this.themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme)
  }

  get theme() {
    return this.themeAPI
  }
}

export const configApi = new ConfigApi()

export const themeAPI = configApi.theme

export type * from './types'
export { ConfigApiError } from './types'

export default configApi
