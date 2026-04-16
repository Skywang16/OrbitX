import { workspaceApi, type ThreadRecord, type ThreadViewRecord, type WorkspaceRecord } from '@/api/workspace'
import type { Message, SubagentRecord } from '@/types'
import { defineStore } from 'pinia'
import { computed, reactive, ref, shallowRef } from 'vue'

const MAX_CACHED_THREADS = 10
const INITIAL_PAGE_SIZE = 100
const LOAD_MORE_PAGE_SIZE = 50

export interface WorkspaceNode {
  workspace: WorkspaceRecord
  threadViews: ThreadViewRecord[]
  isLoading: boolean
}

export const useWorkspaceStore = defineStore('workspace', () => {
  // State
  const tree = shallowRef<Map<string, WorkspaceNode>>(new Map())
  const selectedThread = ref<ThreadRecord | null>(null)
  const messagesByThreadId = reactive<Map<number, Message[]>>(new Map())
  const subagentsByThreadId = reactive<Map<number, SubagentRecord[]>>(new Map())
  const messagesHasMoreMap = reactive<Map<number, boolean>>(new Map())
  const messageIdToThreadId = new Map<number, number>()
  const activeWorkspacePath = ref<string | null>(null)
  const threadAccessOrder: number[] = [] // LRU tracking: oldest first
  const messageFetches = new Map<number, Promise<void>>()

  // Computed
  const messages = computed<Message[]>(() => {
    const threadId = selectedThread.value?.id
    if (!threadId) return []
    return messagesByThreadId.get(threadId) ?? []
  })

  const messagesHasMore = computed(() => {
    const threadId = selectedThread.value?.id
    if (!threadId) return false
    return messagesHasMoreMap.get(threadId) ?? false
  })

  const workspaces = computed(() => {
    return Array.from(tree.value.values())
      .map(node => node.workspace)
      .sort((a, b) => b.lastAccessedAt - a.lastAccessedAt)
  })

  const currentWorkspacePath = computed(() => activeWorkspacePath.value)

  const hasWorkspace = computed(() => currentWorkspacePath.value !== null)

  // LRU cache management
  const touchThread = (threadId: number) => {
    const idx = threadAccessOrder.indexOf(threadId)
    if (idx >= 0) threadAccessOrder.splice(idx, 1)
    threadAccessOrder.push(threadId)
  }

  const evictOldThreads = () => {
    const currentId = selectedThread.value?.id
    let guard = threadAccessOrder.length
    while (threadAccessOrder.length > MAX_CACHED_THREADS && guard > 0) {
      guard--
      const oldest = threadAccessOrder[0]
      if (oldest === currentId) {
        threadAccessOrder.shift()
        threadAccessOrder.push(oldest)
        continue
      }
      threadAccessOrder.shift()
      const cached = messagesByThreadId.get(oldest)
      if (cached) {
        for (const msg of cached) messageIdToThreadId.delete(msg.id)
      }
      messagesByThreadId.delete(oldest)
      subagentsByThreadId.delete(oldest)
    }
  }

  // Internal helpers
  const indexMessageList = (list: Message[]) => {
    for (const msg of list) {
      messageIdToThreadId.set(msg.id, msg.threadId)
    }
  }

  const setThreadMessages = (threadId: number, list: Message[]) => {
    messagesByThreadId.set(threadId, reactive(list))
    indexMessageList(list)
    touchThread(threadId)
    evictOldThreads()
  }

  const ensureThreadMessages = (threadId: number): Message[] => {
    touchThread(threadId)
    const existing = messagesByThreadId.get(threadId)
    if (existing) return existing
    const created = reactive<Message[]>([])
    messagesByThreadId.set(threadId, created)
    evictOldThreads()
    return created
  }

  const setThreadSubagents = (threadId: number, list: SubagentRecord[]) => {
    subagentsByThreadId.set(threadId, reactive(list))
    touchThread(threadId)
    evictOldThreads()
  }

  const ensureThreadSubagents = (threadId: number): SubagentRecord[] => {
    touchThread(threadId)
    const existing = subagentsByThreadId.get(threadId)
    if (existing) return existing
    const created = reactive<SubagentRecord[]>([])
    subagentsByThreadId.set(threadId, created)
    evictOldThreads()
    return created
  }

  const resolveThreadIdByMessageId = (messageId: number): number | null => {
    const indexed = messageIdToThreadId.get(messageId)
    if (typeof indexed === 'number') return indexed
    for (const [threadId, list] of messagesByThreadId.entries()) {
      if (list.some(m => m.id === messageId)) {
        messageIdToThreadId.set(messageId, threadId)
        return threadId
      }
    }
    return null
  }

  // Tree operations
  const loadTree = async () => {
    const list = await workspaceApi.listRecent(20)
    const newTree = new Map<string, WorkspaceNode>()
    for (const ws of list) {
      newTree.set(ws.path, {
        workspace: ws,
        threadViews: tree.value.get(ws.path)?.threadViews || [],
        isLoading: false,
      })
    }
    tree.value = newTree

    // Restore last active session
    if (!selectedThread.value) {
      for (const ws of list) {
        if (ws.activeThreadId) {
          await selectThreadById(ws.activeThreadId, ws.path)
          break
        }
      }
    }
  }

  const loadThreadViews = async (path: string) => {
    const node = tree.value.get(path)
    if (!node || node.isLoading) return

    // Mark as loading
    tree.value = new Map(tree.value).set(path, { ...node, isLoading: true })

    try {
      const threadViews = await workspaceApi.listThreadViews(path)
      tree.value = new Map(tree.value).set(path, {
        ...node,
        threadViews,
        isLoading: false,
      })
      if (selectedThread.value?.workspacePath === path) {
        const refreshed = threadViews.find(item => item.thread.id === selectedThread.value?.id)?.thread
        if (refreshed) {
          selectedThread.value = refreshed
        }
      }
    } catch (error) {
      console.warn(`Failed to load threads for workspace '${path}':`, error)
      tree.value = new Map(tree.value).set(path, { ...node, isLoading: false })
    }
  }

  const getNode = (path: string | null): WorkspaceNode | undefined => {
    if (!path) return undefined
    return tree.value.get(path)
  }

  const getThreadViews = (path: string | null): ThreadViewRecord[] => {
    return getNode(path)?.threadViews ?? []
  }

  const getThreads = (path: string | null): ThreadRecord[] => {
    return getThreadViews(path).map(item => item.thread)
  }

  const getTopLevelThreads = (path: string | null): ThreadRecord[] => {
    return getThreads(path).filter(thread => thread.parentThreadId == null)
  }

  const getThreadView = (threadId: number, workspacePath?: string | null): ThreadViewRecord | undefined => {
    const path = workspacePath ?? selectedThread.value?.workspacePath ?? activeWorkspacePath.value
    return getThreadViews(path).find(item => item.thread.id === threadId)
  }

  // Thread operations
  const loadInitialMessages = async (threadId: number) => {
    setThreadMessages(threadId, [])
    const loaded = await workspaceApi.getThreadMessages(threadId, INITIAL_PAGE_SIZE)
    setThreadMessages(threadId, loaded)
    const subagents = await workspaceApi.listSubagents(threadId)
    setThreadSubagents(threadId, subagents)
    messagesHasMoreMap.set(threadId, loaded.length >= INITIAL_PAGE_SIZE)
  }

  const selectThread = async (thread: ThreadRecord) => {
    if (selectedThread.value?.id === thread.id) return
    selectedThread.value = thread
    activeWorkspacePath.value = thread.workspacePath
    workspaceApi.setActiveThread(thread.workspacePath, thread.id)
    await loadInitialMessages(thread.id)
  }

  const selectThreadById = async (threadId: number, workspacePath: string) => {
    if (selectedThread.value?.id === threadId) {
      // Same thread is already selected (e.g., user sent another message to current thread).
      // We still need to refresh threadViews so the sidebar title reflects any title update
      // that the backend wrote before emitting agent_run_created.
      void loadThreadViews(workspacePath)
      return
    }
    const threadViews = await workspaceApi.listThreadViews(workspacePath)
    const existingNode = tree.value.get(workspacePath)
    const workspace: WorkspaceRecord = existingNode?.workspace ?? {
      path: workspacePath,
      displayName: null,
      lastAccessedAt: Date.now(),
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }
    tree.value = new Map(tree.value).set(workspacePath, {
      workspace,
      threadViews,
      isLoading: false,
    })
    const thread =
      threadViews.find(item => item.thread.id === threadId)?.thread ?? (await workspaceApi.getThread(threadId))
    if (thread) {
      selectedThread.value = thread
      activeWorkspacePath.value = workspacePath
      await loadInitialMessages(threadId)
    }
  }

  const loadMoreMessages = async () => {
    const threadId = selectedThread.value?.id
    if (!threadId) return
    const list = messagesByThreadId.get(threadId)
    if (!list || list.length === 0) return
    const oldestId = list[0].id
    const older = await workspaceApi.getThreadMessages(threadId, LOAD_MORE_PAGE_SIZE, oldestId)
    if (older.length === 0) {
      messagesHasMoreMap.set(threadId, false)
      return
    }
    list.unshift(...older)
    indexMessageList(older)
    messagesHasMoreMap.set(threadId, older.length >= LOAD_MORE_PAGE_SIZE)
  }

  const clearSelection = () => {
    selectedThread.value = null
  }

  const setActiveWorkspace = (path: string | null) => {
    activeWorkspacePath.value = path
  }

  const deleteThread = async (threadId: number, workspacePath: string) => {
    await workspaceApi.deleteThread(threadId)
    const node = tree.value.get(workspacePath)
    if (node) {
      tree.value = new Map(tree.value).set(workspacePath, {
        ...node,
        threadViews: node.threadViews.filter(item => item.thread.id !== threadId),
      })
    }
    if (selectedThread.value?.id === threadId) {
      selectedThread.value = null
      workspaceApi.clearActiveThread(workspacePath)
    }
    const cached = messagesByThreadId.get(threadId)
    if (cached) {
      for (const msg of cached) messageIdToThreadId.delete(msg.id)
    }
    messagesByThreadId.delete(threadId)
    subagentsByThreadId.delete(threadId)
  }

  const deleteWorkspace = async (workspacePath: string) => {
    await workspaceApi.deleteWorkspace(workspacePath)
    const node = tree.value.get(workspacePath)
    if (node) {
      for (const view of node.threadViews) {
        const cached = messagesByThreadId.get(view.thread.id)
        if (cached) {
          for (const msg of cached) messageIdToThreadId.delete(msg.id)
        }
        messagesByThreadId.delete(view.thread.id)
        subagentsByThreadId.delete(view.thread.id)
      }
    }
    const newTree = new Map(tree.value)
    newTree.delete(workspacePath)
    tree.value = newTree
    if (selectedThread.value?.workspacePath === workspacePath) {
      selectedThread.value = null
    }
    if (activeWorkspacePath.value === workspacePath) {
      activeWorkspacePath.value = null
    }
  }

  const createThread = async (workspacePath: string, title?: string) => {
    await workspaceApi.getOrCreate(workspacePath)
    const thread = await workspaceApi.createThread(workspacePath, title ?? '')
    const node = tree.value.get(workspacePath)
    if (node) {
      tree.value = new Map(tree.value).set(workspacePath, {
        ...node,
        threadViews: [{ thread, timeline: [], executionTree: [] }, ...node.threadViews],
      })
    }
    selectedThread.value = thread
    activeWorkspacePath.value = workspacePath
    workspaceApi.setActiveThread(workspacePath, thread.id)
    setThreadMessages(thread.id, [])
    setThreadSubagents(thread.id, [])
    return thread
  }

  // Message operations (for stream updates)
  const upsertMessage = (message: Message) => {
    const list = ensureThreadMessages(message.threadId)
    const idx = list.findIndex(m => m.id === message.id)
    if (idx >= 0) {
      list[idx] = message
    } else {
      list.push(message)
    }
    messageIdToThreadId.set(message.id, message.threadId)
  }

  const appendBlock = (messageId: number, block: Message['blocks'][number]) => {
    const threadId = resolveThreadIdByMessageId(messageId)
    if (!threadId) {
      console.warn(`[workspace] appendBlock: thread not found for message ${messageId}`)
      return
    }
    const list = messagesByThreadId.get(threadId)
    if (!list) {
      console.warn(`[workspace] appendBlock: message list not found for thread ${threadId}`)
      return
    }
    const msg = list.find(m => m.id === messageId)
    if (!msg) {
      console.warn(`[workspace] appendBlock: message ${messageId} not found`)
      return
    }
    msg.blocks.push(block)
  }

  const updateBlock = (messageId: number, blockId: string, block: Message['blocks'][number]) => {
    const threadId = resolveThreadIdByMessageId(messageId)
    if (!threadId) {
      console.warn(`[workspace] updateBlock: thread not found for message ${messageId}`)
      return
    }
    const list = messagesByThreadId.get(threadId)
    if (!list) {
      console.warn(`[workspace] updateBlock: message list not found for thread ${threadId}`)
      return
    }
    const msg = list.find(m => m.id === messageId)
    if (!msg) {
      console.warn(`[workspace] updateBlock: message ${messageId} not found`)
      return
    }
    const idx = msg.blocks.findIndex(b => 'id' in b && b.id === blockId)
    if (idx >= 0) {
      msg.blocks[idx] = block
    } else {
      console.warn(`[workspace] updateBlock: block ${blockId} not found in message ${messageId}`)
    }
  }

  const finishMessage = (
    messageId: number,
    patch: Partial<Pick<Message, 'status' | 'finishedAt' | 'durationMs' | 'tokenUsage' | 'contextUsage'>>
  ) => {
    const threadId = resolveThreadIdByMessageId(messageId)
    if (!threadId) {
      console.warn(`[workspace] finishMessage: thread not found for message ${messageId}`)
      return
    }
    const list = messagesByThreadId.get(threadId)
    if (!list) {
      console.warn(`[workspace] finishMessage: message list not found for thread ${threadId}`)
      return
    }
    const msg = list.find(m => m.id === messageId)
    if (!msg) {
      console.warn(`[workspace] finishMessage: message ${messageId} not found`)
      return
    }
    Object.assign(msg, patch)
  }

  const upsertSubagent = (subagent: SubagentRecord) => {
    const list = ensureThreadSubagents(subagent.parentThreadId)
    const idx = list.findIndex(item => item.id === subagent.id)
    if (idx >= 0) {
      list[idx] = subagent
    } else {
      list.push(subagent)
      list.sort((left, right) => {
        if (left.createdAt === right.createdAt) return left.id.localeCompare(right.id)
        return left.createdAt.localeCompare(right.createdAt)
      })
    }
  }

  const fetchSubagents = async (threadId: number) => {
    const loaded = await workspaceApi.listSubagents(threadId)
    setThreadSubagents(threadId, loaded)
  }

  const getSubagents = (threadId: number) => subagentsByThreadId.get(threadId) ?? []

  const fetchMessages = async (threadId: number) => {
    const existing = messageFetches.get(threadId)
    if (existing) {
      await existing
      return
    }

    const request = (async () => {
      const loaded = await workspaceApi.getThreadMessages(threadId, INITIAL_PAGE_SIZE)
      setThreadMessages(threadId, loaded)
      messagesHasMoreMap.set(threadId, loaded.length >= INITIAL_PAGE_SIZE)
    })()

    messageFetches.set(threadId, request)
    try {
      await request
    } finally {
      messageFetches.delete(threadId)
    }
  }

  const getCachedMessages = (threadId: number) => messagesByThreadId.get(threadId) ?? []

  return {
    // State
    tree,
    selectedThread,
    messages,
    messagesHasMore,
    subagentsByThreadId,
    activeWorkspacePath,
    workspaces,
    currentWorkspacePath,
    hasWorkspace,
    // Tree
    loadTree,
    loadThreadViews,
    getNode,
    getThreads,
    getTopLevelThreads,
    getThreadViews,
    getThreadView,
    // Thread
    selectThread,
    selectThreadById,
    clearSelection,
    setActiveWorkspace,
    createThread,
    deleteThread,
    deleteWorkspace,
    // Message
    upsertMessage,
    appendBlock,
    updateBlock,
    finishMessage,
    fetchMessages,
    upsertSubagent,
    fetchSubagents,
    getSubagents,
    loadMoreMessages,
    getCachedMessages,
  }
})
