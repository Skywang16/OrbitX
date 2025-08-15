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
  params: Record<string, unknown>
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
  metadata: Record<string, unknown>
}

// ============================================================================
// 会话状态相关类型 - 精简版，与后端完全一致
// ============================================================================

/** 窗口状态 */
export interface WindowState {
  /** X坐标 */
  x: number
  /** Y坐标 */
  y: number
  /** 宽度 */
  width: number
  /** 高度 */
  height: number
  /** 是否最大化 */
  maximized: boolean
}

/** 终端状态 */
export interface TerminalState {
  /** 终端ID */
  id: string
  /** 终端标题 */
  title: string
  /** 工作目录 */
  cwd: string
  /** 是否激活 */
  active: boolean
  /** Shell类型（可选） */
  shell?: string
}

/** UI状态 */
export interface UiState {
  /** 主题名称 */
  theme: string
  /** 字体大小 */
  fontSize: number
  /** 侧边栏宽度 */
  sidebarWidth: number
}

/** AI状态 */
export interface AiState {
  /** 是否可见 */
  visible: boolean
  /** 侧边栏宽度 */
  width: number
  /** 聊天模式 */
  mode: 'chat' | 'agent'
  /** 当前会话ID */
  conversationId?: number
}

/** 会话状态数据结构 */
export interface SessionState {
  /** 版本号 */
  version: number
  /** 窗口状态 */
  window: WindowState
  /** 终端状态列表 */
  terminals: TerminalState[]
  /** 当前活跃的标签页ID */
  activeTabId?: string
  /** UI状态 */
  ui: UiState
  /** AI状态 */
  ai: AiState
  /** 时间戳 */
  timestamp: string
}

// ============================================================================
// 事件类型
// ============================================================================

/** 存储事件类型 */
export interface StorageEvent {
  /** 事件类型 */
  type: 'config_changed' | 'state_saved' | 'state_loaded' | 'data_updated' | 'cache_event' | 'error'
  /** 事件数据 */
  data: unknown
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
    window: {
      x: 100,
      y: 100,
      width: 1200,
      height: 800,
      maximized: false,
    },
    terminals: [],
    activeTabId: undefined,
    ui: {
      theme: 'dark',
      fontSize: 14,
      sidebarWidth: 300,
    },
    ai: {
      visible: false,
      width: 350,
      mode: 'chat',
      conversationId: undefined,
    },
    timestamp: new Date().toISOString(),
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
