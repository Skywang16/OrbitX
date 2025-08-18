/**
 * 完全统一的工具数据结构
 *
 * 这是所有工具的标准数据格式，不再有任何特殊字段
 */

/**
 * 工具执行信息 - 完全统一的结构
 */
export interface ToolExecution {
  /** 工具名称 */
  name: string
  /** 所有参数 */
  params: Record<string, unknown>
  /** 执行状态 */
  status: 'running' | 'completed' | 'error'
  /** 开始时间 */
  startTime: number
  /** 结束时间 */
  endTime?: number
  /** 工具执行结果 */
  result?: unknown
  /** 错误信息 */
  error?: string
}

/**
 * 创建工具执行信息
 */
export function createToolExecution(
  name: string,
  params: Record<string, unknown>,
  status: 'running' | 'completed' | 'error' = 'running'
): ToolExecution {
  return {
    name,
    params,
    status,
    startTime: Date.now(),
  }
}

/**
 * 获取工具执行时长（毫秒）
 */
export function getExecutionDuration(toolExecution: ToolExecution): number | null {
  if (!toolExecution.endTime) return null
  return toolExecution.endTime - toolExecution.startTime
}

/**
 * 格式化执行时长为人类可读格式
 */
export function formatExecutionDuration(toolExecution: ToolExecution): string {
  const duration = getExecutionDuration(toolExecution)
  if (duration === null) return '执行中...'

  if (duration < 1000) return `${duration}ms`
  if (duration < 60000) return `${(duration / 1000).toFixed(1)}s`
  return `${(duration / 60000).toFixed(1)}min`
}
