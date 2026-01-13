import { invoke } from '@/utils/request'
import type { BranchInfo, CommitFileChange, CommitInfo, DiffContent, RepositoryStatus } from './types'

export interface GetDiffOptions {
  path: string
  filePath: string
  staged?: boolean
  commitHash?: string
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

  stagePaths = async (path: string, paths: string[]): Promise<void> => {
    await invoke<void>('git_stage_paths', { path, paths })
  }

  stageAll = async (path: string): Promise<void> => {
    await invoke<void>('git_stage_all', { path })
  }

  unstagePaths = async (path: string, paths: string[]): Promise<void> => {
    await invoke<void>('git_unstage_paths', { path, paths })
  }

  unstageAll = async (path: string): Promise<void> => {
    await invoke<void>('git_unstage_all', { path })
  }

  discardWorktreePaths = async (path: string, paths: string[]): Promise<void> => {
    await invoke<void>('git_discard_worktree_paths', { path, paths })
  }

  discardWorktreeAll = async (path: string): Promise<void> => {
    await invoke<void>('git_discard_worktree_all', { path })
  }

  cleanPaths = async (path: string, paths: string[]): Promise<void> => {
    await invoke<void>('git_clean_paths', { path, paths })
  }

  cleanAll = async (path: string): Promise<void> => {
    await invoke<void>('git_clean_all', { path })
  }

  commit = async (path: string, message: string): Promise<void> => {
    await invoke<void>('git_commit', { path, message })
  }

  push = async (path: string): Promise<void> => {
    await invoke<void>('git_push', { path })
  }

  pull = async (path: string): Promise<void> => {
    await invoke<void>('git_pull', { path })
  }

  fetch = async (path: string): Promise<void> => {
    await invoke<void>('git_fetch', { path })
  }

  checkoutBranch = async (path: string, branch: string): Promise<void> => {
    await invoke<void>('git_checkout_branch', { path, branch })
  }

  initRepo = async (path: string): Promise<void> => {
    await invoke<void>('git_init_repo', { path })
  }
}

export const gitApi = new GitApi()
export default gitApi
