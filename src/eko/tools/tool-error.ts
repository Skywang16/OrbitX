/**
 * 工具错误处理模块
 */

export class ToolError extends Error {
  public readonly code: string
  public readonly details?: unknown

  constructor(message: string, code = 'TOOL_ERROR', details?: unknown) {
    super(message)
    this.name = 'ToolError'
    this.code = code
    this.details = details
  }
}

export class FileNotFoundError extends ToolError {
  constructor(path: string) {
    super(`文件不存在: ${path}`, 'FILE_NOT_FOUND', { path })
  }
}

export class FileAlreadyExistsError extends ToolError {
  constructor(path: string) {
    super(`文件已存在: ${path}`, 'FILE_ALREADY_EXISTS', { path })
  }
}

export class DirectoryNotFoundError extends ToolError {
  constructor(path: string) {
    super(`目录不存在: ${path}`, 'DIRECTORY_NOT_FOUND', { path })
  }
}

export class PermissionDeniedError extends ToolError {
  constructor(path: string) {
    super(`权限被拒绝: ${path}`, 'PERMISSION_DENIED', { path })
  }
}

export class NetworkError extends ToolError {
  constructor(message: string, details?: unknown) {
    super(`网络错误: ${message}`, 'NETWORK_ERROR', details)
  }
}

export class ValidationError extends ToolError {
  constructor(message: string, details?: unknown) {
    super(`验证失败: ${message}`, 'VALIDATION_ERROR', details)
  }
}

export class TerminalError extends ToolError {
  constructor(message: string, details?: unknown) {
    super(`终端错误: ${message}`, 'TERMINAL_ERROR', details)
  }
}

/**
 * 格式化工具错误信息
 */
export function formatToolError(error: unknown): string {
  if (error instanceof ToolError) {
    return `❌ ${error.message}`
  }

  if (error instanceof Error) {
    return `❌ ${error.message}`
  }

  return `❌ 未知错误: ${String(error)}`
}

/**
 * 包装工具执行结果
 */
export function wrapToolResult<T>(
  operation: () => Promise<T> | T,
  errorMessage?: string
): Promise<{ success: true; data: T } | { success: false; error: string }> {
  return Promise.resolve()
    .then(() => operation())
    .then(data => ({ success: true as const, data }))
    .catch(error => ({
      success: false as const,
      error: errorMessage || formatToolError(error),
    }))
}

