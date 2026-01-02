import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { debounce } from 'lodash-es'
import { gitApi, shellApi } from '@/api'
import { useTerminalStore } from '@/stores/Terminal'
import { useSessionStore } from '@/stores/session'
import { getHandler } from '@/stores/TabHandlers'
import type { BranchInfo, CommitInfo, FileChange, RepositoryStatus } from '@/api/git/types'
import type { DiffTabState } from '@/types/domain/storage'

export const useGitStore = defineStore('git', () => {
  const terminalStore = useTerminalStore()

  const status = ref<RepositoryStatus | null>(null)
  const branches = ref<BranchInfo[]>([])
  const commits = ref<CommitInfo[]>([])
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

  const currentPath = computed(() => terminalStore.currentWorkingDirectory)

  let _watchingRoot: string | null = null
  let _watchStartInFlight: Promise<void> | null = null
  const ensureGitWatch = async (root: string | null) => {
    if (!root) {
      if (_watchingRoot) {
        try {
          await gitApi.watchStop()
        } catch (e) {
          console.warn('Failed to stop git watcher:', e)
        } finally {
          _watchingRoot = null
        }
      }
      return
    }

    if (_watchingRoot === root) return

    if (_watchStartInFlight) {
      await _watchStartInFlight
      if (_watchingRoot === root) return
    }

    _watchStartInFlight = (async () => {
      try {
        await gitApi.watchStart(root)
        _watchingRoot = root
      } catch (e) {
        console.warn('Failed to start git watcher:', e)
      } finally {
        _watchStartInFlight = null
      }
    })()

    await _watchStartInFlight
  }

  let _pendingRefresh = false
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
      _pendingRefresh = true
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
      if (_pendingRefresh) {
        _pendingRefresh = false
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

    try {
      commits.value = await gitApi.getCommits(path, limit)
    } catch (e) {
      console.error('Failed to load commits:', e)
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

  const requireGitCwd = (): string | null => {
    const path = requireRepoPath()
    if (!path) return null
    return repositoryRoot.value ?? path
  }

  const runGit = async (args: string[]) => {
    const cwd = requireGitCwd()
    if (!cwd) return
    await shellApi.executeBackgroundProgram('git', args, cwd)
  }

  const stageFile = async (file: FileChange) => {
    await runGit(['add', '--', file.path])
    await refreshStatus()
  }

  const stageAllFiles = async () => {
    await runGit(['add', '-A'])
    await refreshStatus()
  }

  const unstageFile = async (file: FileChange) => {
    await runGit(['restore', '--staged', '--', file.path])
    await refreshStatus()
  }

  const unstageAllFiles = async () => {
    await runGit(['restore', '--staged', '.'])
    await refreshStatus()
  }

  const discardFile = async (file: FileChange) => {
    // Tracked changes
    await runGit(['restore', '--', file.path])
    // Untracked files (no-op for tracked paths)
    await runGit(['clean', '-f', '--', file.path])
    await refreshStatus()
  }

  const discardAllChanges = async () => {
    await runGit(['restore', '.'])
    await runGit(['clean', '-fd'])
    await refreshStatus()
  }

  const commit = async (message: string) => {
    await runGit(['commit', '-m', message])
    await refreshStatus()
    await loadCommits()
  }

  const push = async () => {
    await runGit(['push'])
    await refreshStatus()
  }

  const pull = async () => {
    await runGit(['pull'])
    await refreshStatus()
  }

  const fetch = async () => {
    await runGit(['fetch'])
    await refreshStatus()
  }

  const sync = async () => {
    await runGit(['pull'])
    await runGit(['push'])
    await refreshStatus()
  }

  // === Diff Tab 管理 ===
  let _diffTabIdCounter = Date.now()

  const openDiffTab = async (file: FileChange, staged: boolean) => {
    const sessionStore = useSessionStore()

    selectedFile.value = file
    selectedFileIsStaged.value = staged

    // 检查是否已存在相同文件的 diff tab
    const existingTab = sessionStore.tabs.find(
      t =>
        t.type === 'diff' &&
        (t as DiffTabState).data.filePath === file.path &&
        (t as DiffTabState).data.staged === staged &&
        !(t as DiffTabState).data.commitHash
    )

    if (existingTab) {
      await getHandler('diff').activate(existingTab.id)
      return
    }

    const tabId = _diffTabIdCounter++
    const newTab: DiffTabState = {
      type: 'diff',
      id: tabId,
      isActive: false,
      data: {
        filePath: file.path,
        staged,
      },
    }

    sessionStore.addTab(newTab)
    await getHandler('diff').activate(tabId)
  }

  const showCommitFileDiff = async (hash: string, filePath: string) => {
    const sessionStore = useSessionStore()

    // 检查是否已存在相同的 commit file diff tab
    const existingTab = sessionStore.tabs.find(
      t =>
        t.type === 'diff' &&
        (t as DiffTabState).data.filePath === filePath &&
        (t as DiffTabState).data.commitHash === hash
    )

    if (existingTab) {
      await getHandler('diff').activate(existingTab.id)
      return
    }

    const tabId = _diffTabIdCounter++
    const newTab: DiffTabState = {
      type: 'diff',
      id: tabId,
      isActive: false,
      data: {
        filePath,
        commitHash: hash,
      },
    }

    sessionStore.addTab(newTab)
    await getHandler('diff').activate(tabId)
  }

  watch(
    currentPath,
    () => {
      refreshStatus()
      selectedFile.value = null
      selectedFileIsStaged.value = false
    },
    { immediate: true }
  )

  let _isGitChangedListenerSetup = false
  const debouncedRefreshStatus = debounce(() => {
    void refreshStatus()
  }, 150)

  const setupGitChangedListener = async () => {
    if (_isGitChangedListenerSetup) return
    _isGitChangedListenerSetup = true

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
    currentPath,

    refreshStatus,
    loadBranches,
    loadCommits,
    togglePanel,
    setPanelWidth,

    stageFile,
    stageAllFiles,
    unstageFile,
    unstageAllFiles,
    discardFile,
    discardAllChanges,
    commit,
    push,
    pull,
    fetch,
    sync,

    openDiffTab,
    showCommitFileDiff,
  }
})
