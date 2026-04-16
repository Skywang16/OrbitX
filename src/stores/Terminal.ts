import type { ShellInfo } from '@/api'
import { shellApi, storageApi, terminalApi, terminalContextApi, windowApi, workspaceApi } from '@/api'
import { useWorkspaceStore } from '@/stores/workspace'
import type { RuntimeTerminalState } from '@/types'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, nextTick, ref } from 'vue'

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
  const workspaceStore = useWorkspaceStore()

  const shellManager = ref<ShellManagerState>({
    availableShells: [],
    isLoading: false,
    error: null,
  })

  const listenersByPaneId = ref<Map<number, ListenerEntry[]>>(new Map())

  // Controls UI loading: only show when "newly created terminal" and "no output received yet"
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
  let initializationPromise: Promise<void> | null = null

  const pendingOperations = ref<Set<string>>(new Set())
  const operationQueue = ref<Array<() => Promise<void>>>([])
  let isProcessingQueue = false
  const MAX_CONCURRENT_OPERATIONS = 2

  type TerminalExitCallback = (paneId: number, exitCode: number | null) => void
  type TerminalCwdChangedCallback = (paneId: number, cwd: string) => void

  const terminalExitListeners = ref<TerminalExitCallback[]>([])
  const terminalCwdChangedListeners = ref<TerminalCwdChangedCallback[]>([])

  // Cache home directory to avoid repeated requests
  let homeDirectory: string | null = null
  const getHomeDirectory = async (): Promise<string> => {
    if (!homeDirectory) {
      homeDirectory = await windowApi.getHomeDirectory()
    }
    return homeDirectory
  }

  // Track initial directory for each terminal to avoid recording
  const terminalInitialCwd = ref<Map<number, string>>(new Map())

  // PTY resize debounce: split/drag causes layout jitter that triggers fit+resize rapidly, zsh redraws prompt.
  // Use trailing debounce: only send the last size after changes stop.
  const RESIZE_DEBOUNCE_MS = 180
  const resizeStateById = new Map<
    number,
    {
      timer: number | null
      pending: { rows: number; cols: number } | null
      lastSent: { rows: number; cols: number } | null
    }
  >()

  const activeTerminal = computed(() => terminals.value.find(t => t.id === activeTerminalId.value))
  const currentWorkingDirectory = computed(() => activeTerminal.value?.cwd || null)

  const queueOperation = async <T>(operation: () => Promise<T>): Promise<T> => {
    return new Promise((resolve, reject) => {
      const wrappedOperation = async () => {
        try {
          const result = await operation()
          resolve(result)
        } catch (err) {
          reject(err)
        }
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

    const removeTerminalState = (paneId: number, exitCode: number | null = null) => {
      const existsInStore = terminals.value.some(t => t.id === paneId)
      const hasPaneLifecycleState =
        paneOutputById.value.has(paneId) ||
        paneCreatedAtById.value.has(paneId) ||
        listenersByPaneId.value.has(paneId) ||
        terminalInitialCwd.value.has(paneId)

      if (!existsInStore && !hasPaneLifecycleState) {
        return
      }

      paneOutputById.value.delete(paneId)
      paneCreatedAtById.value.delete(paneId)

      const listeners = listenersByPaneId.value.get(paneId) || []
      listeners.forEach(listener => listener.callbacks.onExit(exitCode))
      terminalExitListeners.value.forEach(cb => cb(paneId, exitCode))

      terminalInitialCwd.value.delete(paneId)
      unregisterTerminalCallbacks(paneId)

      const index = terminals.value.findIndex(t => t.id === paneId)
      if (index !== -1) {
        terminals.value.splice(index, 1)
      }

      if (activeTerminalId.value === paneId) {
        activeTerminalId.value = terminals.value[0]?.id ?? null
      }
    }

    const unlistenCreated = await terminalApi.onTerminalCreated(async payload => {
      try {
        const state = await storageApi.getTerminalState(payload.paneId)
        if (!state) {
          return
        }
        upsertRuntimeTerminal(state)
      } catch (error) {
        console.error(`Failed to load runtime state for created terminal ${payload.paneId}:`, error)
      }
    })

    const unlistenClosed = await terminalApi.onTerminalClosed(payload => {
      removeTerminalState(payload.paneId, null)
    })

    const unlistenExit = await terminalApi.onTerminalExit(payload => {
      try {
        removeTerminalState(payload.paneId, payload.exitCode)
      } catch (error) {
        console.error('Error handling terminal exit event:', error)
      }
    })

    const unlistenCwdChanged = await terminalApi.onCwdChanged(payload => {
      try {
        const terminal = terminals.value.find(t => t.id === payload.paneId)
        if (terminal) {
          const previousCwd = terminal.cwd
          terminal.cwd = payload.cwd
          terminalCwdChangedListeners.value.forEach(cb => cb(payload.paneId, payload.cwd))

          // Record workspace to recent list
          // Exclude: 1) ~ directory  2) home directory  3) terminal's initial directory (first CWD change)
          const initialCwd = terminalInitialCwd.value.get(payload.paneId)
          const isFirstCwdChange = initialCwd === undefined

          if (isFirstCwdChange) {
            // Record initial directory, won't record next time
            terminalInitialCwd.value.set(payload.paneId, payload.cwd)
          } else if (payload.cwd && payload.cwd !== '~' && previousCwd !== payload.cwd) {
            // Only record when CWD actually changes
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

          // Refresh display title (cwd change affects title)
          refreshTerminalState(payload.paneId)
        }
      } catch (error) {
        console.error('Error handling terminal CWD change event:', error)
      }
    })

    const unlistenTitleChanged = await terminalApi.onTitleChanged(payload => {
      refreshTerminalState(payload.paneId)
    })

    const unlistenCommandEvent = await terminalApi.onCommandEvent(payload => {
      refreshTerminalState(payload.paneId)
    })

    globalListenersUnlisten = [
      unlistenCreated,
      unlistenClosed,
      unlistenExit,
      unlistenCwdChanged,
      unlistenTitleChanged,
      unlistenCommandEvent,
    ]
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

  // Dispatch output directly to registered callbacks via Channel subscription
  const dispatchOutputForPaneId = (paneId: number, data: string) => {
    const listeners = listenersByPaneId.value.get(paneId) || []
    listeners.forEach(listener => {
      try {
        listener.callbacks.onOutput(data)
      } catch (error) {
        console.error('Error dispatching terminal output:', error)
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

  const registerRuntimeTerminal = (terminal: RuntimeTerminalState) => {
    upsertRuntimeTerminal(terminal)
  }

  const createTerminalPane = async (
    initialDirectory?: string,
    options?: { shellName?: string; threadId?: number | null }
  ): Promise<number> => {
    const paneId =
      typeof options?.shellName === 'string'
        ? await terminalApi.createTerminalWithShell({
            shellName: options.shellName,
            rows: 24,
            cols: 80,
            threadId: options.threadId ?? undefined,
          })
        : await terminalApi.createTerminal({
            rows: 24,
            cols: 80,
            cwd: initialDirectory,
            threadId: options?.threadId ?? undefined,
          })

    const runtimeState = await storageApi.getTerminalState(paneId)
    if (!runtimeState) {
      throw new Error(`Failed to load runtime state for created terminal ${paneId}`)
    }

    upsertRuntimeTerminal(runtimeState)

    paneCreatedAtById.value.set(paneId, Date.now())
    paneOutputById.value.set(paneId, false)
    return paneId
  }

  const closeTerminal = async (id: number) => {
    return queueOperation(async () => {
      const terminal = terminals.value.find(t => t.id === id)
      if (!terminal) {
        console.warn(`Trying to close non-existent terminal: ${id}`)
        return
      }

      const resizeState = resizeStateById.get(id)
      if (resizeState?.timer) {
        clearTimeout(resizeState.timer)
      }
      resizeStateById.delete(id)

      // Clean up terminal's initial directory tracking
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
      console.warn(`Trying to activate non-existent terminal: ${id}`)
      return
    }

    activeTerminalId.value = id

    await terminalContextApi.setActivePaneId(id)
  }

  const writeToTerminal = async (id: number, data: string, execute: boolean = false) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal) {
      console.error(`Cannot write to terminal '${id}': not found.`)
      return
    }

    const finalData = execute ? `${data}\n` : data
    await terminalApi.writeToTerminal({ paneId: terminal.id, data: finalData })
  }

  const resizeTerminal = async (id: number, rows: number, cols: number) => {
    let state = resizeStateById.get(id)
    if (!state) {
      state = { timer: null, pending: null, lastSent: null }
      resizeStateById.set(id, state)
    }

    state.pending = { rows, cols }
    if (state.timer) {
      clearTimeout(state.timer)
    }

    state.timer = window.setTimeout(async () => {
      const current = resizeStateById.get(id)
      if (!current) return

      current.timer = null
      const pending = current.pending
      current.pending = null
      if (!pending) return

      if (current.lastSent && current.lastSent.rows === pending.rows && current.lastSent.cols === pending.cols) {
        return
      }

      const terminalSession = terminals.value.find(t => t.id === id)
      if (!terminalSession) {
        console.warn(`[HMR] Terminal '${id}' not in store, possibly due to hot reload`)
        return
      }

      current.lastSent = pending
      await terminalApi.resizeTerminal({
        paneId: terminalSession.id,
        rows: pending.rows,
        cols: pending.cols,
      })
    }, RESIZE_DEBOUNCE_MS)
  }
  const loadAvailableShells = async () => {
    shellManager.value.isLoading = true
    shellManager.value.error = null
    try {
      const shells = await shellApi.getAvailableShells()
      shellManager.value.availableShells = shells as ShellInfo[]
    } catch (error) {
      shellManager.value.error = error instanceof Error ? error.message : String(error)
      console.error('Failed to load available shells:', error)
    } finally {
      shellManager.value.isLoading = false
    }
  }

  const initializeShellManager = async () => {
    await loadAvailableShells()
  }

  const initializeTerminalStore = async () => {
    if (initializationPromise) {
      return initializationPromise
    }

    initializationPromise = (async () => {
      await initializeShellManager()
      await refreshRuntimeTerminals()
      await setupGlobalListeners()
    })()

    try {
      await initializationPromise
    } finally {
      initializationPromise = null
    }
  }

  const refreshRuntimeTerminals = async (): Promise<void> => {
    const runtimeStates = await storageApi.getTerminalsState()
    terminals.value = runtimeStates

    if (typeof activeTerminalId.value === 'number') {
      const activeStillExists = terminals.value.some(t => t.id === activeTerminalId.value)
      if (!activeStillExists) activeTerminalId.value = null
    }
  }

  /** Debounce timers per pane for title refresh */
  const titleRefreshTimers = new Map<number, ReturnType<typeof setTimeout>>()

  /** Refresh a single terminal's state from backend (for display title updates) */
  const refreshTerminalState = (paneId: number): void => {
    // Debounce: fast commands (ls, pwd, etc) execute too quickly causing title flicker
    // Multiple events within 150ms window are merged into one query
    const existing = titleRefreshTimers.get(paneId)
    if (existing) clearTimeout(existing)

    titleRefreshTimers.set(
      paneId,
      setTimeout(async () => {
        titleRefreshTimers.delete(paneId)
        try {
          const state = await storageApi.getTerminalState(paneId)
          if (!state) return

          const index = terminals.value.findIndex(t => t.id === paneId)
          if (index !== -1) {
            terminals.value[index] = state
          }

          if (state.kind === 'workspace' && typeof state.threadId === 'number' && state.displayTitle.trim()) {
            await workspaceStore.updateThreadTitle(state.threadId, state.displayTitle)
          }
        } catch (error) {
          console.warn(`Failed to refresh terminal state for pane ${paneId}:`, error)
        }
      }, 150)
    )
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
    registerRuntimeTerminal,
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
