import workspaceService, { type SessionRecord, type WorkspaceRecord } from '@/api/workspace/service'
import { useTerminalStore } from '@/stores/Terminal'
import { useSessionStore } from '@/stores/session'
import { getWorkspacePathForTab } from '@/tabs/context'
import type { Message } from '@/types'
import type { TabState } from '@/types/domain/storage'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

/** 未分组工作区的特殊路径（与后端保持一致） */
export const UNGROUPED_WORKSPACE_PATH = '__ungrouped__'

export const useWorkspaceStore = defineStore('workspace-store', () => {
  // 内部状态 - 当前加载的工作区数据
  const loadedWorkspacePath = ref<string | null>(null)
  const currentWorkspace = ref<WorkspaceRecord | null>(null)
  const sessions = ref<SessionRecord[]>([])
  const currentSession = ref<SessionRecord | null>(null)
  const messages = ref<Message[]>([])
  const messagesBySession = ref<Record<number, Message[]>>({})
  const messageIndex = new Map<number, { sessionId: number; index: number }>()
  const isLoading = ref(false)
  const recentWorkspaces = ref<WorkspaceRecord[]>([])

  const sessionStore = useSessionStore()
  const terminalStore = useTerminalStore()

  const resolveTabPath = (tab: TabState): string | null => {
    const path = getWorkspacePathForTab(tab, { terminals: terminalStore.terminals })
    return path && path !== '~' ? path : null
  }

  // 工作区路径：始终以当前选中的 tab 为唯一来源
  const currentWorkspacePath = computed(() => {
    const activeTab = sessionStore.activeTab
    if (!activeTab) return UNGROUPED_WORKSPACE_PATH

    return resolveTabPath(activeTab) ?? UNGROUPED_WORKSPACE_PATH
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
    const loaded = await workspaceService.getMessages(sessionId)
    clearIndexForSession(sessionId)
    messagesBySession.value[sessionId] = loaded
    indexSessionMessages(sessionId, loaded)
    if (currentSession.value?.id === sessionId) {
      messages.value = loaded
    }
  }

  // 加载指定工作区的数据（会话列表、当前会话、消息）
  const loadWorkspaceData = async (path: string, force = false) => {
    if (!path) return
    if (!force && isLoading.value && loadedWorkspacePath.value === path) {
      return
    }

    isLoading.value = true
    loadedWorkspacePath.value = path
    try {
      await fetchWorkspace(path)
      await fetchSessions(path)
      await fetchActiveSession(path)
      if (currentSession.value) {
        await fetchMessages(currentSession.value.id)
        messages.value = messagesBySession.value[currentSession.value.id] || []
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
      messages.value = messagesBySession.value[currentSession.value.id] || []
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

  const indexSessionMessages = (sessionId: number, list: Message[]) => {
    for (let i = 0; i < list.length; i++) {
      messageIndex.set(list[i].id, { sessionId, index: i })
    }
  }

  const clearIndexForSession = (sessionId: number) => {
    const existing = messagesBySession.value[sessionId]
    if (!existing) return
    for (const msg of existing) {
      const pos = messageIndex.get(msg.id)
      if (pos?.sessionId === sessionId) {
        messageIndex.delete(msg.id)
      }
    }
  }

  const upsertMessage = (message: Message) => {
    const sessionId = message.sessionId
    const existingPos = messageIndex.get(message.id)
    if (existingPos) {
      const existingList = messagesBySession.value[existingPos.sessionId]
      if (existingList?.[existingPos.index]) {
        // Replace in place; keep stable ordering.
        existingList[existingPos.index] = message
      }

      // If the session changed (shouldn't happen), reindex into the new session list.
      if (existingPos.sessionId !== sessionId) {
        messageIndex.delete(message.id)
        const nextList = messagesBySession.value[sessionId] || []
        nextList.push(message)
        messagesBySession.value[sessionId] = nextList
        messageIndex.set(message.id, { sessionId, index: nextList.length - 1 })
      }
    } else {
      const list = messagesBySession.value[sessionId] || []
      list.push(message)
      messagesBySession.value[sessionId] = list
      messageIndex.set(message.id, { sessionId, index: list.length - 1 })
    }

    if (currentSession.value?.id === sessionId) {
      messages.value = messagesBySession.value[sessionId] || []
    }
  }

  const appendBlock = (messageId: number, block: Message['blocks'][number]) => {
    const pos = messageIndex.get(messageId)
    if (!pos) {
      console.warn('[appendBlock] message not found in index!')
      return
    }
    const msg = messagesBySession.value[pos.sessionId]?.[pos.index]
    if (!msg) {
      console.warn('[appendBlock] message not found in messagesBySession!')
      return
    }
    msg.blocks.push(block)
  }

  const updateBlock = (messageId: number, blockId: string, block: Message['blocks'][number]) => {
    const pos = messageIndex.get(messageId)
    if (!pos) {
      console.warn('[updateBlock] message not found in index!')
      return
    }
    const msg = messagesBySession.value[pos.sessionId]?.[pos.index]
    if (!msg) {
      console.warn('[updateBlock] message not found in messagesBySession!')
      return
    }
    const idx = msg.blocks.findIndex(b => 'id' in b && b.id === blockId)
    if (idx >= 0) {
      msg.blocks[idx] = block
    } else {
      console.warn('[updateBlock] block not found in message!')
    }
  }

  const finishMessage = (
    messageId: number,
    patch: Partial<Pick<Message, 'status' | 'finishedAt' | 'durationMs' | 'tokenUsage' | 'contextUsage'>>
  ) => {
    const pos = messageIndex.get(messageId)
    if (!pos) return
    const msg = messagesBySession.value[pos.sessionId]?.[pos.index]
    if (!msg) return
    Object.assign(msg, patch)
  }

  const getCachedMessages = (sessionId: number) => {
    return messagesBySession.value[sessionId] || []
  }

  // 清空当前会话（开始新对话时使用）
  const clearCurrentSession = async () => {
    const path = currentWorkspacePath.value
    if (path) {
      // 通知后端清空活跃会话
      await workspaceService.clearActiveSession(path)
    }
    currentSession.value = null
    messages.value = []
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
    clearCurrentSession,
    fetchMessages,
    upsertMessage,
    appendBlock,
    updateBlock,
    finishMessage,
    getCachedMessages,
    loadRecentWorkspaces,
  }
})
