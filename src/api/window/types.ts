/**
 * 窗口模块相关的类型定义
 */

export interface PlatformInfo {
  platform: string
  arch: string
  os_version: string
  is_mac: boolean
}

export interface WindowStateSnapshot {
  alwaysOnTop: boolean
  currentDirectory: string
  homeDirectory: string
  platformInfo: PlatformInfo
  opacity: number
  timestamp: number
}

export interface WindowStateUpdate {
  alwaysOnTop?: boolean
  opacity?: number
  refreshDirectories?: boolean
}
