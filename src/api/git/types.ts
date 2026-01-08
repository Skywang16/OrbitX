export type FileChangeStatus = 'added' | 'modified' | 'deleted' | 'renamed' | 'copied' | 'untracked' | 'conflicted'

export interface FileChange {
  path: string
  status: FileChangeStatus
  oldPath?: string | null
}

export interface CommitFileChange {
  path: string
  status: FileChangeStatus
  oldPath?: string | null
  additions: number
  deletions: number
}

export interface BranchInfo {
  name: string
  isCurrent: boolean
  isRemote: boolean
  upstream?: string | null
  ahead?: number | null
  behind?: number | null
}

export interface CommitInfo {
  hash: string
  shortHash: string
  authorName: string
  authorEmail: string
  date: string
  message: string
  refs: CommitRef[]
  parents: string[]
}

export interface CommitRef {
  name: string
  refType: CommitRefType
}

export type CommitRefType = 'localBranch' | 'remoteBranch' | 'tag' | 'head'

export interface RepositoryStatus {
  isRepository: boolean
  rootPath?: string | null
  currentBranch?: string | null
  stagedFiles: FileChange[]
  modifiedFiles: FileChange[]
  untrackedFiles: FileChange[]
  conflictedFiles: FileChange[]
  ahead?: number | null
  behind?: number | null
  isEmpty: boolean
  isDetached: boolean
}

export type DiffLineType = 'context' | 'added' | 'removed' | 'header'

export interface DiffLine {
  lineType: DiffLineType
  content: string
  oldLineNumber?: number | null
  newLineNumber?: number | null
}

export interface DiffHunk {
  header: string
  lines: DiffLine[]
}

export interface DiffContent {
  filePath: string
  hunks: DiffHunk[]
}
