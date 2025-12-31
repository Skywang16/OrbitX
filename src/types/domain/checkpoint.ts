/**
 * Checkpoint 系统类型定义
 */

export type FileChangeType = 'added' | 'modified' | 'deleted'

export interface CheckpointSummary {
  id: number
  workspacePath: string
  sessionId: number
  parentId: number | null
  userMessage: string
  createdAt: string
  fileCount: number
  totalSize: number
}

export interface FileDiff {
  filePath: string
  changeType: FileChangeType
  diffContent: string | null
}

export interface RollbackResult {
  checkpointId: number
  newCheckpointId: number
  restoredFiles: string[]
  failedFiles: [string, string][]
}
