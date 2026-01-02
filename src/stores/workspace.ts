import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import workspaceService, { type SessionRecord, type WorkspaceRecord } from '@/api/workspace/service'
import type { Message } from '@/types'
import { useTerminalStore } from '@/stores/Terminal'
import { useSessionStore } from '@/stores/session'

/** 未分组工作区的特殊路径（与后端保持一致） */
export const UNGROUPED_WORKSPACE_PATH = '__ungrouped__'

export const useWorkspaceStore = defineStore('workspace-store', () => {
  // 内部状态 - 当前加载的工作区数据
  const _loadedWorkspacePath = ref<string | null>(null)
  const currentWorkspace = ref<WorkspaceRecord | null>(null)
  const sessions = ref<SessionRecord[]>([])
  const currentSession = ref<SessionRecord | null>(null)
  const messages = ref<Message[]>([])
  const isLoading = ref(false)
  const recentWorkspaces = ref<WorkspaceRecord[]>([])

  // 工作区路径：终端 tab 用终端 cwd，其他 tab 用未分组
  const currentWorkspacePath = computed(() => {
    const sessionStore = useSessionStore()
    const terminalStore = useTerminalStore()

    // 获取当前激活的 tab
    const activeTab = sessionStore.tabs.find(t => t.isActive)

    // 非终端 tab 或无 tab 时使用未分组
    if (!activeTab || activeTab.type !== 'terminal') {
      return UNGROUPED_WORKSPACE_PATH
    }

    // 终端 tab：查找对应终端的 cwd
    const terminal = terminalStore.terminals.find(t => t.id === activeTab.id)
    const cwd = terminal?.cwd
    if (!cwd || cwd === '~') return UNGROUPED_WORKSPACE_PATH
    return cwd
  })

  const loadRecentWorkspaces = async (limit = 10) => {
    recentWorkspaces.value = await workspaceService.listRecent(limit)
  }

  const fetchWorkspace = async (path: string) => {
    currentWorkspace.value = await workspaceService.getOrCreate(path)
  }

  const fetchSessions = async (path: string) => {
    sessions.value = await workspaceService.listSessions(path)
  }

  const fetchActiveSession = async (path: string) => {
    currentSession.value = await workspaceService.getActiveSession(path)
  }

  const fetchMessages = async (sessionId: number) => {
    messages.value = await workspaceService.getMessages(sessionId)
  }

  // 加载指定工作区的数据（会话列表、当前会话、消息）
  const loadWorkspaceData = async (path: string, force = false) => {
    if (!path) return
    if (!force && isLoading.value && _loadedWorkspacePath.value === path) {
      return
    }

    isLoading.value = true
    _loadedWorkspacePath.value = path
    try {
      await fetchWorkspace(path)
      await fetchSessions(path)
      await fetchActiveSession(path)
      if (currentSession.value) {
        await fetchMessages(currentSession.value.id)
      } else {
        messages.value = []
      }
      await loadRecentWorkspaces()
    } finally {
      isLoading.value = false
    }
  }

  // 切换会话
  const switchSession = async (sessionId: number) => {
    const path = currentWorkspacePath.value
    if (!path) return
    if (currentSession.value?.id === sessionId) return
    await workspaceService.setActiveSession(path, sessionId)
    currentSession.value = sessions.value.find(session => session.id === sessionId) ?? currentSession.value
    if (currentSession.value) {
      await fetchMessages(currentSession.value.id)
    }
  }

  // 创建新会话
  const createSession = async (title?: string) => {
    const path = currentWorkspacePath.value
    if (!path) return
    const created = await workspaceService.createSession(path, title)
    sessions.value.unshift(created)
    await switchSession(created.id)
  }

  return {
    currentWorkspacePath,
    currentWorkspace,
    sessions,
    currentSession,
    messages,
    isLoading,
    recentWorkspaces,
    loadWorkspaceData,
    switchSession,
    createSession,
    fetchMessages,
    loadRecentWorkspaces,
  }
})
