import { shellApi, storageApi, terminalApi, terminalContextApi, windowApi, workspaceApi } from '@/api'
import type { ShellInfo } from '@/api'
import type { RuntimeTerminalState } from '@/types'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, ref, nextTick } from 'vue'

interface TerminalEventListeners {
  onOutput: (data: string) => void
  onExit: (exitCode: number | null) => void
}

interface ListenerEntry {
  id: string
  callbacks: TerminalEventListeners
}

interface ShellManagerState {
  availableShells: ShellInfo[]
  isLoading: boolean
  error: string | null
}

export const useTerminalStore = defineStore('Terminal', () => {
  const terminals = ref<RuntimeTerminalState[]>([])
  const activeTerminalId = ref<number | null>(null)

  const shellManager = ref<ShellManagerState>({
    availableShells: [],
    isLoading: false,
    error: null,
  })

  const listenersByPaneId = ref<Map<number, ListenerEntry[]>>(new Map())

  // 用于控制 UI loading：只在“刚创建的新终端”且“尚未收到输出”时显示
  const paneOutputById = ref<Map<number, boolean>>(new Map())
  const paneCreatedAtById = ref<Map<number, number>>(new Map())

  type CommandEventType = 'started' | 'finished'
  interface CommandEventStartedPayload {
    commandId: string
  }
  interface CommandEventFinishedPayload {
    commandId: string
    exitCode: number
    isSuccess: boolean
  }
  type CommandEventPayload = CommandEventStartedPayload | CommandEventFinishedPayload
  type CommandEventCallback = (terminalId: number, event: CommandEventType, data?: CommandEventPayload) => void
  const commandEventListeners = ref<CommandEventCallback[]>([])

  const subscribeToCommandEvents = (callback: CommandEventCallback) => {
    commandEventListeners.value.push(callback)
    return () => {
      const index = commandEventListeners.value.indexOf(callback)
      if (index > -1) {
        commandEventListeners.value.splice(index, 1)
      }
    }
  }

  type CommandEventPayloadMap = {
    started: CommandEventStartedPayload
    finished: CommandEventFinishedPayload
  }
  const emitCommandEvent = <E extends CommandEventType>(
    terminalId: number,
    event: E,
    data: CommandEventPayloadMap[E]
  ): void => {
    commandEventListeners.value.forEach(callback => {
      try {
        callback(terminalId, event, data)
      } catch (error) {
        console.error('Command event callback error:', error)
      }
    })
  }

  let globalListenersUnlisten: UnlistenFn[] = []
  let isListenerSetup = false

  const pendingOperations = ref<Set<string>>(new Set())
  const operationQueue = ref<Array<() => Promise<void>>>([])
  let isProcessingQueue = false
  const MAX_CONCURRENT_OPERATIONS = 2

  type TerminalExitCallback = (paneId: number, exitCode: number | null) => void
  type TerminalCwdChangedCallback = (paneId: number, cwd: string) => void

  const terminalExitListeners = ref<TerminalExitCallback[]>([])
  const terminalCwdChangedListeners = ref<TerminalCwdChangedCallback[]>([])

  // 缓存 home 目录，避免重复请求
  let homeDirectory: string | null = null
  const getHomeDirectory = async (): Promise<string> => {
    if (!homeDirectory) {
      homeDirectory = await windowApi.getHomeDirectory()
    }
    return homeDirectory
  }

  // 跟踪每个终端的初始目录，避免记录
  const terminalInitialCwd = ref<Map<number, string>>(new Map())

  const activeTerminal = computed(() => terminals.value.find(t => t.id === activeTerminalId.value))
  const currentWorkingDirectory = computed(() => activeTerminal.value?.cwd || null)

  const queueOperation = async <T>(operation: () => Promise<T>): Promise<T> => {
    return new Promise(resolve => {
      const wrappedOperation = async () => {
        const result = await operation()
        resolve(result)
      }

      operationQueue.value.push(wrappedOperation)
      processQueue()
    })
  }

  const processQueue = async () => {
    if (isProcessingQueue || operationQueue.value.length === 0) {
      return
    }

    if (pendingOperations.value.size >= MAX_CONCURRENT_OPERATIONS) {
      return
    }

    isProcessingQueue = true

    while (operationQueue.value.length > 0 && pendingOperations.value.size < MAX_CONCURRENT_OPERATIONS) {
      const operation = operationQueue.value.shift()
      if (operation) {
        const operationId = `op-${Date.now()}-${Math.random()}`
        pendingOperations.value.add(operationId)

        operation().finally(() => {
          pendingOperations.value.delete(operationId)
          nextTick(() => processQueue())
        })
      }
    }

    isProcessingQueue = false
  }

  const setupGlobalListeners = async () => {
    if (isListenerSetup) return

    const unlistenExit = await terminalApi.onTerminalExit(payload => {
      try {
        const terminal = terminals.value.find(t => t.id === payload.paneId)
        const terminalId = terminal?.id ?? payload.paneId

        paneOutputById.value.delete(terminalId)
        paneCreatedAtById.value.delete(terminalId)

        const listeners = listenersByPaneId.value.get(terminalId) || []
        listeners.forEach(listener => listener.callbacks.onExit(payload.exitCode))
        terminalExitListeners.value.forEach(cb => cb(terminalId, payload.exitCode))

        terminalInitialCwd.value.delete(terminalId)
        unregisterTerminalCallbacks(terminalId)

        const index = terminals.value.findIndex(t => t.id === terminalId)
        if (index !== -1) terminals.value.splice(index, 1)
        if (activeTerminalId.value === terminalId) activeTerminalId.value = null
      } catch (error) {
        console.error('处理终端退出事件时发生错误:', error)
      }
    })

    const unlistenCwdChanged = await terminalApi.onCwdChanged(payload => {
      try {
        const terminal = terminals.value.find(t => t.id === payload.paneId)
        if (terminal) {
          const previousCwd = terminal.cwd
          terminal.cwd = payload.cwd
          terminalCwdChangedListeners.value.forEach(cb => cb(payload.paneId, payload.cwd))

          // 记录工作区到最近列表
          // 排除：1) ~ 目录  2) home 目录  3) 终端的初始目录（首次 CWD 变化）
          const initialCwd = terminalInitialCwd.value.get(payload.paneId)
          const isFirstCwdChange = initialCwd === undefined

          if (isFirstCwdChange) {
            // 记录初始目录，下次就不会记录了
            terminalInitialCwd.value.set(payload.paneId, payload.cwd)
          } else if (payload.cwd && payload.cwd !== '~' && previousCwd !== payload.cwd) {
            // 只有在 CWD 真正变化时才记录
            getHomeDirectory()
              .then(homeDir => {
                if (payload.cwd !== homeDir) {
                  return workspaceApi.addRecentWorkspace(payload.cwd)
                }
              })
              .catch(error => {
                console.warn('Failed to record recent workspace:', error)
              })
          }
        }
      } catch (error) {
        console.error('Error handling terminal CWD change event:', error)
      }
    })

    globalListenersUnlisten = [unlistenExit, unlistenCwdChanged]
    isListenerSetup = true
  }

  const teardownGlobalListeners = () => {
    globalListenersUnlisten.forEach(unlisten => unlisten())
    globalListenersUnlisten = []
    isListenerSetup = false
  }

  const registerTerminalCallbacks = (id: number, callbacks: TerminalEventListeners) => {
    const listeners = listenersByPaneId.value.get(id) || []
    const entry: ListenerEntry = {
      id: `${id}-${Date.now()}`,
      callbacks,
    }
    listeners.push(entry)
    listenersByPaneId.value.set(id, listeners)
  }

  const unregisterTerminalCallbacks = (id: number, callbacks?: TerminalEventListeners) => {
    if (!callbacks) {
      listenersByPaneId.value.delete(id)
    } else {
      const listeners = listenersByPaneId.value.get(id) || []
      const filtered = listeners.filter(listener => listener.callbacks !== callbacks)
      if (filtered.length > 0) {
        listenersByPaneId.value.set(id, filtered)
      } else {
        listenersByPaneId.value.delete(id)
      }
    }
  }

  const hasOutputSubscribers = (paneId: number): boolean => {
    const listeners = listenersByPaneId.value.get(paneId)
    return Array.isArray(listeners) && listeners.length > 0
  }

  // 由 Channel 订阅直接分发输出给已注册回调
  const dispatchOutputForPaneId = (paneId: number, data: string) => {
    const listeners = listenersByPaneId.value.get(paneId) || []
    listeners.forEach(listener => {
      try {
        listener.callbacks.onOutput(data)
      } catch (error) {
        console.error('分发终端输出时发生错误:', error)
      }
    })
  }

  const upsertRuntimeTerminal = (terminal: RuntimeTerminalState) => {
    const existingIndex = terminals.value.findIndex(t => t.id === terminal.id)
    if (existingIndex !== -1) {
      terminals.value.splice(existingIndex, 1)
    }
    terminals.value.push(terminal)
  }

  const createTerminalPane = async (initialDirectory?: string, options?: { shellName?: string }): Promise<number> => {
    const paneId =
      typeof options?.shellName === 'string'
        ? await terminalApi.createTerminalWithShell({
            shellName: options.shellName,
            rows: 24,
            cols: 80,
          })
        : await terminalApi.createTerminal({
            rows: 24,
            cols: 80,
            cwd: initialDirectory,
          })

    const terminal: RuntimeTerminalState = {
      id: paneId,
      cwd: initialDirectory || '~',
      shell: 'shell',
    }

    if (typeof options?.shellName === 'string') {
      const shellInfo = shellManager.value.availableShells.find(s => s.name === options.shellName)
      terminal.shell = shellInfo?.displayName ?? options.shellName
    } else {
      const defaultShell = await shellApi.getDefaultShell()
      terminal.shell = defaultShell.displayName
    }

    upsertRuntimeTerminal(terminal)
    paneCreatedAtById.value.set(paneId, Date.now())
    paneOutputById.value.set(paneId, false)
    return paneId
  }

  const closeTerminal = async (id: number) => {
    return queueOperation(async () => {
      const terminal = terminals.value.find(t => t.id === id)
      if (!terminal) {
        console.warn(`尝试关闭不存在的终端: ${id}`)
        return
      }

      // 清理终端的初始目录跟踪
      terminalInitialCwd.value.delete(id)
      paneOutputById.value.delete(id)
      paneCreatedAtById.value.delete(id)

      unregisterTerminalCallbacks(id)

      await terminalApi.closeTerminal(id)

      const index = terminals.value.findIndex(t => t.id === id)
      if (index !== -1) {
        terminals.value.splice(index, 1)
      }

      if (activeTerminalId.value === id && terminals.value.length === 0) {
        activeTerminalId.value = null
      }
    })
  }

  const setActiveTerminal = async (id: number) => {
    const targetTerminal = terminals.value.find(t => t.id === id)
    if (!targetTerminal) {
      console.warn(`尝试激活不存在的终端: ${id}`)
      return
    }

    activeTerminalId.value = id

    await terminalContextApi.setActivePaneId(id)
  }

  const writeToTerminal = async (id: number, data: string, execute: boolean = false) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal) {
      console.error(`无法写入终端 '${id}': 未找到。`)
      return
    }

    const finalData = execute ? `${data}\n` : data
    await terminalApi.writeToTerminal({ paneId: terminal.id, data: finalData })
  }

  const resizeTerminal = async (id: number, rows: number, cols: number) => {
    const terminalSession = terminals.value.find(t => t.id === id)
    if (!terminalSession) {
      console.warn(`[HMR] 终端 '${id}' 不在 store 中，可能是热更新导致`)
      return
    }

    await terminalApi.resizeTerminal({
      paneId: terminalSession.id,
      rows,
      cols,
    })
  }
  const loadAvailableShells = async () => {
    shellManager.value.isLoading = true
    shellManager.value.error = null
    const shells = await shellApi.getAvailableShells()
    shellManager.value.availableShells = shells as ShellInfo[]
    shellManager.value.isLoading = false
  }

  const createAgentTerminal = async (initialDirectory?: string): Promise<number> => {
    return queueOperation(async () => {
      const existingAgentTerminal = terminals.value.find(terminal => terminal.shell === 'agent')
      if (existingAgentTerminal) return existingAgentTerminal.id

      const paneId = await terminalApi.createTerminal({
        rows: 24,
        cols: 80,
        cwd: initialDirectory,
      })

      upsertRuntimeTerminal({
        id: paneId,
        cwd: initialDirectory || '~',
        shell: 'agent',
      })
      paneCreatedAtById.value.set(paneId, Date.now())
      paneOutputById.value.set(paneId, false)

      return paneId
    })
  }

  const initializeShellManager = async () => {
    await loadAvailableShells()
  }

  const initializeTerminalStore = async () => {
    await initializeShellManager()
    await refreshRuntimeTerminals()
    await setupGlobalListeners()
  }

  const refreshRuntimeTerminals = async (): Promise<void> => {
    const runtimeStates = await storageApi.getTerminalsState()
    terminals.value = runtimeStates

    if (typeof activeTerminalId.value === 'number') {
      const activeStillExists = terminals.value.some(t => t.id === activeTerminalId.value)
      if (!activeStillExists) activeTerminalId.value = null
    }
  }

  const subscribeToTerminalExit = (callback: TerminalExitCallback) => {
    terminalExitListeners.value.push(callback)
    return () => {
      const index = terminalExitListeners.value.indexOf(callback)
      if (index !== -1) terminalExitListeners.value.splice(index, 1)
    }
  }

  const subscribeToCwdChanged = (callback: TerminalCwdChangedCallback) => {
    terminalCwdChangedListeners.value.push(callback)
    return () => {
      const index = terminalCwdChangedListeners.value.indexOf(callback)
      if (index !== -1) terminalCwdChangedListeners.value.splice(index, 1)
    }
  }

  const markPaneHasOutput = (paneId: number) => {
    paneOutputById.value.set(paneId, true)
  }

  const hasPaneOutput = (paneId: number): boolean => {
    return paneOutputById.value.get(paneId) === true
  }

  const isPaneNew = (paneId: number): boolean => {
    const createdAt = paneCreatedAtById.value.get(paneId)
    if (typeof createdAt !== 'number') return false
    return Date.now() - createdAt < 2000
  }

  return {
    terminals,
    activeTerminalId,
    activeTerminal,
    currentWorkingDirectory,
    shellManager,
    setupGlobalListeners,
    teardownGlobalListeners,
    registerTerminalCallbacks,
    unregisterTerminalCallbacks,
    hasOutputSubscribers,
    dispatchOutputForPaneId,
    createAgentTerminal,
    closeTerminal,
    setActiveTerminal,
    writeToTerminal,
    resizeTerminal,
    createTerminalPane,
    initializeShellManager,
    refreshRuntimeTerminals,
    initializeTerminalStore,
    subscribeToCommandEvents,
    emitCommandEvent,
    subscribeToTerminalExit,
    subscribeToCwdChanged,
    markPaneHasOutput,
    hasPaneOutput,
    isPaneNew,
  }
})
