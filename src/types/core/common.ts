/**
 * 核心通用类型定义
 * 不依赖任何业务类型，作为整个类型系统的基础
 */

// ===== 基础通用类型 =====

export type Size = 'small' | 'medium' | 'large'
export type Status = 'idle' | 'loading' | 'success' | 'error'

// ===== 通用操作结果类型 =====

export interface OperationResult<T = void> {
  success: boolean
  data?: T
  error?: string
  timestamp?: string
}

// ===== 通用分页类型 =====

export interface PaginationOptions {
  page?: number
  pageSize?: number
  sortBy?: string
  sortOrder?: 'asc' | 'desc'
}

export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  pageSize: number
  totalPages: number
}

// ===== 通用缓存类型 =====

export interface CacheOptions {
  ttl?: number
  maxSize?: number
  enabled?: boolean
}

export interface BaseCacheStats {
  totalEntries: number
  capacity: number
  expiredEntries: number
  hitRate: number
}

// ===== 通用文件系统类型 =====

export interface FileInfo {
  name: string
  path: string
  isDir: boolean
  size?: number
  modified?: number
}

// ===== 通用系统信息类型 =====

export interface SystemInfo {
  platform: string
  arch: string
  version: string
  homeDir: string
  currentDir: string
}

// ===== 通用进程信息类型 =====

export interface ProcessInfo {
  pid: number
  name: string
  command: string
  status: 'running' | 'stopped' | 'zombie'
}

// ===== 通用权限信息类型 =====

export interface PermissionInfo {
  read: boolean
  write: boolean
  execute: boolean
}

// ===== 通用事件类型 =====

export interface BaseEvent {
  type: string
  timestamp: string
  source?: string
}

// ===== 通用配置类型 =====

export interface BaseConfig {
  version: string
  lastModified: string
  enabled: boolean
}

// ===== 日志相关类型 =====

export interface LogEntry {
  timestamp: string
  level: 'debug' | 'info' | 'warn' | 'error'
  message: string
  module?: string
}

// ===== 插件相关类型 =====

export interface PluginInfo {
  name: string
  version: string
  enabled: boolean
  description?: string
}

// ===== 快捷键相关类型 =====

export interface KeyBinding {
  key: string
  modifiers: string[]
  action: string
  description?: string
}

// ===== 搜索相关类型 =====

export interface SearchOptions {
  query: string
  caseSensitive?: boolean
  regex?: boolean
  wholeWord?: boolean
}

export interface SearchResult {
  file: string
  line: number
  column: number
  text: string
  match: string
}
