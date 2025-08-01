/**
 * API模块主入口文件
 *
 * 重新导出所有API模块，提供统一的访问接口
 */

// 重新导出请求相关功能
export * from '../utils/request'

// 导出各个功能模块
export * from './ai' // AI功能模块
export * from './completion' // 补全功能模块
export * from './config' // 配置管理模块
export * from './shell' // Shell管理模块
export * from './shortcuts' // 快捷键管理模块
export * from './storage' // 存储管理模块
export * from './terminal' // 终端管理模块
export * from './terminal-config' // 终端配置管理模块
export * from './window' // 窗口管理模块

// 类型定义现在统一从 @/types 导入，不在此处重复导出
