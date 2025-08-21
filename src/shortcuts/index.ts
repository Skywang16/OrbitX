/**
 * 快捷键系统入口
 */

export { useShortcutListener } from './listener'
export { shortcutActionsService } from './actions'
export * from './constants'
export * from './utils'

// 类型已迁移到统一类型系统 @/types

// 重新导出快捷键 API 和类型
export { shortcutsApi, ShortcutsApi } from '@/api/shortcuts'
