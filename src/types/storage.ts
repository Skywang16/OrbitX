/**
 * 存储系统类型定义
 *
 * 与后端存储系统对应的TypeScript类型定义
 */

// ============================================================================
// 基础枚举类型
// ============================================================================

/** 存储层类型 */
export enum StorageLayer {
  /** TOML配置层 */
  Config = 'config',
  /** MessagePack状态层 */
  State = 'state',
  /** SQLite数据层 */
  Data = 'data',
}

/** 缓存层类型 */
export enum CacheLayer {
  /** 内存缓存 */
  Memory = 'memory',
  /** LRU缓存 */
  Lru = 'lru',
  /** 磁盘缓存 */
  Disk = 'disk',
}

/** 配置节类型 */
export enum ConfigSection {
  /** 应用配置 */
  App = 'app',
  /** 外观配置 */
  Appearance = 'appearance',
  /** 终端配置 */
  Terminal = 'terminal',
  /** 快捷键配置 */
  Shortcuts = 'shortcuts',
  /** AI配置 */
  Ai = 'ai',
}

// ============================================================================
// 查询和保存选项
// ============================================================================

/** 数据查询结构 */
export interface DataQuery {
  /** 查询语句或条件 */
  query: string
  /** 查询参数 */
  params: Record<string, any>
  /** 限制结果数量 */
  limit?: number
  /** 偏移量 */
  offset?: number
  /** 排序字段 */
  order_by?: string
  /** 是否降序 */
  desc: boolean
}

/** 保存选项 */
export interface SaveOptions {
  /** 目标表或集合名称 */
  table?: string
  /** 是否覆盖现有数据 */
  overwrite: boolean
  /** 是否创建备份 */
  backup: boolean
  /** 是否验证数据 */
  validate: boolean
  /** 自定义元数据 */
  metadata: Record<string, any>
}

// ============================================================================
// 会话状态相关类型
// ============================================================================

/** 窗口状态 */
export interface WindowState {
  /** 窗口位置 [x, y] */
  position: [number, number]
  /** 窗口大小 [width, height] */
  size: [number, number]
  /** 是否最大化 */
  isMaximized: boolean
  /** 是否全屏 */
  isFullscreen: boolean
  /** 是否置顶 */
  isAlwaysOnTop: boolean
}

/** 标签页状态 */
export interface TabState {
  /** 标签页ID */
  id: string
  /** 标签页标题 */
  title: string
  /** 是否激活 */
  isActive: boolean
  /** 工作目录 */
  workingDirectory: string
  /** 终端会话ID */
  terminalSessionId?: string
  /** 自定义数据 */
  customData: Record<string, any>
}

/** 终端会话状态 */
export interface TerminalSession {
  /** 会话ID */
  id: string
  /** 会话标题 */
  title: string
  /** 工作目录 */
  workingDirectory: string
  /** 环境变量 */
  environment: Record<string, string>
  /** 命令历史 */
  commandHistory: string[]
  /** 是否活跃 */
  isActive: boolean
  /** 创建时间 */
  createdAt: string
  /** 最后活跃时间 */
  lastActive: string
}

/** UI状态 */
export interface UiState {
  /** 侧边栏是否可见 */
  sidebarVisible: boolean
  /** 侧边栏宽度 */
  sidebarWidth: number
  /** 当前主题 */
  currentTheme: string
  /** 字体大小 */
  fontSize: number
  /** 缩放级别 */
  zoomLevel: number
  /** 面板布局 */
  panelLayout: Record<string, any>
  /** OrbitX AI 聊天状态 */
  orbitxChat?: {
    /** 是否可见 */
    isVisible: boolean
    /** 侧边栏宽度 */
    sidebarWidth: number
    /** 当前模式 */
    chatMode: 'chat' | 'agent'
    /** 当前会话ID */
    currentConversationId: number | null
  }
}

/** 会话状态数据结构 */
export interface SessionState {
  /** 版本号 */
  version: number
  /** 窗口状态 */
  windowState: WindowState
  /** 标签页状态 */
  tabs: TabState[]
  /** 终端会话状态 */
  terminalSessions: Record<string, TerminalSession>
  /** UI状态 */
  uiState: UiState
  /** 创建时间 */
  createdAt: string
  /** 校验和 */
  checksum?: string
}

// ============================================================================
// 统计信息类型
// ============================================================================

/** 单层缓存统计 */
export interface LayerStats {
  /** 命中次数 */
  hits: number
  /** 未命中次数 */
  misses: number
  /** 条目数量 */
  entries: number
  /** 内存使用量（字节） */
  memory_usage: number
  /** 平均访问时间（纳秒） */
  avg_access_time: number
}

/** 缓存统计信息 */
export interface CacheStats {
  /** 缓存层统计 */
  layers: Record<CacheLayer, LayerStats>
  /** 总命中率 */
  total_hit_rate: number
  /** 总内存使用量（字节） */
  total_memory_usage: number
  /** 总条目数 */
  total_entries: number
}

/** 存储统计信息 */
export interface StorageStats {
  /** 总大小（字节） */
  total_size: number
  /** 配置层大小 */
  config_size: number
  /** 状态层大小 */
  state_size: number
  /** 数据层大小 */
  data_size: number
  /** 缓存层大小 */
  cache_size: number
  /** 备份大小 */
  backups_size: number
  /** 日志大小 */
  logs_size: number
}

// ============================================================================
// 健康检查和事件类型
// ============================================================================

/** 健康检查结果 */
export interface HealthCheckResult {
  /** 检查项名称 */
  name: string
  /** 是否健康 */
  healthy: boolean
  /** 检查消息 */
  message: string
  /** 检查时间 */
  checked_at: number
  /** 检查耗时（毫秒） */
  duration: number
}

/** 存储事件类型 */
export interface StorageEvent {
  /** 事件类型 */
  type: 'config_changed' | 'state_saved' | 'state_loaded' | 'data_updated' | 'cache_event' | 'error'
  /** 事件数据 */
  data: any
  /** 事件时间戳 */
  timestamp: number
}

// ============================================================================
// 工具类型和辅助函数
// ============================================================================

/** 创建默认的数据查询 */
export const createDataQuery = (query: string): DataQuery => {
  return {
    query,
    params: {},
    desc: false,
  }
}

/** 创建默认的保存选项 */
export const createSaveOptions = (table?: string): SaveOptions => {
  return {
    table,
    overwrite: false,
    backup: true,
    validate: true,
    metadata: {},
  }
}

/** 创建默认的会话状态 */
export const createDefaultSessionState = (): SessionState => {
  return {
    version: 1,
    windowState: {
      position: [100, 100],
      size: [1200, 800],
      isMaximized: false,
      isFullscreen: false,
      isAlwaysOnTop: false,
    },
    tabs: [],
    terminalSessions: {},
    uiState: {
      sidebarVisible: true,
      sidebarWidth: 300,
      currentTheme: 'dark',
      fontSize: 14,
      zoomLevel: 1.0,
      panelLayout: {},
      orbitxChat: {
        isVisible: false,
        sidebarWidth: 350,
        chatMode: 'chat',
        currentConversationId: null,
      },
    },
    createdAt: new Date().toISOString(),
  }
}

/** 格式化字节大小为人类可读的字符串 */
export const formatBytes = (bytes: number): string => {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = bytes
  let unitIndex = 0

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }

  return `${size.toFixed(2)} ${units[unitIndex]}`
}

/** 计算缓存命中率 */
export const calculateHitRate = (hits: number, misses: number): number => {
  if (hits + misses === 0) return 0
  return hits / (hits + misses)
}
