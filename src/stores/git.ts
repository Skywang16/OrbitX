import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { debounce } from 'lodash-es'
import { gitApi, shellApi } from '@/api'
import { createMessage } from '@/ui'
import { useTerminalStore } from '@/stores/Terminal'
import { useEditorStore } from '@/stores/Editor'
import { getWorkspacePathFromContext } from '@/tabs/context'
import type { BranchInfo, CommitInfo, FileChange, RepositoryStatus } from '@/api/git/types'

export const useGitStore = defineStore('git', () => {
  const terminalStore = useTerminalStore()
  const editorStore = useEditorStore()

  const status = ref<RepositoryStatus | null>(null)
  const branches = ref<BranchInfo[]>([])
  const commits = ref<CommitInfo[]>([])
  const commitsHasMore = ref(true)
  let commitsInFlight: Promise<void> | null = null
  const selectedFile = ref<FileChange | null>(null)
  const selectedFileIsStaged = ref(false)

  const isLoading = ref(false)
  const error = ref<string | null>(null)

  const isVisible = ref(false)
  const panelWidth = ref(320)

  const isRepository = computed(() => status.value?.isRepository ?? false)
  const repositoryRoot = computed(() => status.value?.rootPath ?? null)
  const currentBranch = computed(() => status.value?.currentBranch ?? null)
  const stagedCount = computed(() => status.value?.stagedFiles.length ?? 0)
  const changedCount = computed(() => {
    const staged = status.value?.stagedFiles.length ?? 0
    const modified = status.value?.modifiedFiles.length ?? 0
    const untracked = status.value?.untrackedFiles.length ?? 0
    const conflicted = status.value?.conflictedFiles.length ?? 0
    return staged + modified + untracked + conflicted
  })

  const currentPath = computed(() => {
    const active = editorStore.activeTab
    if (!active) return null
    return getWorkspacePathFromContext(active.context, { terminals: terminalStore.terminals })
  })

  let watchingRoot: string | null = null
  let watchStartInFlight: Promise<void> | null = null
  const ensureGitWatch = async (root: string | null) => {
    if (!root) {
      if (watchingRoot) {
        try {
          await gitApi.watchStop()
        } catch (e) {
          console.warn('Failed to stop git watcher:', e)
        } finally {
          watchingRoot = null
        }
      }
      return
    }

    if (watchingRoot === root) return

    if (watchStartInFlight) {
      await watchStartInFlight
      if (watchingRoot === root) return
    }

    watchStartInFlight = (async () => {
      try {
        await gitApi.watchStart(root)
        watchingRoot = root
      } catch (e) {
        console.warn('Failed to start git watcher:', e)
      } finally {
        watchStartInFlight = null
      }
    })()

    await watchStartInFlight
  }

  let pendingRefresh = false
  const refreshStatus = async () => {
    const path = currentPath.value
    if (!path || path === '~') {
      status.value = null
      branches.value = []
      commits.value = []
      selectedFile.value = null
      await ensureGitWatch(null)
      return
    }

    if (isLoading.value) {
      pendingRefresh = true
      return
    }

    isLoading.value = true
    error.value = null

    try {
      status.value = await gitApi.getStatus(path)

      if (!status.value.isRepository) {
        branches.value = []
        commits.value = []
        selectedFile.value = null
        await ensureGitWatch(null)
      } else {
        await ensureGitWatch(status.value.rootPath ?? null)
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      status.value = null
      await ensureGitWatch(null)
    } finally {
      isLoading.value = false
      if (pendingRefresh) {
        pendingRefresh = false
        refreshStatus()
      }
    }
  }

  const loadBranches = async () => {
    const path = currentPath.value
    if (!path || path === '~' || !isRepository.value) return

    try {
      branches.value = await gitApi.getBranches(path)
    } catch (e) {
      console.error('Failed to load branches:', e)
    }
  }

  const loadCommits = async (limit = 50) => {
    const path = currentPath.value
    if (!path || path === '~' || !isRepository.value) return

    while (commitsInFlight) {
      try {
        await commitsInFlight
      } catch {
        break
      }
    }

    const requestLimit = Math.min(limit + 1, 200)

    const task = (async () => {
      const fetched = await gitApi.getCommits(path, requestLimit, 0)
      commitsHasMore.value = fetched.length > limit
      commits.value = fetched.slice(0, limit)
      return commits.value.length
    })()
    commitsInFlight = task.then(
      () => {},
      () => {}
    )

    try {
      return await task
    } catch (e) {
      console.error('Failed to load commits:', e)
      return 0
    } finally {
      commitsInFlight = null
    }
  }

  const loadMoreCommits = async (limit = 50): Promise<{ loaded: number; hasMore: boolean }> => {
    const path = currentPath.value
    if (!path || path === '~' || !isRepository.value) return { loaded: 0, hasMore: false }

    while (commitsInFlight) {
      try {
        await commitsInFlight
      } catch {
        break
      }
    }

    const skip = commits.value.length
    const requestLimit = Math.min(limit + 1, 200)
    const hasOverfetch = requestLimit > limit

    const task = (async () => {
      const fetched = await gitApi.getCommits(path, requestLimit, skip)
      const fetchedHasMore = hasOverfetch ? fetched.length > limit : fetched.length === limit
      const page = fetched.slice(0, limit)

      if (page.length === 0) {
        commitsHasMore.value = false
        return { loaded: 0, hasMore: false }
      }

      const existing = new Set(commits.value.map(c => c.hash))
      const unique = page.filter(c => !existing.has(c.hash))
      if (unique.length === 0) {
        commitsHasMore.value = false
        return { loaded: 0, hasMore: false }
      }

      commits.value = [...commits.value, ...unique]
      commitsHasMore.value = fetchedHasMore
      return { loaded: unique.length, hasMore: commitsHasMore.value }
    })()
    commitsInFlight = task.then(
      () => {},
      () => {}
    )

    try {
      return await task
    } catch (e) {
      console.error('Failed to load commits:', e)
      return { loaded: 0, hasMore: false }
    } finally {
      commitsInFlight = null
    }
  }

  const togglePanel = () => {
    isVisible.value = !isVisible.value
    if (isVisible.value) {
      refreshStatus()
    }
  }

  const setPanelWidth = (width: number) => {
    panelWidth.value = Math.max(200, Math.min(600, width))
  }

  const requireRepoPath = (): string | null => {
    const path = currentPath.value
    if (!path || path === '~') return null
    return path
  }

  let resolvedGitRoot: string | null = null
  const resolveGitRoot = async (): Promise<string | null> => {
    if (repositoryRoot.value) {
      resolvedGitRoot = null
      return repositoryRoot.value
    }

    if (resolvedGitRoot) {
      return resolvedGitRoot
    }

    const path = requireRepoPath()
    if (!path) return null

    try {
      const detectedRoot = await gitApi.checkRepository(path)
      if (detectedRoot) {
        resolvedGitRoot = detectedRoot
        return detectedRoot
      }
    } catch (error) {
      console.warn('Failed to resolve git root:', error)
    }

    return null
  }

  const ensureGitRoot = async (): Promise<string | null> => {
    const cwd = await resolveGitRoot()
    if (!cwd) {
      createMessage.error('当前路径不是 Git 仓库')
    }
    return cwd
  }

  const handleGitError = (args: string[], error: unknown) => {
    console.error('Git command failed:', args.join(' '), error)
    const message = error instanceof Error ? error.message : String(error)
    createMessage.error(message || 'Git 命令执行失败')
  }

  const runGit = async (args: string[], cwdOverride?: string): Promise<boolean> => {
    const cwd = cwdOverride ?? (await ensureGitRoot())
    if (!cwd) return false

    try {
      await shellApi.executeBackgroundProgram('git', args, cwd)
      return true
    } catch (error) {
      handleGitError(args, error)
      return false
    }
  }

  const toRootPathSpec = (filePath: string) => {
    if (!filePath) return filePath
    const normalized = filePath.replace(/^\/+/, '')
    return normalized.startsWith(':/') ? normalized : `:/${normalized}`
  }

  const toWorktreePathSpec = (filePath: string) => {
    if (!filePath) return filePath
    const withoutRootMagic = filePath.startsWith(':/') ? filePath.slice(2) : filePath
    return withoutRootMagic.replace(/^\/+/, '')
  }

  const stageFile = async (file: FileChange, repoPathOverride?: string) => {
    const pathSpec = toRootPathSpec(file.path)
    if (await runGit(['add', '--', pathSpec], repoPathOverride)) {
      await refreshStatus()
    }
  }

  const stageFiles = async (files: FileChange[]) => {
    if (files.length === 0) return
    const pathSpecs = files.map(f => toRootPathSpec(f.path))
    if (await runGit(['add', '--', ...pathSpecs])) {
      await refreshStatus()
    }
  }

  const stageAllFiles = async () => {
    if (await runGit(['add', '-A'])) {
      await refreshStatus()
    }
  }

  const unstageFile = async (file: FileChange, repoPathOverride?: string) => {
    // git reset -- <path> 取消暂存单个文件
    const pathSpec = toRootPathSpec(file.path)
    if (await runGit(['reset', '--', pathSpec], repoPathOverride)) {
      await refreshStatus()
    }
  }

  const unstageFiles = async (files: FileChange[]) => {
    if (files.length === 0) return
    const pathSpecs = files.map(f => toRootPathSpec(f.path))
    if (await runGit(['reset', '--', ...pathSpecs])) {
      await refreshStatus()
    }
  }

  const unstageAllFiles = async () => {
    // git reset 取消所有暂存
    if (await runGit(['reset'])) {
      await refreshStatus()
    }
  }

  const discardFile = async (file: FileChange) => {
    if (file.status === 'untracked') {
      // 未跟踪文件直接删除
      const pathSpec = toRootPathSpec(file.path)
      if (!(await runGit(['clean', '-f', '--', pathSpec]))) {
        return
      }
    } else {
      // 已跟踪文件：仅丢弃 worktree 变更，保留 index（避免吞掉已暂存内容）
      const pathSpec = toWorktreePathSpec(file.path)
      if (!(await runGit(['checkout', '--', pathSpec]))) {
        return
      }
    }
    await refreshStatus()
  }

  const discardFiles = async (files: FileChange[]) => {
    if (files.length === 0) return

    const cwd = await ensureGitRoot()
    if (!cwd) return

    // 分开处理 untracked 和 tracked 文件
    const untracked = files.filter(f => f.status === 'untracked')
    const tracked = files.filter(f => f.status !== 'untracked')

    // 恢复 tracked 文件
    if (tracked.length > 0) {
      const pathSpecs = tracked.map(f => toWorktreePathSpec(f.path))
      if (!(await runGit(['checkout', '--', ...pathSpecs], cwd))) {
        return
      }
    }

    // 删除 untracked 文件
    if (untracked.length > 0) {
      const pathSpecs = untracked.map(f => toRootPathSpec(f.path))
      if (!(await runGit(['clean', '-f', '--', ...pathSpecs], cwd))) {
        return
      }
    }

    await refreshStatus()
  }

  const discardAllChanges = async () => {
    // 恢复所有已跟踪文件的 worktree 变更（保留 index）
    const cwd = await ensureGitRoot()
    if (!cwd) return

    const resetTracked = await runGit(['checkout', '--', '.'], cwd)
    if (!resetTracked) return
    // 删除所有未跟踪文件和目录
    if (!(await runGit(['clean', '-fd'], cwd))) return
    await refreshStatus()
  }

  const commit = async (message: string) => {
    if (await runGit(['commit', '-m', message])) {
      await refreshStatus()
      await loadCommits()
    }
  }

  const push = async () => {
    if (await runGit(['push'])) {
      await refreshStatus()
    }
  }

  const pull = async () => {
    if (await runGit(['pull'])) {
      await refreshStatus()
    }
  }

  const fetch = async () => {
    if (await runGit(['fetch'])) {
      await refreshStatus()
    }
  }

  const sync = async () => {
    if (!(await runGit(['pull']))) return
    if (!(await runGit(['push']))) return
    await refreshStatus()
  }

  const checkoutBranch = async (branchName: string) => {
    if (await runGit(['checkout', branchName])) {
      await refreshStatus()
      await loadBranches()
      await loadCommits()
    }
  }

  const initRepository = async () => {
    const path = currentPath.value
    if (!path || path === '~') return

    await shellApi.executeBackgroundProgram('git', ['init'], path)
    await refreshStatus()
  }

  const openDiffTab = async (file: FileChange, staged: boolean) => {
    selectedFile.value = file
    selectedFileIsStaged.value = staged

    const repoPath = await ensureGitRoot()
    if (!repoPath) return

    await editorStore.openDiffTab({
      repoPath,
      data: {
        filePath: file.path,
        staged,
      },
    })
  }

  const showCommitFileDiff = async (hash: string, filePath: string) => {
    const repoPath = await ensureGitRoot()
    if (!repoPath) return

    await editorStore.openDiffTab({
      repoPath,
      data: {
        filePath,
        commitHash: hash,
        staged: false,
      },
    })
  }

  watch(
    currentPath,
    () => {
      resolvedGitRoot = null
      refreshStatus()
      selectedFile.value = null
      selectedFileIsStaged.value = false
    },
    { immediate: true }
  )

  watch(repositoryRoot, () => {
    resolvedGitRoot = null
  })

  let isGitChangedListenerSetup = false
  const debouncedRefreshStatus = debounce(() => {
    void refreshStatus()
  }, 150)

  const setupGitChangedListener = async () => {
    if (isGitChangedListenerSetup) return
    isGitChangedListenerSetup = true

    try {
      await gitApi.onChanged(payload => {
        const root = repositoryRoot.value
        if (!root) return
        if (payload?.path && payload.path !== root) return
        debouncedRefreshStatus()
      })
    } catch (e) {
      console.warn('Failed to setup git:changed listener:', e)
    }
  }

  setupGitChangedListener()

  return {
    status,
    branches,
    commits,
    selectedFile,
    selectedFileIsStaged,
    isLoading,
    error,
    isVisible,
    panelWidth,

    isRepository,
    repositoryRoot,
    currentBranch,
    stagedCount,
    changedCount,
    currentPath,
    commitsHasMore,

    refreshStatus,
    loadBranches,
    loadCommits,
    loadMoreCommits,
    togglePanel,
    setPanelWidth,

    stageFile,
    stageFiles,
    stageAllFiles,
    unstageFile,
    unstageFiles,
    unstageAllFiles,
    discardFile,
    discardFiles,
    discardAllChanges,
    commit,
    push,
    pull,
    fetch,
    sync,
    initRepository,

    openDiffTab,
    showCommitFileDiff,
    checkoutBranch,
  }
})
