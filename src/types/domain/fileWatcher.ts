export type GitChangeType = 'index' | 'head' | 'refs' | 'worktree'
export type FsEventType = 'created' | 'modified' | 'deleted' | 'renamed'

export type FileWatcherEvent =
  | {
      type: 'git_changed'
      repoRoot: string
      changeType: GitChangeType
      timestampMs: number
    }
  | {
      type: 'fs_changed'
      workspaceRoot: string
      path: string
      eventType: FsEventType
      oldPath: string | null
      timestampMs: number
    }

export interface FileWatcherEventBatch {
  seq: number
  events: FileWatcherEvent[]
}

export interface FileWatcherStatus {
  running: boolean
  workspaceRoot: string | null
  repoRoot: string | null
}
