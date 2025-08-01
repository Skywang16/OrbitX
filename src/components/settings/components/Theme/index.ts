/**
 * 主题设置模块统一导出
 */

// 导出主题相关的组合函数（从统一主题系统）
export { useTheme as useThemeSettingsStore } from '@/composables/useTheme'

// 导出主组件
export { default as ThemeSettings } from './ThemeSettings.vue'
