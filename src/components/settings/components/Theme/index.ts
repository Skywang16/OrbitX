/**
 * 主题设置模块统一导出
 */

// 导出主题相关的组合函数（从统一主题系统）
export { useThemeStore as useThemeSettingsStore } from '@/stores/theme'

// 导出主组件
export { default as ThemeSettings } from './ThemeSettings.vue'
