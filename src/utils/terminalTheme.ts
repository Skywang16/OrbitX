import { themeAPI } from '@/api/config'
import { convertThemeToXTerm, createDefaultXTermTheme, type XTermTheme } from './themeConverter'

export const getTerminalThemeFromConfig = async (): Promise<XTermTheme> => {
  try {
    const theme = await themeAPI.getCurrentTheme()

    return convertThemeToXTerm(theme)
  } catch (error) {
    return createDefaultXTermTheme()
  }
}
