/**
 * 窗口模块相关的类型定义
 */

// ===== 窗口相关类型 =====

export interface WindowAlwaysOnTopOptions {
  alwaysOnTop: boolean
}

// ===== 平台信息类型 =====

export interface PlatformInfo {
  platform: string
  arch: string
  os_version: string
  is_mac: boolean
}

// ===== 目录缓存类型 =====

export interface DirectoryCache {
  value: string
  timestamp: number
}

// ===== 窗口状态类型 =====

export interface WindowState {
  alwaysOnTop: boolean
  currentDirectory: string
  homeDirectory: string
}

// ===== 窗口属性类型 =====

export interface WindowProperties {
  alwaysOnTop?: boolean
}

// ===== 新的批量窗口状态管理类型 =====

/**
 * 窗口状态操作类型
 */
export type WindowStateOperation =
  | 'get_state' // 获取窗口状态
  | 'set_always_on_top' // 设置置顶状态
  | 'toggle_always_on_top' // 切换置顶状态
  | 'reset_state' // 重置窗口状态

/**
 * 窗口状态操作请求
 */
export interface WindowStateOperationRequest {
  operation: WindowStateOperation
  params?: {
    alwaysOnTop?: boolean
    [key: string]: any
  }
}

/**
 * 批量窗口状态操作请求
 */
export interface WindowStateBatchRequest {
  operations: WindowStateOperationRequest[]
}

/**
 * 窗口状态操作结果
 */
export interface WindowStateOperationResult {
  operation: WindowStateOperation
  success: boolean
  data?: any
  error?: string
}

/**
 * 批量窗口状态操作响应
 */
export interface WindowStateBatchResponse {
  results: WindowStateOperationResult[]
  overallSuccess: boolean
}

/**
 * 完整的窗口状态信息
 */
export interface CompleteWindowState {
  alwaysOnTop: boolean
  currentDirectory: string
  homeDirectory: string
  platformInfo: PlatformInfo
  timestamp: number
}

// ===== 路径操作结果类型 =====

export interface PathOperationResult<T = string> {
  success: boolean
  data?: T
  error?: string
}

// ===== 路径信息类型 =====

export interface PathInfo {
  path: string
  exists: boolean
  isAbsolute: boolean
  parent: string
  fileName: string
  normalized: string
}

// ===== 目录操作选项类型 =====

export interface DirectoryOptions {
  useCache?: boolean
  forceRefresh?: boolean
}

// ===== 路径解析选项类型 =====

export interface PathResolveOptions {
  basePath?: string
  normalize?: boolean
}
