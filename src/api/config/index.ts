import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type { AppConfig, ConfigFileInfo, Theme, ThemeInfo, ThemeConfigStatus } from './types'

class ThemeAPI {
  async getThemeConfigStatus(): Promise<ThemeConfigStatus> {
    try {
      return await invoke<ThemeConfigStatus>('get_theme_config_status')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get theme config status'))
    }
  }

  async getCurrentTheme(): Promise<Theme> {
    try {
      return await invoke<Theme>('get_current_theme')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get current theme'))
    }
  }

  async getAvailableThemes(): Promise<ThemeInfo[]> {
    try {
      return await invoke<ThemeInfo[]>('get_available_themes')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get available themes'))
    }
  }

  async setTerminalTheme(name: string): Promise<void> {
    try {
      await invoke('set_terminal_theme', { themeName: name })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to set terminal theme'))
    }
  }

  async setFollowSystemTheme(followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> {
    try {
      await invoke('set_follow_system_theme', {
        followSystem,
        lightTheme: lightTheme || null,
        darkTheme: darkTheme || null,
      })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to set follow system theme'))
    }
  }
}

export class ConfigApi {
  private themeAPI = new ThemeAPI()

  async getConfig(): Promise<AppConfig> {
    try {
      return await invoke<AppConfig>('get_config')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get config'))
    }
  }

  async updateConfig(config: AppConfig): Promise<void> {
    try {
      await invoke('update_config', { newConfig: config })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to update config'))
    }
  }

  async saveConfig(): Promise<void> {
    try {
      await invoke('save_config')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to save config'))
    }
  }

  async validateConfig(): Promise<void> {
    try {
      await invoke('validate_config')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to validate config'))
    }
  }

  async resetConfig(): Promise<void> {
    try {
      await invoke('reset_config')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to reset config'))
    }
  }

  async reloadConfig(): Promise<void> {
    try {
      await invoke('reload_config')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to reload config'))
    }
  }


  async getConfigFileInfo(): Promise<ConfigFileInfo> {
    try {
      return await invoke<ConfigFileInfo>('get_config_file_info')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get config file info'))
    }
  }

  async exportConfig(): Promise<string> {
    try {
      return await invoke<string>('export_config')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to export config'))
    }
  }

  async importConfig(configData: string): Promise<void> {
    try {
      await invoke('import_config', { data: configData })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to import config'))
    }
  }

  async resetToDefaults(): Promise<void> {
    try {
      await invoke('reset_config_to_defaults')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to reset config to defaults'))
    }
  }

  async getFilePath(): Promise<string> {
    try {
      return await invoke<string>('get_config_file_path')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get config file path'))
    }
  }

  async getFileInfo(): Promise<ConfigFileInfo> {
    try {
      return await invoke<ConfigFileInfo>('get_config_file_info')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get config file info'))
    }
  }

  async openFile(): Promise<void> {
    try {
      await invoke('open_config_file')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to open config file'))
    }
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
