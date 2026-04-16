/**
 * Checkpoint system type definitions
 */

export type FileChangeType = 'added' | 'modified' | 'deleted'

export interface CheckpointSummary {
  id: number
  workspacePath: string
  threadId: number
  messageId: number
  parentId: number | null
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
  restoredFiles: string[]
  failedFiles: [string, string][]
}
