import { invoke } from '@/utils/request'
import type { AppConfig, ConfigFileInfo, Theme, ThemeInfo, ThemeConfigStatus } from './types'

class ThemeAPI {
  async getThemeConfigStatus(): Promise<ThemeConfigStatus> {
    return await invoke<ThemeConfigStatus>('get_theme_config_status')
  }

  async getCurrentTheme(): Promise<Theme> {
    return await invoke<Theme>('get_current_theme')
  }

  async getAvailableThemes(): Promise<ThemeInfo[]> {
    return await invoke<ThemeInfo[]>('get_available_themes')
  }

  async setTerminalTheme(name: string): Promise<void> {
    await invoke<void>('set_terminal_theme', { themeName: name })
  }

  async setFollowSystemTheme(followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> {
    await invoke<void>('set_follow_system_theme', {
      followSystem,
      lightTheme: lightTheme || null,
      darkTheme: darkTheme || null,
    })
  }
}

export class ConfigApi {
  private themeAPI = new ThemeAPI()

  async getConfig(): Promise<AppConfig> {
    return await invoke<AppConfig>('get_config')
  }

  async updateConfig(config: AppConfig): Promise<void> {
    await invoke<void>('update_config', { newConfig: config })
  }

  async saveConfig(): Promise<void> {
    await invoke<void>('save_config')
  }

  async validateConfig(): Promise<void> {
    await invoke<void>('validate_config')
  }

  async resetConfig(): Promise<void> {
    await invoke<void>('reset_config')
  }

  async reloadConfig(): Promise<void> {
    await invoke<void>('reload_config')
  }

  async getConfigFileInfo(): Promise<ConfigFileInfo> {
    return await invoke<ConfigFileInfo>('get_config_file_info')
  }

  async exportConfig(): Promise<string> {
    return await invoke<string>('export_config')
  }

  async importConfig(configData: string): Promise<void> {
    await invoke<void>('import_config', { data: configData })
  }

  async resetToDefaults(): Promise<void> {
    await invoke<void>('reset_config_to_defaults')
  }

  async getFilePath(): Promise<string> {
    return await invoke<string>('get_config_file_path')
  }

  async getFileInfo(): Promise<ConfigFileInfo> {
    return await invoke<ConfigFileInfo>('get_config_file_info')
  }

  async openFile(): Promise<void> {
    await invoke<void>('open_config_file')
  }

  async getConfigFolderPath(): Promise<string> {
    return await invoke<string>('get_config_folder_path')
  }

  async openConfigFolder(): Promise<void> {
    await invoke<void>('open_config_folder')
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
