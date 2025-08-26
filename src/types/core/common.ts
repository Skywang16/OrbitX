export type Size = 'small' | 'medium' | 'large'
export type Status = 'idle' | 'loading' | 'success' | 'error'

export interface OperationResult<T = void> {
  success: boolean
  data?: T
  error?: string
  timestamp?: string
}

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

export interface FileInfo {
  name: string
  path: string
  isDir: boolean
  size?: number
  modified?: number
}

export interface SystemInfo {
  platform: string
  arch: string
  version: string
  homeDir: string
  currentDir: string
}

export interface ProcessInfo {
  pid: number
  name: string
  command: string
  status: 'running' | 'stopped' | 'zombie'
}

export interface PermissionInfo {
  read: boolean
  write: boolean
  execute: boolean
}

export interface BaseEvent {
  type: string
  timestamp: string
  source?: string
}

export interface BaseConfig {
  version: string
  lastModified: string
  enabled: boolean
}

export interface LogEntry {
  timestamp: string
  level: 'debug' | 'info' | 'warn' | 'error'
  message: string
  module?: string
}

export interface PluginInfo {
  name: string
  version: string
  enabled: boolean
  description?: string
}

export interface KeyBinding {
  key: string
  modifiers: string[]
  action: string
  description?: string
}

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
