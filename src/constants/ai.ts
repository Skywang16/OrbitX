/**
 * AI相关的常量配置
 */

// ===== 缓存配置 =====
export const AI_CACHE_CONFIG = {
  /** 缓存持续时间（毫秒） */
  DURATION: 5 * 60 * 1000, // 5分钟
  /** 模型列表缓存持续时间 */
  MODELS_DURATION: 5 * 60 * 1000,
} as const

// ===== 会话配置 =====
export const AI_SESSION_CONFIG = {
  /** 最大保存会话数量 */
  MAX_SESSIONS: 50,
  /** 会话标题最大长度 */
  TITLE_MAX_LENGTH: 30,
  /** 存储键名 */
  STORAGE_KEY: 'ai-chat-sessions',
} as const

// ===== 流式传输配置 =====
export const AI_STREAMING_CONFIG = {
  /** 默认超时时间 */
  DEFAULT_TIMEOUT: 30000,
  /** 最大重试次数 */
  MAX_RETRIES: 3,
  /** 重试延迟 */
  RETRY_DELAY: 1000,
} as const

// ===== 消息配置 =====
export const AI_MESSAGE_CONFIG = {
  /** 代码块ID前缀 */
  CODE_BLOCK_ID_PREFIX: 'code',
  /** 复制按钮文本配置 */
  COPY_BUTTON_TEXT: {
    IDLE: '复制',
    COPYING: '复制中...',
    SUCCESS: '已复制',
    ERROR: '复制失败',
  },
  /** 复制状态重置延迟 */
  COPY_RESET_DELAY: 2000,
} as const
