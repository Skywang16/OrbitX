/**
 * TermX 统一配置系统 - 类型定义
 *
 * 定义与后端一致的 TypeScript 类型，确保前后端类型安全。
 *
 * @module config/types
 */

// ============================================================================
// 主配置结构
// ============================================================================

/**
 * 主配置结构
 *
 * 与后端 AppConfig 结构体保持一致
 */
export interface AppConfig {
  /** 配置版本 */
  version: string
  /** 配置元数据 */
  metadata?: ConfigMetadata
  /** 应用配置 */
  app: AppConfigApp
  /** 外观配置 */
  appearance: AppearanceConfig
  /** 终端配置 */
  terminal: TerminalConfig
  /** AI 配置 */
  ai: AIConfig
  /** 快捷键配置 */
  shortcuts: ShortcutsConfig
}

// ============================================================================
// 配置元数据
// ============================================================================

/**
 * 配置元数据 (对应后端 ConfigMetadata)
 */
export interface ConfigMetadata {
  /** 创建时间 */
  createdAt: string
  /** 最后修改时间 */
  modifiedAt: string
  /** 配置版本 */
  version: string
  /** 校验和 */
  checksum: string
  /** 备份信息 */
  backupInfo?: BackupInfo
}

/**
 * 备份信息
 */
export interface BackupInfo {
  /** 备份数量 */
  count: number
  /** 最后备份时间 */
  lastBackup: string
}

// ============================================================================
// 应用配置
// ============================================================================

/**
 * 应用配置 (对应后端 AppConfigApp)
 */
export interface AppConfigApp {
  /** 界面语言 */
  language: string
  /** 退出时确认 */
  confirm_on_exit: boolean
  /** 启动行为 */
  startup_behavior: string
}

// ============================================================================
// 外观配置
// ============================================================================

/**
 * 外观配置 (对应后端 AppearanceConfig)
 */
export interface AppearanceConfig {
  /** UI 缩放比例 */
  ui_scale: number
  /** 启用动画 */
  animations_enabled: boolean
  /** 主题配置 */
  theme_config: ThemeConfig
  /** 字体配置 */
  font: FontConfig
}

// ============================================================================
// 终端配置
// ============================================================================

/**
 * 终端配置 (对应后端 TerminalConfig)
 */
export interface TerminalConfig {
  /** 滚动缓冲区行数 */
  scrollback: number
  /** Shell 配置 */
  shell: ShellConfig
  /** 光标配置 */
  cursor: CursorConfig
  /** 终端行为配置 */
  behavior: TerminalBehaviorConfig
}

/**
 * Shell 配置 (对应后端 ShellConfig)
 */
export interface ShellConfig {
  /** 默认 shell */
  default: string
  /** shell 参数 */
  args: string[]
  /** 工作目录 */
  workingDirectory: string
}

/**
 * 终端行为配置 (对应后端 TerminalBehaviorConfig)
 */
export interface TerminalBehaviorConfig {
  /** 进程退出时关闭 */
  closeOnExit: boolean
  /** 关闭时确认 */
  confirmClose: boolean
}

/**
 * 字体配置 (对应后端 FontConfig)
 */
export interface FontConfig {
  /** 字体族 */
  family: string
  /** 字体大小 */
  size: number
  /** 字体粗细 */
  weight: FontWeight
  /** 字体样式 */
  style: FontStyle
  /** 行高 */
  line_height: number
  /** 字符间距 */
  letter_spacing: number
}

/**
 * 字体粗细
 */
export type FontWeight = 'thin' | 'light' | 'normal' | 'medium' | 'bold' | 'black'

/**
 * 字体样式
 */
export type FontStyle = 'normal' | 'italic' | 'oblique'

/**
 * 光标配置 (对应后端 CursorConfig)
 */
export interface CursorConfig {
  /** 光标样式 */
  style: CursorStyle
  /** 光标闪烁 */
  blink: boolean
  /** 光标颜色 */
  color: string
  /** 光标厚度 */
  thickness: number
}

/**
 * 光标样式
 */
export type CursorStyle = 'block' | 'underline' | 'beam'

/**
 * 滚动配置
 */
export interface ScrollingConfig {
  /** 历史行数 */
  historySize: number
  /** 滚动步长 */
  scrollStep: number
  /** 启用平滑滚动 */
  smoothScrolling: boolean
  /** 自动滚动到底部 */
  autoScrollToBottom: boolean
}

/**
 * 选择配置
 */
export interface SelectionConfig {
  /** 选择颜色 */
  color: string
  /** 选择不透明度 */
  opacity: number
  /** 启用语义选择 */
  semanticSelection: boolean
  /** 三击选择整行 */
  tripleClickSelectLine: boolean
}

// ============================================================================
// AI 配置
// ============================================================================

/**
 * AI 配置 (对应后端 AIConfig)
 */
export interface AIConfig {
  /** AI 模型配置数组 */
  models: AISimpleModelConfig[]
  /** 功能配置 */
  features: AIFeaturesConfig
}

/**
 * AI 简单模型配置 (对应后端 AISimpleModelConfig)
 */
export interface AISimpleModelConfig {
  /** 模型名称 */
  name: string
  /** 提供商 */
  provider: string
  /** 是否启用 */
  enabled: boolean
}

/**
 * AI 功能配置 (对应后端 AIFeaturesConfig)
 */
export interface AIFeaturesConfig {
  /** 聊天功能配置 */
  chat: AIChatFeatureConfig
}

/**
 * AI 聊天功能配置 (对应后端 AIChatFeatureConfig)
 */
export interface AIChatFeatureConfig {
  /** 是否启用聊天功能 */
  enabled: boolean
  /** 使用的模型 */
  model: string
  /** 是否启用解释功能 */
  explanation: boolean
}

/**
 * 模型配置
 */
export interface ModelConfig {
  /** 模型名称 */
  name: string
  /** 提供商 */
  provider: string
  /** API 端点 */
  endpoint: string
  /** API 密钥 */
  apiKey?: string
  /** 模型参数 */
  parameters: ModelParameters
  /** 启用状态 */
  enabled: boolean
}

/**
 * 模型参数
 */
export interface ModelParameters {
  /** 温度 */
  temperature: number
  /** Top-p */
  topP: number
  /** 最大令牌数 */
  maxTokens: number
  /** 频率惩罚 */
  frequencyPenalty: number
  /** 存在惩罚 */
  presencePenalty: number
}

/**
 * API 配置
 */
export interface APIConfig {
  /** 请求超时（秒） */
  timeout: number
  /** 重试次数 */
  retryCount: number
  /** 重试间隔（毫秒） */
  retryInterval: number
  /** 启用缓存 */
  enableCache: boolean
  /** 缓存过期时间（秒） */
  cacheTtl: number
}

/**
 * 补全配置
 */
export interface CompletionConfig {
  /** 启用自动补全 */
  autoCompletion: boolean
  /** 补全延迟（毫秒） */
  completionDelay: number
  /** 最大建议数 */
  maxSuggestions: number
  /** 启用上下文感知 */
  contextAware: boolean
  /** 上下文窗口大小 */
  contextWindowSize: number
}

/**
 * 聊天配置
 */
export interface ChatConfig {
  /** 启用聊天历史 */
  enableHistory: boolean
  /** 历史记录数量 */
  historySize: number
  /** 启用流式响应 */
  streaming: boolean
  /** 系统提示 */
  systemPrompt: string
  /** 启用语法高亮 */
  syntaxHighlighting: boolean
}

// ============================================================================
// 主题配置
// ============================================================================

// 主题相关类型已迁移到 @/types/theme.ts

/**
 * 主题定义
 */
export interface Theme {
  /** 主题名称 */
  name: string
  /** 主题类型 */
  themeType: ThemeType
  /** 颜色配置 */
  colors: ColorScheme
  /** 语法高亮 */
  syntax: SyntaxHighlight
  /** UI 颜色 */
  ui: UIColors
}

/**
 * 主题类型
 */
export type ThemeType = 'light' | 'dark' | 'auto'

/**
 * 颜色方案
 */
export interface ColorScheme {
  /** 前景色 */
  foreground: string
  /** 背景色 */
  background: string
  /** 光标颜色 */
  cursor: string
  /** 选择颜色 */
  selection: string
  /** ANSI 颜色 */
  ansi: AnsiColors
  /** 明亮 ANSI 颜色 */
  bright: AnsiColors
}

/**
 * ANSI 颜色
 */
export interface AnsiColors {
  black: string
  red: string
  green: string
  yellow: string
  blue: string
  magenta: string
  cyan: string
  white: string
}

/**
 * 语法高亮
 */
export interface SyntaxHighlight {
  /** 关键字 */
  keyword: string
  /** 字符串 */
  string: string
  /** 注释 */
  comment: string
  /** 数字 */
  number: string
  /** 函数 */
  function: string
  /** 变量 */
  variable: string
  /** 类型 */
  typeName: string
  /** 操作符 */
  operator: string
}

/**
 * UI 颜色
 */
export interface UIColors {
  /** 主色调 */
  primary: string
  /** 次要色调 */
  secondary: string
  /** 成功色 */
  success: string
  /** 警告色 */
  warning: string
  /** 错误色 */
  error: string
  /** 信息色 */
  info: string
  /** 边框色 */
  border: string
  /** 分隔线色 */
  divider: string
}

// ============================================================================
// 快捷键配置
// ============================================================================

/**
 * 快捷键配置 (对应后端 ShortcutsConfig)
 */
export interface ShortcutsConfig {
  /** 全局快捷键 */
  global: ShortcutBinding[]
  /** 终端快捷键 */
  terminal: ShortcutBinding[]
  /** 自定义快捷键 */
  custom: ShortcutBinding[]
}

/**
 * 快捷键绑定 (对应后端 ShortcutBinding)
 */
export interface ShortcutBinding {
  /** 按键 */
  key: string
  /** 修饰键 */
  modifiers: string[]
  /** 动作 */
  action: ShortcutAction
}

/**
 * 快捷键动作 (对应后端 ShortcutAction)
 */
export type ShortcutAction =
  | string
  | {
      type: string
      text?: string
    }

// ============================================================================
// 配置管理相关类型
// ============================================================================

/**
 * 配置文件信息
 */
export interface ConfigFileInfo {
  /** 文件路径 */
  path: string
  /** 文件是否存在 */
  exists: boolean
  /** 文件大小（字节） */
  size?: number
  /** 最后修改时间 */
  modifiedAt?: string
  /** 是否可读 */
  readable: boolean
  /** 是否可写 */
  writable: boolean
}

/**
 * 配置更改事件
 */
export interface ConfigChangeEvent {
  /** 更改类型 */
  changeType: ConfigChangeType
  /** 字段路径 */
  fieldPath: string
  /** 旧值 */
  oldValue?: any
  /** 新值 */
  newValue?: any
  /** 时间戳 */
  timestamp: string
}

/**
 * 配置更改类型
 */
export type ConfigChangeType = 'created' | 'updated' | 'deleted' | 'migrated'

// ============================================================================
// 状态管理相关类型
// ============================================================================

/**
 * 配置加载状态
 */
export interface ConfigLoadingState {
  /** 是否正在加载 */
  loading: boolean
  /** 错误信息 */
  error: string | null
  /** 最后更新时间 */
  lastUpdated: Date | null
}

/**
 * 配置文件状态
 */
export interface ConfigFileState {
  /** 文件信息 */
  info: ConfigFileInfo | null
  /** 是否正在加载 */
  loading: boolean
  /** 错误信息 */
  error: string | null
}

// ============================================================================
// API 相关类型
// ============================================================================

/**
 * 配置 API 错误
 */
export class ConfigApiError extends Error {
  constructor(
    message: string,
    public readonly cause?: unknown
  ) {
    super(message)
    this.name = 'ConfigApiError'
  }
}

/**
 * 配置更新选项
 */
export interface ConfigUpdateOptions {
  /** 是否自动保存 */
  autoSave?: boolean
  /** 是否验证配置 */
  validate?: boolean
  /** 是否创建备份 */
  createBackup?: boolean
}

/**
 * 配置验证结果
 */
export interface ConfigValidationResult {
  /** 验证是否通过 */
  valid: boolean
  /** 错误列表 */
  errors: ConfigValidationError[]
  /** 警告列表 */
  warnings: ConfigValidationWarning[]
}

/**
 * 配置验证错误
 */
export interface ConfigValidationError {
  /** 字段路径 */
  fieldPath: string
  /** 错误消息 */
  message: string
  /** 错误代码 */
  code: string
  /** 当前值 */
  currentValue?: any
  /** 期望值 */
  expectedValue?: any
}

/**
 * 配置验证警告
 */
export interface ConfigValidationWarning {
  /** 字段路径 */
  fieldPath: string
  /** 警告消息 */
  message: string
  /** 警告代码 */
  code: string
  /** 建议值 */
  suggestedValue?: any
}
