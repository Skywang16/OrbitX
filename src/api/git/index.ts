import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@/utils/request'
import type { BranchInfo, CommitFileChange, CommitInfo, DiffContent, RepositoryStatus } from './types'

export interface GetDiffOptions {
  path: string
  filePath: string
  staged?: boolean
  commitHash?: string
}

export interface GitChangedPayload {
  path: string
  changeType: 'index' | 'head' | 'refs' | 'worktree'
}

export class GitApi {
  checkRepository = async (path: string): Promise<string | null> => {
    return invoke<string | null>('git_check_repository', { path })
  }

  getStatus = async (path: string): Promise<RepositoryStatus> => {
    return invoke<RepositoryStatus>('git_get_status', { path })
  }

  getBranches = async (path: string): Promise<BranchInfo[]> => {
    return invoke<BranchInfo[]>('git_get_branches', { path })
  }

  getCommits = async (path: string, limit?: number, skip?: number): Promise<CommitInfo[]> => {
    return invoke<CommitInfo[]>('git_get_commits', { path, limit, skip })
  }

  getCommitFiles = async (path: string, commitHash: string): Promise<CommitFileChange[]> => {
    return invoke<CommitFileChange[]>('git_get_commit_files', { path, commitHash })
  }

  getDiff = async (options: GetDiffOptions): Promise<DiffContent> => {
    return invoke<DiffContent>('git_get_diff', {
      path: options.path,
      filePath: options.filePath,
      staged: options.staged,
      commitHash: options.commitHash,
    })
  }

  watchStart = async (path: string): Promise<void> => {
    return invoke<void>('git_watch_start', { path })
  }

  watchStop = async (): Promise<void> => {
    return invoke<void>('git_watch_stop')
  }

  watchStatus = async (): Promise<string | null> => {
    return invoke<string | null>('git_watch_status')
  }

  onChanged = async (callback: (payload: GitChangedPayload) => void): Promise<UnlistenFn> => {
    return await listen<GitChangedPayload>('git:changed', event => {
      callback(event.payload)
    })
  }
}

export const gitApi = new GitApi()
export default gitApi
