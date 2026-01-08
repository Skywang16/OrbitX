import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import {
  type SessionState,
  type TabContext,
  type TabId,
  type TabState,
  type UiState,
  type AiState,
  type GroupNode,
  type WorkspaceState,
} from '@/types/domain/storage'
import { createDefaultSessionState } from '@/types/utils/helpers'
import { storageApi } from '@/api/storage'

const isGroupNode = (value: unknown): value is GroupNode => {
  if (!value || typeof value !== 'object') return false
  const v = value as Record<string, unknown>
  const type = v.type

  if (type === 'leaf') {
    if (typeof v.id !== 'string') return false
    if (typeof v.groupId !== 'string') return false
    return true
  }

  if (type === 'split') {
    if (typeof v.id !== 'string') return false
    const direction = v.direction
    if (direction !== 'row' && direction !== 'column') return false
    if (typeof v.ratio !== 'number') return false
    if (!isGroupNode(v.first)) return false
    if (!isGroupNode(v.second)) return false
    return true
  }

  return false
}

const isTabContext = (value: unknown): value is TabContext => {
  if (!value || typeof value !== 'object') return false
  const kind = (value as { kind?: unknown }).kind
  if (kind === 'none') return true
  if (kind === 'terminal') {
    return typeof (value as { paneId?: unknown }).paneId === 'number'
  }
  if (kind === 'workspace') return typeof (value as { path?: unknown }).path === 'string'
  if (kind === 'git') return typeof (value as { repoPath?: unknown }).repoPath === 'string'
  return false
}

const normalizeLoadedWorkspaceState = (value: unknown): WorkspaceState | null => {
  if (!value || typeof value !== 'object') return null
  const input = value as Partial<WorkspaceState> & { groups?: unknown }
  if (!isGroupNode(input.root)) return null
  if (!input.groups || typeof input.groups !== 'object') return null

  const groupsRaw = input.groups as Record<string, unknown>
  const groupEntries = Object.entries(groupsRaw)
  if (groupEntries.length === 0) return null

  const groups: WorkspaceState['groups'] = {}

  for (const [groupId, groupValue] of groupEntries) {
    if (!groupValue || typeof groupValue !== 'object') continue
    const g = groupValue as Record<string, unknown>
    if (typeof g.id !== 'string' || g.id !== groupId) continue
    const tabsRaw = Array.isArray(g.tabs) ? (g.tabs as unknown[]) : []
    const activeTabId = typeof g.activeTabId === 'string' ? g.activeTabId : null

    const tabs: TabState[] = tabsRaw
      .map(tabRaw => {
        if (!tabRaw || typeof tabRaw !== 'object') return null
        const tab = tabRaw as Record<string, unknown>

        const type = tab.type
        if (type !== 'terminal' && type !== 'settings' && type !== 'diff') return null

        const originalId = tab.id
        if (typeof originalId !== 'string' || !originalId) return null
        const id = originalId as TabId

        const isActive = typeof tab.isActive === 'boolean' ? tab.isActive : false
        const data = (tab.data && typeof tab.data === 'object' ? tab.data : {}) as Record<string, unknown>

        let context: TabContext = { kind: 'none' }
        if (isTabContext(tab.context)) {
          context = tab.context
        } else if (type === 'diff') {
          const repoPath = (data as { repoPath?: unknown }).repoPath
          if (typeof repoPath === 'string' && repoPath) {
            context = { kind: 'git', repoPath }
          }
        }

        if (type === 'terminal') {
          if (context.kind !== 'terminal') return null
          return {
            type: 'terminal',
            id,
            isActive,
            context,
            data: {
              cwd: typeof data.cwd === 'string' ? data.cwd : undefined,
              shellName: typeof data.shellName === 'string' ? data.shellName : undefined,
            },
          } as TabState
        }

        if (type === 'settings') {
          const normalizedData: Record<string, unknown> = {}
          if (typeof data.lastSection === 'string') normalizedData.lastSection = data.lastSection
          return {
            type: 'settings',
            id,
            isActive,
            context: { kind: 'none' },
            data: normalizedData,
          } as TabState
        }

        if (context.kind !== 'git') return null
        if (typeof data.filePath !== 'string' || !data.filePath) return null
        return {
          type: 'diff',
          id,
          isActive,
          context,
          data: {
            filePath: data.filePath,
            staged: typeof data.staged === 'boolean' ? data.staged : undefined,
            commitHash: typeof data.commitHash === 'string' ? data.commitHash : undefined,
          },
        } as TabState
      })
      .filter((t): t is TabState => !!t)

    groups[groupId] = {
      id: groupId,
      tabs,
      activeTabId: activeTabId && tabs.some(t => t.id === activeTabId) ? activeTabId : (tabs[0]?.id ?? null),
    }
  }

  const activeGroupId =
    typeof input.activeGroupId === 'string' && input.activeGroupId in groups
      ? input.activeGroupId
      : Object.keys(groups)[0]!

  return {
    root: input.root,
    groups,
    activeGroupId,
  }
}

const normalizeLoadedSessionState = (raw: unknown): SessionState => {
  const fallback = createDefaultSessionState()
  if (!raw || typeof raw !== 'object') return fallback

  const input = raw as Partial<SessionState>
  const workspace = normalizeLoadedWorkspaceState(input.workspace) ?? fallback.workspace

  return {
    version: typeof input.version === 'number' ? input.version : 1,
    workspace,
    ui: (input.ui && typeof input.ui === 'object' ? input.ui : fallback.ui) as UiState,
    ai: (input.ai && typeof input.ai === 'object' ? input.ai : fallback.ai) as AiState,
    timestamp: typeof input.timestamp === 'string' ? input.timestamp : fallback.timestamp,
  }
}

/**
 * 会话状态管理Store - 统一 tab 管理
 */
export const useSessionStore = defineStore('session', () => {
  /** 当前会话状态 */
  const sessionState = ref<SessionState>(createDefaultSessionState())

  /** 是否正在加载 */
  const isLoading = ref(false)

  /** 是否正在保存 */
  const isSaving = ref(false)
  /** 有保存在进行时是否积压了新的保存请求 */
  const pendingSave = ref(false)

  /** 错误信息 */
  const error = ref<string | null>(null)

  /** 是否已初始化 */
  const initialized = ref(false)

  /** 是否正在执行操作 */
  const isOperating = computed(() => isLoading.value || isSaving.value)

  /** Workspace 状态 */
  const workspaceState = computed(() => sessionState.value.workspace)
  const activeGroupId = computed(() => sessionState.value.workspace.activeGroupId)
  const activeGroup = computed(() => sessionState.value.workspace.groups[activeGroupId.value] ?? null)
  const activeTabId = computed<TabId | null>(() => activeGroup.value?.activeTabId ?? null)
  const activeTab = computed<TabState | null>(() => {
    const group = activeGroup.value
    if (!group) return null
    const id = group.activeTabId
    if (!id) return null
    return group.tabs.find(t => t.id === id) ?? null
  })

  /** UI状态 */
  const uiState = computed(() => sessionState.value.ui)

  /** AI状态 */
  const aiState = computed(() => sessionState.value.ai)

  const saveSessionState = async (): Promise<void> => {
    if (isSaving.value) {
      pendingSave.value = true
      return
    }

    isSaving.value = true
    error.value = null

    try {
      do {
        pendingSave.value = false
        sessionState.value.timestamp = new Date().toISOString()
        await storageApi.saveSessionState(sessionState.value)
      } while (pendingSave.value)
    } finally {
      isSaving.value = false
    }
  }

  const loadSessionState = async (): Promise<void> => {
    if (isLoading.value) return
    isLoading.value = true
    error.value = null
    const state = await storageApi.loadSessionState().finally(() => {
      isLoading.value = false
    })
    if (state) {
      sessionState.value = normalizeLoadedSessionState(state)
    }
  }

  const updateWorkspaceState = (workspace: WorkspaceState): void => {
    sessionState.value.workspace = workspace
    saveSessionState().catch(() => {})
  }

  const updateUiState = (updates: Partial<UiState>): void => {
    sessionState.value.ui = {
      ...sessionState.value.ui,
      ...updates,
    }
    saveSessionState().catch(() => {})
  }

  const updateAiState = (updates: Partial<AiState>): void => {
    sessionState.value.ai = {
      ...sessionState.value.ai,
      ...updates,
    }
    saveSessionState().catch(() => {})
  }

  const clearError = (): void => {
    error.value = null
  }

  const cleanup = (): void => {}
  const initialize = async (): Promise<void> => {
    if (initialized.value) return

    try {
      await loadSessionState()
      initialized.value = true
    } catch (err) {
      console.error('会话状态管理初始化失败:', err)
      throw err
    }
  }

  return {
    sessionState: readonly(sessionState),
    isLoading: readonly(isLoading),
    isSaving: readonly(isSaving),
    error: readonly(error),
    initialized: readonly(initialized),

    isOperating,
    workspaceState,
    activeGroupId,
    activeGroup,
    activeTabId,
    activeTab,
    uiState,
    aiState,

    // 核心方法
    saveSessionState,
    loadSessionState,
    initialize,
    cleanup,

    // 状态更新方法
    updateWorkspaceState,
    updateUiState,
    updateAiState,

    // 工具方法
    clearError,
  }
})
