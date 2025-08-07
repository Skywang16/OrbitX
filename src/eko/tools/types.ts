/**
 * 终端工具相关类型定义
 */

/**
 * 执行命令参数
 */
export interface ExecuteCommandParams {
  command: string
  terminalId?: number
}

/**
 * 写入文件参数
 */
export interface WriteFileParams {
  path: string
  content: string
  append?: boolean
}

/**
 * 列出目录参数
 */
export interface ListDirectoryParams {
  path?: string
  showHidden?: boolean
  detailed?: boolean
}

/**
 * 获取终端状态参数
 */
export interface GetTerminalStatusParams {
  terminalId?: number
  detailed?: boolean
}

/**
 * 精确编辑参数
 */
export interface PreciseEditParams {
  file_path: string
  old_string: string
  new_string: string
  expected_replacements?: number
  create_backup?: boolean
}

/**
 * 保存文件参数
 */
export interface SaveFileParams {
  file_path: string
  content: string
  encoding?: string
  create_directories?: boolean
  overwrite?: boolean
  file_permissions?: string
  add_newline?: boolean
}

/**
 * 删除文件参数
 */
export interface RemoveFilesParams {
  paths: string[]
  recursive?: boolean
  force?: boolean
  create_backup?: boolean
  dry_run?: boolean
}

/**
 * 代码搜索参数
 */
export interface CodeSearchParams {
  pattern: string
  file_path?: string
  directory?: string
  case_sensitive?: boolean
  regex?: boolean
  show_line_numbers?: boolean
  context_lines?: number
  file_extensions?: string
}

/**
 * 增强文件读取参数
 */
export interface EnhancedReadFileParams {
  file_path: string
  show_line_numbers?: boolean
  start_line?: number
  end_line?: number
  show_file_info?: boolean
}

/**
 * 创建目录参数
 */
export interface CreateDirectoryParams {
  path: string
  recursive?: boolean
}

/**
 * 切换目录参数
 */
export interface ChangeDirectoryParams {
  path: string
}
