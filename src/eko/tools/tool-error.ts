/**
 * 工具错误类型定义
 */

export class ToolError extends Error {
  public readonly code: string

  constructor(message: string, code: string = 'TOOL_ERROR') {
    super(message)
    this.name = 'ToolError'
    this.code = code
  }
}

export class ValidationError extends ToolError {
  constructor(message: string) {
    super(message, 'VALIDATION_ERROR')
    this.name = 'ValidationError'
  }
}

export class FileNotFoundError extends ToolError {
  constructor(path: string) {
    super(`File or directory not found: ${path}`, 'FILE_NOT_FOUND')
    this.name = 'FileNotFoundError'
  }
}

export class PermissionError extends ToolError {
  constructor(message: string) {
    super(message, 'PERMISSION_ERROR')
    this.name = 'PermissionError'
  }
}

export class NetworkError extends ToolError {
  constructor(message: string) {
    super(message, 'NETWORK_ERROR')
    this.name = 'NetworkError'
  }
}

/**
 * 格式化工具错误信息
 */
export function formatToolError(error: unknown): string {
  if (error instanceof ToolError) {
    return `[${error.code}] ${error.message}`
  }

  if (error instanceof Error) {
    return error.message
  }

  return String(error)
}
