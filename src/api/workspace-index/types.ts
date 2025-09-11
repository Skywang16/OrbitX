/**
 * 工作区向量索引API类型定义
 */

/**
 * 工作区索引状态枚举
 */
export enum IndexStatus {
  Building = 'building',
  Ready = 'ready',
  Error = 'error',
}

/**
 * 工作区索引信息接口
 */
export interface WorkspaceIndex {
  id: number
  workspacePath: string
  name?: string
  status: IndexStatus
  fileCount: number
  indexSizeBytes: number
  errorMessage?: string
  createdAt: string
  updatedAt: string
}

/**
 * 构建工作区索引的参数
 */
export interface BuildWorkspaceIndexParams {
  path: string
  name?: string
}

/**
 * 工作区索引API选项
 */
export interface WorkspaceIndexAPIOptions {
  timeout?: number
  retries?: number
  signal?: AbortSignal
}