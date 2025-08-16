/**
 * AI工具统一API接口
 *
 * 整合所有工具相关的API调用，包括：
 * - AST代码分析
 * - 文件系统操作
 * - 网络请求
 * - Shell命令执行
 * - 存储管理
 */

import { invoke } from '@tauri-apps/api/core'
import type { SessionState } from '../storage/types'

// ===== AST代码分析相关 =====

/**
 * 代码符号信息
 */
export interface CodeSymbol {
  /** 符号名称 */
  name: string
  /** 符号类型 */
  type: string
  /** 所在行号 */
  line: number
  /** 所在列号 */
  column: number
  /** 符号范围 */
  range?: {
    start: { line: number; column: number }
    end: { line: number; column: number }
  }
}

/**
 * 代码分析结果
 */
export interface CodeAnalysis {
  /** 文件路径 */
  file: string
  /** 编程语言 */
  language: string
  /** 符号列表 */
  symbols: CodeSymbol[]
  /** 导入语句 */
  imports: string[]
  /** 导出语句 */
  exports: string[]
  /** 代码复杂度 */
  complexity?: number
}

/**
 * 分析结果汇总
 */
export interface AnalysisResult {
  /** 分析结果列表 */
  analyses: CodeAnalysis[]
  /** 总文件数 */
  total_files: number
  /** 成功分析数 */
  success_count: number
  /** 失败分析数 */
  error_count: number
}

/**
 * AST分析工具参数
 */
export interface AnalyzeCodeParams {
  /** 文件路径或目录路径 */
  path: string
  /** 是否递归分析目录 */
  recursive?: boolean
  /** 包含的文件模式 */
  include?: string[]
  /** 排除的文件模式 */
  exclude?: string[]
}

/**
 * 分析代码结构
 */
export async function analyzeCode(params: AnalyzeCodeParams): Promise<AnalysisResult> {
  return await invoke('analyze_code', { params })
}

// ===== 网络请求相关 =====

/**
 * 网络请求参数
 */
export interface WebFetchRequest {
  url: string
  method?: string
  headers?: Record<string, string>
  body?: string
  timeout?: number
  follow_redirects?: boolean
  response_format?: string
  extract_content?: boolean
  max_content_length?: number
  use_jina_reader?: boolean
}

/**
 * 网络请求响应
 */
export interface WebFetchResponse {
  status: number
  status_text: string
  headers: Record<string, string>
  data: string
  response_time: number
  final_url: string
  success: boolean
  error?: string
  content_type?: string
  content_length?: number
  extracted_text?: string
  page_title?: string
}

/**
 * 执行网络请求（完整功能）
 */
export async function webFetchHeadless(request: WebFetchRequest): Promise<WebFetchResponse> {
  return await invoke('web_fetch_headless', { request })
}

/**
 * 执行简单网络请求
 */
export async function simpleWebFetch(url: string): Promise<WebFetchResponse> {
  return await invoke('simple_web_fetch', { url })
}

// ===== Shell命令相关 =====

/**
 * 终端创建参数
 */
export interface CreateTerminalParams extends Record<string, unknown> {
  shell?: string
  working_directory?: string
  environment?: Record<string, string>
}

/**
 * 终端信息
 */
export interface TerminalInfo {
  id: number
  shell: string
  working_directory: string
  created_at: string
  is_active: boolean
}

/**
 * 创建终端
 */
export async function createTerminal(params?: CreateTerminalParams): Promise<number> {
  return await invoke('create_terminal', params || {})
}

/**
 * 向终端写入内容
 */
export async function writeToTerminal(terminalId: number, input: string): Promise<void> {
  return await invoke('write_to_terminal', { terminalId, input })
}

/**
 * 调整终端大小
 */
export async function resizeTerminal(terminalId: number, rows: number, cols: number): Promise<void> {
  return await invoke('resize_terminal', { terminalId, rows, cols })
}

/**
 * 关闭终端
 */
export async function closeTerminal(terminalId: number): Promise<void> {
  return await invoke('close_terminal', { terminalId })
}

/**
 * 获取终端列表
 */
export async function listTerminals(): Promise<TerminalInfo[]> {
  return await invoke('list_terminals')
}

// ===== 存储管理相关 =====
// SessionState 类型从 @/api/storage 导入

/**
 * 获取存储配置
 */
export async function storageGetConfig(key: string): Promise<any> {
  return await invoke('storage_get_config', { key })
}

/**
 * 更新存储配置
 */
export async function storageUpdateConfig(key: string, value: any): Promise<void> {
  return await invoke('storage_update_config', { key, value })
}

/**
 * 保存会话状态
 */
export async function storageSaveSessionState(state: SessionState): Promise<void> {
  return await invoke('storage_save_session_state', { state })
}

/**
 * 加载会话状态
 */
export async function storageLoadSessionState(): Promise<SessionState | null> {
  return await invoke('storage_load_session_state')
}

// ===== 文件系统相关 =====

/**
 * 文件信息
 */
export interface FileInfo {
  name: string
  path: string
  size: number
  is_directory: boolean
  is_file: boolean
  modified: string
  created?: string
  permissions?: string
}

/**
 * 目录内容
 */
export interface DirectoryContent {
  files: FileInfo[]
  total_count: number
  directory_path: string
}
