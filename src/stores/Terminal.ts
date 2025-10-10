import { shellApi, storageApi, terminalApi, terminalContextApi } from '@/api'
import type { ShellInfo } from '@/api'
import { useSessionStore } from '@/stores/session'
import type { RuntimeTerminalState, TerminalState } from '@/types'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { defineStore } from 'pinia'
import { computed, ref, nextTick } from 'vue'

declare global {
  interface Window {
    os?: {
      homedir?: () => string
    }
  }
}
interface TerminalEventListeners {
  onOutput: (data: string) => void
  onExit: (exitCode: number | null) => void
}

interface ListenerEntry {
  id: string
  callbacks: TerminalEventListeners
}

type ResizeCallback = () => void

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

  const _listeners = ref<Map<number, ListenerEntry[]>>(new Map())

  const _resizeCallbacks = ref<Map<number, ResizeCallback>>(new Map())
  let _globalResizeListener: (() => void) | null = null

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
  const _commandEventListeners = ref<CommandEventCallback[]>([])

  const subscribeToCommandEvents = (callback: CommandEventCallback) => {
    _commandEventListeners.value.push(callback)
    return () => {
      const index = _commandEventListeners.value.indexOf(callback)
      if (index > -1) {
        _commandEventListeners.value.splice(index, 1)
      }
    }
  }

  type CommandEventPayloadMap = {
    started: CommandEventStartedPayload
    finished: CommandEventFinishedPayload
  }
  function emitCommandEvent<E extends CommandEventType>(
    terminalId: number,
    event: E,
    data: CommandEventPayloadMap[E]
  ): void {
    _commandEventListeners.value.forEach(callback => {
      try {
        callback(terminalId, event, data)
      } catch (error) {
        console.error('Command event callback error:', error)
      }
    })
  }

  let _globalListenersUnlisten: UnlistenFn[] = []
  let _isListenerSetup = false

  const _pendingOperations = ref<Set<string>>(new Set())
  const _operationQueue = ref<Array<() => Promise<void>>>([])
  let _isProcessingQueue = false
  const MAX_CONCURRENT_OPERATIONS = 2

  const sessionStore = useSessionStore()

  const activeTerminal = computed(() => terminals.value.find(t => t.id === activeTerminalId.value))
  const currentWorkingDirectory = computed(() => activeTerminal.value?.cwd || null)

  const queueOperation = async <T>(operation: () => Promise<T>): Promise<T> => {
    return new Promise(resolve => {
      const wrappedOperation = async () => {
        const result = await operation()
        resolve(result)
      }

      _operationQueue.value.push(wrappedOperation)
      processQueue()
    })
  }

  const processQueue = async () => {
    if (_isProcessingQueue || _operationQueue.value.length === 0) {
      return
    }

    if (_pendingOperations.value.size >= MAX_CONCURRENT_OPERATIONS) {
      return
    }

    _isProcessingQueue = true

    while (_operationQueue.value.length > 0 && _pendingOperations.value.size < MAX_CONCURRENT_OPERATIONS) {
      const operation = _operationQueue.value.shift()
      if (operation) {
        const operationId = `op-${Date.now()}-${Math.random()}`
        _pendingOperations.value.add(operationId)

        operation().finally(() => {
          _pendingOperations.value.delete(operationId)
          nextTick(() => processQueue())
        })
      }
    }

    _isProcessingQueue = false
  }

  const setupGlobalListeners = async () => {
    if (_isListenerSetup) return

    const unlistenExit = await terminalApi.onTerminalExit(payload => {
      try {
        const terminal = terminals.value.find(t => t.id === payload.paneId)
        if (terminal) {
          const listeners = _listeners.value.get(terminal.id) || []
          listeners.forEach(listener => listener.callbacks.onExit(payload.exitCode))

          closeTerminal(terminal.id)
        }
      } catch (error) {
        console.error('处理终端退出事件时发生错误:', error)
      }
    })

    const unlistenCwdChanged = await terminalApi.onCwdChanged(payload => {
      try {
        const terminal = terminals.value.find(t => t.id === payload.paneId)
        if (terminal) {
          terminal.cwd = payload.cwd
          updateTerminalTitle(terminal, payload.cwd)
        }
      } catch (error) {
        console.error('Error handling terminal CWD change event:', error)
      }
    })

    _globalListenersUnlisten = [unlistenExit, unlistenCwdChanged]
    _isListenerSetup = true
  }

  const teardownGlobalListeners = () => {
    _globalListenersUnlisten.forEach(unlisten => unlisten())
    _globalListenersUnlisten = []
    _isListenerSetup = false
  }

  const registerTerminalCallbacks = (id: number, callbacks: TerminalEventListeners) => {
    const listeners = _listeners.value.get(id) || []
    const entry: ListenerEntry = {
      id: `${id}-${Date.now()}`,
      callbacks,
    }
    listeners.push(entry)
    _listeners.value.set(id, listeners)
  }

  const unregisterTerminalCallbacks = (id: number, callbacks?: TerminalEventListeners) => {
    if (!callbacks) {
      _listeners.value.delete(id)
    } else {
      const listeners = _listeners.value.get(id) || []
      const filtered = listeners.filter(listener => listener.callbacks !== callbacks)
      if (filtered.length > 0) {
        _listeners.value.set(id, filtered)
      } else {
        _listeners.value.delete(id)
      }
    }
  }

  // 由 Channel 订阅直接分发输出给已注册回调
  const dispatchOutputForPaneId = (paneId: number, data: string) => {
    const listeners = _listeners.value.get(paneId) || []
    listeners.forEach(listener => {
      try {
        listener.callbacks.onOutput(data)
      } catch (error) {
        console.error('分发终端输出时发生错误:', error)
      }
    })
  }

  const registerResizeCallback = (terminalId: number, callback: ResizeCallback) => {
    _resizeCallbacks.value.set(terminalId, callback)

    if (_resizeCallbacks.value.size === 1 && !_globalResizeListener) {
      _globalResizeListener = () => {
        if (activeTerminalId.value) {
          const activeCallback = _resizeCallbacks.value.get(activeTerminalId.value)
          if (activeCallback) {
            activeCallback()
          }
        }
      }
      window.addEventListener('resize', _globalResizeListener)
    }
  }

  const unregisterResizeCallback = (terminalId: number) => {
    _resizeCallbacks.value.delete(terminalId)

    if (_resizeCallbacks.value.size === 0 && _globalResizeListener) {
      window.removeEventListener('resize', _globalResizeListener)
      _globalResizeListener = null
    }
  }

  const createTerminal = async (initialDirectory?: string): Promise<number> => {
    return queueOperation(async () => {
      const paneId = await terminalApi.createTerminal({
        rows: 24,
        cols: 80,
        cwd: initialDirectory,
      })

      const defaultShell = await shellApi.getDefaultShell()

      let resolvedCwd = initialDirectory || null
      try {
        resolvedCwd = await storageApi.getTerminalCwd(paneId)
      } catch (error) {
        console.warn('获取终端工作目录失败，使用回退目录:', error)
      }

      const terminal: RuntimeTerminalState = {
        id: paneId,
        title: defaultShell.name,
        cwd: resolvedCwd || initialDirectory || '~',
        active: false,
        shell: defaultShell.name,
      }

      updateTerminalTitle(terminal, terminal.cwd)

      const existingIndex = terminals.value.findIndex(t => t.id === paneId)
      if (existingIndex !== -1) {
        terminals.value.splice(existingIndex, 1)
      }
      terminals.value.push(terminal)
      await setActiveTerminal(paneId)

      // setActiveTerminal已经会调用syncToSessionStore和保存，不需要再次调用

      return paneId
    })
  }

  const closeTerminal = async (id: number) => {
    return queueOperation(async () => {
      const terminal = terminals.value.find(t => t.id === id)
      if (!terminal) {
        console.warn(`尝试关闭不存在的终端: ${id}`)
        return
      }

      unregisterTerminalCallbacks(id)

      await terminalApi.closeTerminal(id)

      await cleanupTerminalState(id)
      // cleanupTerminalState内部可能会调用setActiveTerminal，它已经会保存状态
      // 如果没有其他终端了，手动同步并保存
      if (terminals.value.length === 0) {
        syncToSessionStore()
        sessionStore.setActiveTabId(null)
      }
    })
  }

  const cleanupTerminalState = async (id: number) => {
    const index = terminals.value.findIndex(t => t.id === id)
    if (index !== -1) {
      terminals.value.splice(index, 1)
    }

    if (activeTerminalId.value === id) {
      if (terminals.value.length > 0) {
        await setActiveTerminal(terminals.value[0].id)
      } else {
        activeTerminalId.value = null
      }
    }
  }

  const setActiveTerminal = async (id: number) => {
    const targetTerminal = terminals.value.find(t => t.id === id)
    if (!targetTerminal) {
      console.warn(`尝试激活不存在的终端: ${id}`)
      return
    }

    activeTerminalId.value = id

    terminals.value.forEach(terminal => {
      terminal.active = terminal.id === id
    })

    await terminalContextApi.setActivePaneId(id)

    // 统一在最后同步并保存一次，避免多次调用
    // sessionStore.setActiveTabId内部会调用saveSessionState，所以这里只需要同步状态即可
    syncToSessionStore()
    sessionStore.setActiveTabId(id)
  }

  const writeToTerminal = async (id: number, data: string) => {
    const terminal = terminals.value.find(t => t.id === id)
    if (!terminal) {
      console.error(`无法写入终端 '${id}': 未找到。`)
      return
    }

    await terminalApi.writeToTerminal({ paneId: terminal.id, data })
  }

  const resizeTerminal = async (id: number, rows: number, cols: number) => {
    const terminalSession = terminals.value.find(t => t.id === id)
    if (!terminalSession) {
      console.error(`无法调整终端 '${id}' 大小: 未找到。`)
      return
    }

    await terminalApi.resizeTerminal({
      paneId: terminalSession.id,
      rows,
      cols,
    })
  }

  const updateTerminalTitle = (terminal: RuntimeTerminalState, cwd: string) => {
    try {
      if (terminal.shell === 'agent') {
        return
      }

      let displayPath = cwd

      if (typeof window !== 'undefined' && window.os?.homedir) {
        const homeDir = window.os.homedir?.()
        if (homeDir && cwd.startsWith(homeDir)) {
          displayPath = cwd.replace(homeDir, '~')
        }
      }

      const pathParts = displayPath.split(/[/\\]/).filter(part => part.length > 0)

      let newTitle: string

      if (displayPath === '~') {
        newTitle = displayPath
      } else {
        const parts = displayPath === '/' ? [] : pathParts
        if (parts.length === 0) {
          newTitle = '/'
        } else {
          newTitle = parts.slice(-2).join('/')
          if (parts.length > 3) {
            newTitle = `…/${newTitle}`
          }
        }
      }

      if (newTitle.length > 30) {
        newTitle = `…${newTitle.slice(-27)}`
      }

      if (terminal.title !== newTitle) {
        terminal.title = newTitle
      }
    } catch (error) {
      console.error('更新终端标题时发生错误:', error)
      const fallbackTitle = cwd.split(/[/\\]/).pop() || 'Terminal'
      if (terminal.title !== fallbackTitle) {
        terminal.title = fallbackTitle
      }
    }
  }

  const loadAvailableShells = async () => {
    shellManager.value.isLoading = true
    shellManager.value.error = null
    const shells = await shellApi.getAvailableShells()
    shellManager.value.availableShells = shells as ShellInfo[]
    shellManager.value.isLoading = false
  }

  const createAgentTerminal = async (agentName: string = 'AI Agent', initialDirectory?: string): Promise<number> => {
    return queueOperation(async () => {
      const agentTerminalTitle = agentName

      const existingAgentTerminal = terminals.value.find(
        terminal => terminal.shell === 'agent' && terminal.title === agentName
      )

      if (existingAgentTerminal) {
        await setActiveTerminal(existingAgentTerminal.id)
        existingAgentTerminal.title = agentTerminalTitle
        return existingAgentTerminal.id
      }

      const paneId = await terminalApi.createTerminal({
        rows: 24,
        cols: 80,
        cwd: initialDirectory,
      })

      let resolvedCwd = initialDirectory || null
      try {
        resolvedCwd = await storageApi.getTerminalCwd(paneId)
      } catch (error) {
        console.warn('获取Agent终端工作目录失败，使用回退目录:', error)
      }

      const terminal: RuntimeTerminalState = {
        id: paneId,
        title: agentTerminalTitle,
        cwd: resolvedCwd || initialDirectory || '~',
        active: false,
        shell: 'agent',
      }

      const existingIndex = terminals.value.findIndex(t => t.id === paneId)
      if (existingIndex !== -1) {
        terminals.value.splice(existingIndex, 1)
      }
      terminals.value.push(terminal)
      await new Promise(resolve => setTimeout(resolve, 100))
      await setActiveTerminal(paneId)
      // setActiveTerminal已经会同步并保存状态
      return paneId
    })
  }

  const createTerminalWithShell = async (shellName: string): Promise<number> => {
    return queueOperation(async () => {
      const title = shellName

      const shellInfo = shellManager.value.availableShells.find(s => s.name === shellName)
      if (!shellInfo) {
        throw new Error(`未找到shell: ${shellName}`)
      }

      const paneId = await terminalApi.createTerminalWithShell({
        shellName,
        rows: 24,
        cols: 80,
      })

      let resolvedCwd = shellInfo.path || null
      try {
        resolvedCwd = await storageApi.getTerminalCwd(paneId)
      } catch (error) {
        console.warn(`获取Shell(${shellName})终端工作目录失败，使用默认路径:`, error)
      }

      const terminal: RuntimeTerminalState = {
        id: paneId,
        title,
        cwd: resolvedCwd || shellInfo.path || '~',
        active: false,
        shell: shellInfo.name,
      }

      const existingIndex = terminals.value.findIndex(t => t.id === paneId)
      if (existingIndex !== -1) {
        terminals.value.splice(existingIndex, 1)
      }
      terminals.value.push(terminal)
      await setActiveTerminal(paneId)
      // setActiveTerminal已经会同步并保存状态

      return paneId
    })
  }

  const initializeShellManager = async () => {
    await loadAvailableShells()
  }

  const syncToSessionStore = () => {
    const terminalStates: TerminalState[] = terminals.value.map(terminal => ({
      id: terminal.id,
      title: terminal.title,
      active: terminal.id === activeTerminalId.value,
      shell: terminal.shell,
    }))

    sessionStore.updateTerminals(terminalStates)
    sessionStore.setActiveTabId(activeTerminalId.value)
  }

  const restoreFromSessionState = async () => {
    if (!sessionStore.initialized) {
      await sessionStore.initialize()
    }

    const savedTerminals = sessionStore.terminals || []
    let runtimeTerminals: RuntimeTerminalState[] = []

    try {
      runtimeTerminals = await storageApi.getTerminalsState()
    } catch (error) {
      console.error('加载终端运行时状态失败:', error)
    }

    const runtimeMap = new Map<number, RuntimeTerminalState>()
    runtimeTerminals.forEach(runtime => {
      runtimeMap.set(runtime.id, runtime)
    })

    const restored: RuntimeTerminalState[] = []

    for (const saved of savedTerminals) {
      const runtime = runtimeMap.get(saved.id)
      if (!runtime) {
        continue
      }

      restored.push({
        ...runtime,
        title: saved.title || runtime.title,
      })

      runtimeMap.delete(saved.id)
    }

    for (const runtime of runtimeMap.values()) {
      restored.push(runtime)
    }

    terminals.value = restored
    activeTerminalId.value = null

    const normalizePaneId = (value: unknown): number | null => {
      if (typeof value === 'number' && Number.isFinite(value)) {
        return value
      }
      if (typeof value === 'string' && value.trim().length > 0) {
        const parsed = Number.parseInt(value, 10)
        return Number.isNaN(parsed) ? null : parsed
      }
      return null
    }

    const savedActiveFromSession = normalizePaneId(sessionStore.sessionState.activeTabId)
    const savedActiveTerminalId = normalizePaneId(savedTerminals.find(t => t.active)?.id)
    const runtimeActiveTerminal = terminals.value.find(t => t.active)

    let targetActiveId: number | null = null

    if (savedActiveFromSession && terminals.value.some(t => t.id === savedActiveFromSession)) {
      targetActiveId = savedActiveFromSession
    } else if (savedActiveTerminalId && terminals.value.some(t => t.id === savedActiveTerminalId)) {
      targetActiveId = savedActiveTerminalId
    } else if (runtimeActiveTerminal) {
      targetActiveId = runtimeActiveTerminal.id
    } else if (terminals.value.length > 0) {
      targetActiveId = terminals.value[0].id
    }

    if (targetActiveId != null) {
      await setActiveTerminal(targetActiveId)
      return true
    }

    if (terminals.value.length === 0) {
      await createTerminal()
      return true
    }

    return terminals.value.length > 0
  }

  const saveSessionState = async () => {
    syncToSessionStore()
    await sessionStore.saveSessionState()
  }

  const initializeTerminalStore = async () => {
    await initializeShellManager()
    await restoreFromSessionState()
    await setupGlobalListeners()
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
    dispatchOutputForPaneId,
    registerResizeCallback,
    unregisterResizeCallback,
    createTerminal,
    createAgentTerminal,
    closeTerminal,
    setActiveTerminal,
    writeToTerminal,
    resizeTerminal,
    createTerminalWithShell,
    initializeShellManager,
    syncToSessionStore,
    restoreFromSessionState,
    saveSessionState,
    initializeTerminalStore,
    subscribeToCommandEvents,
    emitCommandEvent,
  }
})
