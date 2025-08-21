/**
 * 快捷键系统统一入口
 *
 * 统一导出所有快捷键相关的功能
 */

// 快捷键监听器
export { useShortcutListener } from './listener'

// 快捷键动作服务
export { shortcutActionsService, ShortcutActionsService } from './actions'

// 工具函数
export * from './utils'

// 常量定义
export * from './constants'

// 类型已迁移到统一类型系统 @/types

// 重新导出快捷键 API 和类型
export { shortcutsApi, ShortcutsApi } from '@/api/shortcuts'
