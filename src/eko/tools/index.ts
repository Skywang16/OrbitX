/**
 * 终端工具统一导出
 *
 * 🎯 工具分类说明：
 *
 * 📁 文件操作工具 (file-tools.ts)：
 * - read_file_enhanced: 📖 读取文件内容（支持行号、范围、文件信息）
 * - save_file: 💾 创建新文件（专业创建，支持编码、权限、目录创建）
 * - write_file: 📝 快速写入/追加内容（简单文本写入，支持追加模式）
 *
 * 📂 目录操作工具 (directory-tools.ts)：
 * - list_directory: 📂 列出目录内容
 * - create_directory: 📁 创建目录
 * - change_directory: 🚶 切换工作目录
 * - get_current_directory: 📍 获取当前目录
 *
 * 🔍 搜索工具 (search-tools.ts)：
 * - search_code: 🔍 搜索代码/文本（支持正则、扩展名过滤）
 *
 * ⚡ 命令执行工具 (command-tools.ts)：
 * - execute_command: 🔧 万能命令执行（其他工具无法满足时使用）
 * - precise_edit: ✏️ 精确编辑现有文件（类似IDE的查找替换）
 *
 * 📊 状态查询工具 (status-tools.ts)：
 * - get_terminal_status: 📊 获取终端状态信息
 * - remove_files: 🗑️ 安全删除文件/目录（支持备份、预览、安全检查）
 */

import type { Tool } from '../types'

// 文件操作工具
import { enhancedReadFileTool, saveFileTool, writeFileTool } from './file-tools'

// 目录操作工具
import { listDirectoryTool, createDirectoryTool, changeDirectoryTool, getCurrentDirectoryTool } from './directory-tools'

// 搜索工具
import { codeSearchTool } from './search-tools'

// 命令执行工具
import { executeCommandTool, preciseEditTool } from './command-tools'

// 状态查询工具
import { getTerminalStatusTool, removeFilesTool } from './status-tools'

/**
 * 所有终端工具的集合
 * 按使用频率和重要性排序
 */
export const terminalTools: Tool[] = [
  // 🔧 核心命令执行
  executeCommandTool,

  // 📁 文件操作（按使用频率排序）
  enhancedReadFileTool, // 读取文件 - 最常用
  saveFileTool, // 创建文件 - 专业创建
  writeFileTool, // 写入/追加 - 快速写入
  preciseEditTool, // 编辑文件 - 精确修改
  removeFilesTool, // 删除文件 - 安全删除

  // 📂 目录操作
  listDirectoryTool, // 列出目录
  createDirectoryTool, // 创建目录
  changeDirectoryTool, // 切换目录
  getCurrentDirectoryTool, // 获取当前目录

  // 🔍 搜索和状态
  codeSearchTool, // 搜索代码
  getTerminalStatusTool, // 终端状态
]

// 导出所有工具
export {
  // 文件操作
  enhancedReadFileTool,
  saveFileTool,
  writeFileTool,

  // 目录操作
  listDirectoryTool,
  createDirectoryTool,
  changeDirectoryTool,
  getCurrentDirectoryTool,

  // 搜索
  codeSearchTool,

  // 命令执行
  executeCommandTool,
  preciseEditTool,

  // 状态查询
  getTerminalStatusTool,
  removeFilesTool,
}

// 导出类型和工具函数
export * from './types'
export * from './utils'
