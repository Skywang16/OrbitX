/**
 * Eko集成相关的TypeScript类型定义
 */

import type {
  StreamCallbackMessage,
  StreamCallback as EkoStreamCallback,
  HumanCallback as EkoHumanCallback,
  Agent,
  AgentContext,
} from '@eko-ai/eko'

// 定义工具相关类型（如果eko没有导出）
export interface Tool<T = unknown> {
  name: string
  description: string
  parameters: {
    type: 'object'
    properties: Record<string, unknown>
    required?: string[]
  }
  execute: (params: T, context: AgentContext) => Promise<ToolResult>
}

export interface ToolResult {
  content: Array<{
    type: 'text' | 'image' | 'file'
    text?: string
    data?: unknown
  }>
}

// ===== 基础类型 =====

/**
 * 终端上下文信息
 */
export interface TerminalContext {
  /** 当前工作目录 */
  workingDirectory: string
  /** 终端ID */
  terminalId: number
  /** 环境变量 */
  environment: Record<string, string>
  /** 命令历史 */
  commandHistory: string[]
  /** 当前shell */
  shell: string
  /** 系统信息 */
  systemInfo: {
    os: string
    arch: string
    platform: string
  }
}

/**
 * 命令执行结果
 */
export interface CommandResult {
  /** 命令 */
  command: string
  /** 退出码 */
  exitCode: number
  /** 标准输出 */
  stdout: string
  /** 标准错误 */
  stderr: string
  /** 执行时间(毫秒) */
  duration: number
  /** 是否成功 */
  success: boolean
}

/**
 * 文件操作结果
 */
export interface FileOperationResult {
  /** 操作类型 */
  operation: 'read' | 'write' | 'create' | 'delete' | 'list'
  /** 文件路径 */
  path: string
  /** 是否成功 */
  success: boolean
  /** 结果数据 */
  data?: unknown
  /** 错误信息 */
  error?: string
}

// ===== 回调相关类型 =====

/**
 * 终端专用流式回调
 */
export interface TerminalStreamCallback {
  onMessage: (message: StreamCallbackMessage) => Promise<void>
}

/**
 * 终端专用人机交互回调
 */
export interface TerminalHumanCallback extends EkoHumanCallback {
  /**
   * @override
   * HumanCallback中的onHumanHelp返回Promise<boolean>，这里我们保持一致
   * @param context
   * @param helpType
   * @param prompt
   */
  onHumanHelp: (context: AgentContext, helpType: string, prompt: string) => Promise<boolean>
  /** 请求用户确认命令执行 */
  onCommandConfirm?: (context: AgentContext, command: string) => Promise<boolean>
  /** 请求用户选择文件 */
  onFileSelect?: (context: AgentContext, prompt: string, directory?: string) => Promise<string>
  /** 请求用户输入路径 */
  onPathInput?: (context: AgentContext, prompt: string, defaultPath?: string) => Promise<string>
}

/**
 * 组合的回调接口, 继承自Eko的核心回调接口，以确保类型兼容
 */
export interface TerminalCallback extends EkoStreamCallback, EkoHumanCallback {
  // 重新声明不兼容的 onHumanHelp
  onHumanHelp: (context: AgentContext, helpType: string, prompt: string) => Promise<boolean>

  // 终端专用回调
  onCommandConfirm?: (context: AgentContext, command: string) => Promise<boolean>
  onFileSelect?: (context: AgentContext, prompt: string, directory?: string) => Promise<string>
  onPathInput?: (context: AgentContext, prompt: string, defaultPath?: string) => Promise<string>
}

// ===== 工具相关类型 =====

/**
 * 终端工具参数基础接口
 */
export interface TerminalToolParams {
  /** 终端ID */
  terminalId?: number
}

/**
 * 执行命令工具参数
 */
export interface ExecuteCommandParams extends TerminalToolParams {
  /** 要执行的命令 */
  command: string
  /** 工作目录 */
  workingDirectory?: string
  /** 环境变量 */
  environment?: Record<string, string>
  /** 超时时间(毫秒) */
  timeout?: number
}

/**
 * 文件读取工具参数
 */
export interface ReadFileParams extends TerminalToolParams {
  /** 文件路径 */
  path: string
  /** 编码格式 */
  encoding?: string
}

/**
 * 文件写入工具参数
 */
export interface WriteFileParams extends TerminalToolParams {
  /** 文件路径 */
  path: string
  /** 文件内容 */
  content: string
  /** 编码格式 */
  encoding?: string
  /** 是否追加 */
  append?: boolean
}

/**
 * 目录列表工具参数
 */
export interface ListDirectoryParams extends TerminalToolParams {
  /** 目录路径 */
  path?: string
  /** 是否显示隐藏文件 */
  showHidden?: boolean
  /** 是否递归 */
  recursive?: boolean
  /** 是否显示详细信息 */
  detailed?: boolean
}

/**
 * 获取终端状态工具参数
 */
export interface GetTerminalStatusParams extends TerminalToolParams {
  /** 是否包含详细信息 */
  detailed?: boolean
}

// ===== Agent相关类型 =====

/**
 * 终端Agent配置
 */
export interface TerminalAgentConfig {
  /** Agent名称 */
  name: string
  /** Agent描述 */
  description: string
  /** 默认终端ID */
  defaultTerminalId?: number
  /** 默认工作目录 */
  defaultWorkingDirectory?: string
  /** 是否启用安全模式 */
  safeMode?: boolean
  /** 允许的命令白名单 */
  allowedCommands?: string[]
  /** 禁止的命令黑名单 */
  blockedCommands?: string[]
}

/**
 * 代码Agent配置
 */
export interface CodeAgentConfig {
  /** Agent名称 */
  name: string
  /** Agent描述 */
  description: string
  /** 默认工作目录 */
  defaultWorkingDirectory?: string
  /** 是否启用安全模式 */
  safeMode?: boolean
  /** 支持的编程语言 */
  supportedLanguages: string[]
  /** 代码风格配置 */
  codeStyle: {
    /** 缩进大小 */
    indentSize: number
    /** 缩进类型 */
    indentType: 'spaces' | 'tabs'
    /** 最大行长度 */
    maxLineLength: number
    /** 插入最终换行符 */
    insertFinalNewline: boolean
    /** 删除尾随空白 */
    trimTrailingWhitespace: boolean
  }
  /** 启用的功能 */
  enabledFeatures: {
    /** 代码生成 */
    codeGeneration: boolean
    /** 代码分析 */
    codeAnalysis: boolean
    /** 重构 */
    refactoring: boolean
    /** 格式化 */
    formatting: boolean
    /** 语法检查 */
    linting: boolean
    /** 测试 */
    testing: boolean
    /** 文档生成 */
    documentation: boolean
  }
}

/**
 * 工具执行上下文
 */
export interface ToolExecutionContext extends AgentContext {
  /** 终端上下文 */
  terminalContext: TerminalContext
  /** 安全检查 */
  safetyCheck?: (command: string) => boolean
}

// ===== Eko实例相关类型 =====

/**
 * Eko实例配置
 */
export interface EkoInstanceConfig {
  /** 调试模式 */
  debug?: boolean
  /** 默认终端ID */
  defaultTerminalId?: number
  /** 回调配置 */
  callback?: TerminalCallback
  /** 终端Agent配置 */
  agentConfig?: TerminalAgentConfig
  /** 代码Agent配置 */
  codeAgentConfig?: CodeAgentConfig
}

/**
 * Eko运行选项
 */
export interface EkoRunOptions {
  /** 终端ID */
  terminalId?: number
  /** 工作目录 */
  workingDirectory?: string
  /** 超时时间 */
  timeout?: number
  /** 是否启用流式输出 */
  streaming?: boolean
}

/**
 * Eko运行结果
 */
export interface EkoRunResult {
  /** 结果内容 */
  result: string
  /** 执行的命令列表 */
  executedCommands?: CommandResult[]
  /** 操作的文件列表 */
  fileOperations?: FileOperationResult[]
  /** 执行时间 */
  duration: number
  /** 是否成功 */
  success: boolean
  /** 错误信息 */
  error?: string
}

// ===== AST分析相关类型 =====
// 重新导出AI模块中的类型定义，避免重复
export type { CodeSymbol, CodeAnalysis, BatchCodeAnalysis as AnalysisResult } from '@/api/ai/types'

/**
 * AST分析工具参数
 */
export interface AnalyzeCodeParams extends TerminalToolParams {
  /** 文件路径或目录路径 */
  path: string
  /** 是否递归分析目录 */
  recursive?: boolean
  /** 包含的文件模式 */
  include?: string[]
  /** 排除的文件模式 */
  exclude?: string[]
}

// ===== 导出所有类型 =====

export type {
  // Eko原生类型
  StreamCallbackMessage,
  EkoStreamCallback,
  EkoHumanCallback,
  Agent,
  AgentContext,
}
