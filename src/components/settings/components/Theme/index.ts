/**
 * 主题设置模块统一导出
 */

// 导出主题相关的组合函数（从统一配置系统）
export { useConfigTheme as useThemeSettingsStore } from '../../../../composables/useConfig'

// 导出主组件
export { default as ThemeSettings } from './ThemeSettings.vue'
